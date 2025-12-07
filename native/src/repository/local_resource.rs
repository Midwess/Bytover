use crate::core_api_impl::io::{CIOCursorBoxWrapper, DIOWriterWrapper};
use crate::repository::id::RedbIdWrapper;
use core_services::db::redb::id::RedbId;
use core_services::db::redb::repository::RedbRepository;
use core_services::db::redb::table::RedbTable;
use core_services::db::repository::abstraction::errors::Resolve;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::local_storage::entry::FileEntry;
use core_services::local_storage::file_system::Folder;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use devlog_sdk::distributed_id::gen_id;
use futures_util::future::join_all;
use redb::Database;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::repository::errors::PersistenceError;
use shared::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use shared::repository::path_resolver::PathResolver;
use shared::shell::api::{CIOCursor, DIOWriter, IOReader, IOWriter};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct LocalResourceRepositoryImpl {
    pub db: PoolRequest<Database>,
    pub path_resolver: Arc<dyn PathResolver>
}

impl RedbId for RedbIdWrapper<LocalResourceId> {
    fn lower_id(&self) -> Vec<Vec<u8>> {
        let code = bincode::serialize(&self.0.path).unwrap();
        let id = bincode::serialize(&self.0.order_id).unwrap();
        vec![code, id]
    }
}

impl Table<RedbIdWrapper<LocalResourceId>> for LocalResource {
    fn get_table() -> &'static str {
        <Self as Table<LocalResourceId>>::get_table()
    }

    fn id(&self) -> RedbIdWrapper<LocalResourceId> {
        RedbIdWrapper(Table::id(self))
    }
}

impl RedbTable<RedbIdWrapper<LocalResourceId>> for LocalResource {}

#[async_trait::async_trait]
impl RedbRepository<LocalResource, RedbIdWrapper<LocalResourceId>> for LocalResourceRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl Repository<LocalResource, LocalResourceId> for LocalResourceRepositoryImpl {
    async fn create(&self, data: LocalResource) -> Resolve<LocalResource>
    where
        LocalResource: 'async_trait
    {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::create(self, data).await
    }

    async fn delete_one(&self, id: &LocalResourceId) -> Resolve<LocalResource> {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::delete_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn find_one(&self, id: &LocalResourceId) -> Resolve<Option<LocalResource>> {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::find_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: LocalResource) -> Resolve<LocalResource> {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::update_one(self, data).await
    }

    async fn find_all(
        &self,
        from_id: Option<&LocalResourceId>,
        to_id: Option<&LocalResourceId>,
        count: Option<usize>
    ) -> Resolve<Vec<LocalResource>> {
        let to_id = to_id.map(|it| RedbIdWrapper(it.clone()));
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::find_all(
            self,
            from_id.map(|it| RedbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }
}

#[async_trait::async_trait]
impl LocalResourceRepository for LocalResourceRepositoryImpl {
    async fn load(&self, path: LocalResourcePath) -> Result<Option<LocalResource>, PersistenceError> {
        let absolute_path = self.path_resolver.get_absolute_path(path.clone()).await;
        let path_buf = PathBuf::from(absolute_path.clone());
        if path_buf.is_dir() {
            let folder = Folder::new(path_buf).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;

            let size = self.size(path.clone()).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;
            let resource = LocalResource {
                order_id: gen_id().await,
                name: folder.name.clone(),
                size,
                path,
                thumbnail_path: None,
                r#type: ResourceType::Folder
            };

            return Ok(Some(resource))
        } else if path_buf.is_symlink() {
            log::warn!("Symlink is not supported: {absolute_path:?}");
            return Ok(None)
        }

        let file = FileEntry::new(None, absolute_path)
            .await
            .map_err(|it| PersistenceError::IOError(format!("{it:?}")))?;
        let resource = LocalResource {
            order_id: gen_id().await,
            name: file.name(),
            size: file.size,
            path,
            thumbnail_path: None,
            r#type: ResourceType::File
        };

        Ok(Some(resource))
    }

    async fn load_all(&self) -> Result<Vec<LocalResource>, PersistenceError> {
        let resources = RedbRepository::find_all(self, None, None, None).await?;
        let mut futures = vec![];
        for resource in resources.iter() {
            futures.push(async {
                let load_result = self.load(resource.path.clone()).await;
                match load_result {
                    Ok(Some(resource)) => Some(resource),
                    _ => {
                        let id = LocalResourceId {
                            order_id: Some(resource.order_id),
                            ..Default::default()
                        };

                        let _ = RedbRepository::delete_one(self, &RedbIdWrapper(id)).await;
                        None
                    }
                }
            });
        }

        let mut resources = join_all(futures).await.into_iter().flatten().collect::<Vec<_>>();

        resources.sort_by(|a, b| a.order_id.cmp(&b.order_id));

        Ok(resources)
    }

    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError> {
        let Some(mut resource) = RedbRepository::find_one(
            self,
            &RedbIdWrapper(LocalResourceId {
                order_id: Some(resource_id),
                ..Default::default()
            })
        )
        .await?
        else {
            return Err(PersistenceError::NotFound(format!("Resource {resource_id}")))
        };

        let path = self.path_resolver.get_thumbnail_file_path(resource_id).await;
        log::info!("Creating thumbnail at {path:?}");
        FileEntry::new(Some(png_bytes), &path)
            .await
            .map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;
        let saved_path = self.path_resolver.get_local_resource_path(path).await;
        resource.thumbnail_path = Some(saved_path.clone());
        RedbRepository::update_one(self, resource).await?;

        Ok(saved_path)
    }

    async fn get_resource_type(&self, path: LocalResourcePath) -> Result<ResourceType, PersistenceError> {
        let absolute_path = self.path_resolver.get_absolute_path(path).await;
        let file = FileEntry::existing(&absolute_path).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;

        if file.is_dir {
            Ok(ResourceType::Folder)
        } else {
            let mime_type = mime_guess::from_path(&file.path).first_or_octet_stream();
            let resource_type = if mime_type.type_() == mime_guess::mime::IMAGE {
                ResourceType::Image
            } else if mime_type.type_() == mime_guess::mime::VIDEO {
                ResourceType::Video
            } else {
                ResourceType::File
            };

            Ok(resource_type)
        }
    }

    async fn read(&self, path: LocalResourcePath, buffer_size: usize, compressed: bool) -> Result<Box<dyn CIOCursor>, PersistenceError> {
        let absolute_path = self.path_resolver.get_absolute_path(path).await;
        let path = PathBuf::from(absolute_path);
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        if path.is_dir() {
            let folder = Folder::new(path).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;
            let cursor = folder.cursor(buffer_size).await.map_err(|it| PersistenceError::IOError(format!("{it:?}")))?;
            let wrapped = CIOCursorBoxWrapper::new(cursor, &file_name);
            return Ok(Box::new(wrapped));
        };

        let file = FileEntry::existing(path).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;
        let cursor = file.cursor(buffer_size).await.map_err(|it| PersistenceError::IOError(format!("{it:?}")))?;
        let wrapped = CIOCursorBoxWrapper::new(cursor, &file_name);
        Ok(Box::new(wrapped))
    }

    async fn write(&self, path: LocalResourcePath, compressed: bool) -> Result<Box<dyn DIOWriter>, PersistenceError> {
        let absolute_path = self.path_resolver.get_absolute_path(path).await;
        let path = PathBuf::from(absolute_path);
        let writer = DIOWriterWrapper::from_path(path, compressed).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;
        Ok(Box::new(writer))
    }

    async fn generate_thumbnail_paths(
        &self,
        _: Option<u64>,
        resource_ids: Vec<u64>
    ) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        let mut result = HashMap::new();
        for resource_id in resource_ids.iter() {
            let thumbnail_absolute = self.path_resolver.get_thumbnail_file_path(*resource_id).await;
            let path = self.path_resolver.get_local_resource_path(thumbnail_absolute).await;
            result.insert(*resource_id, path);
        }

        log::info!("Generated thumbnail paths: {:?}", result);
        Ok(result)
    }

    async fn size(&self, path: LocalResourcePath) -> Result<u64, PersistenceError> {
        let absolute_path = self.path_resolver.get_absolute_path(path).await;
        let path = PathBuf::from(absolute_path);

        let cursor = match path.is_dir() {
            true => {
                let folder = Folder::new(path).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;
                folder.cursor(1024).await.map_err(|it| PersistenceError::IOError(format!("{it:?}")))?
            }
            false => {
                let file = FileEntry::existing(path).await.map_err(|e| PersistenceError::IOError(format!("{e:?}")))?;
                file.cursor(1024).await.map_err(|it| PersistenceError::IOError(format!("{it:?}")))?
            }
        };

        Ok(cursor.entry().await?.size)
    }

    async fn remove(&self, path: LocalResourcePath) -> Result<Vec<LocalResource>, PersistenceError> {
        let from_id = LocalResourceId {
            path: Some(path),
            order_id: None
        };

        let items = RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::find_all(
            self,
            Some(&RedbIdWrapper(from_id)),
            None,
            None
        ).await?;

        let mut removed_items = vec![];
        for item in items.iter() {
            let id: LocalResourceId = Table::<LocalResourceId>::id(item);
            let removed = Repository::<LocalResource, LocalResourceId>::delete_one(self, &id).await;
            if let Ok(item) = removed {
                removed_items.push(item);
            }
        }

        Ok(removed_items)
    }
}
