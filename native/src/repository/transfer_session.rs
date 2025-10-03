use crate::repository::id::RedbIdWrapper;
use core_services::db::redb::id::RedbId;
use core_services::db::redb::repository::RedbRepository;
use core_services::db::redb::table::RedbTable;
use core_services::db::repository::abstraction::errors::Resolve;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use redb::Database;
use shared::entities::local_resource::{LocalResource, LocalResourcePath};
use shared::entities::transfer_session::{TransferProgress, TransferSession};
use shared::repository::errors::PersistenceError;
use shared::repository::path_resolver::PathResolver;
use shared::repository::transfer_session::{TransferSessionId, TransferSessionRepository};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

pub struct TransferSessionRepositoryImpl {
    pub db: PoolRequest<Database>,
    pub path_resolver: Arc<dyn PathResolver>
}

impl RedbId for RedbIdWrapper<TransferSessionId> {
    fn lower_id(&self) -> Vec<Vec<u8>> {
        let code = bincode::serialize(&self.0.r#type).unwrap();
        let target = bincode::serialize(&self.0.target).unwrap();
        let id = bincode::serialize(&self.0.order_id).unwrap();
        vec![code, target, id]
    }
}

impl Table<RedbIdWrapper<TransferSessionId>> for TransferSession {
    fn get_table() -> &'static str {
        <Self as Table<TransferSessionId>>::get_table()
    }

    fn id(&self) -> RedbIdWrapper<TransferSessionId> {
        RedbIdWrapper(Table::id(self))
    }
}

impl RedbTable<RedbIdWrapper<TransferSessionId>> for TransferSession {}

#[async_trait::async_trait]
impl RedbRepository<TransferSession, RedbIdWrapper<TransferSessionId>> for TransferSessionRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl Repository<TransferSession, TransferSessionId> for TransferSessionRepositoryImpl {
    async fn create(&self, data: TransferSession) -> Resolve<TransferSession>
    where
        TransferSession: 'async_trait
    {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::create(self, data).await
    }

    async fn delete_one(&self, id: &TransferSessionId) -> Resolve<TransferSession> {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::delete_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn find_one(&self, id: &TransferSessionId) -> Resolve<Option<TransferSession>> {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: TransferSession) -> Resolve<TransferSession> {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::update_one(self, data).await
    }

    async fn find_all(
        &self,
        from_id: Option<&TransferSessionId>,
        to_id: Option<&TransferSessionId>,
        count: Option<usize>
    ) -> Resolve<Vec<TransferSession>> {
        let to_id = to_id.map(|it| RedbIdWrapper(it.clone()));
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_all(
            self,
            from_id.map(|it| RedbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }
}

#[async_trait::async_trait]
impl TransferSessionRepository for TransferSessionRepositoryImpl {
    async fn update_progresses(
        &self,
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> Result<Option<TransferSession>, PersistenceError> {
        let id = TransferSessionId {
            order_id: Some(order_id),
            ..Default::default()
        };
        let session = RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_one(self, &RedbIdWrapper(id)).await?;

        if let Some(session) = session {
            let mut session = session;
            session.progress = progresses;
            let result = RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::update_one(self, session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    async fn update_resource(
        &self,
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> Result<Option<TransferSession>, PersistenceError> {
        let session =
            RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_one(self, &RedbIdWrapper(session_id.clone()))
                .await?;

        if let Some(mut session) = session {
            session.replace_resource(resource);
            let result = RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::update_one(self, session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    async fn delete_session(&self, session_id: TransferSessionId) -> Result<(), PersistenceError> {
        let session =
            RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::delete_one(self, &RedbIdWrapper(session_id.clone()))
                .await?;
        let session_order_id = session.order_id;
        let session_dir_path = self.path_resolver.get_session_dir_path(session_order_id).await;
        let path_buf = PathBuf::from(session_dir_path);
        if !path_buf.is_dir() {
            return Err(PersistenceError::NotFound(format!(
                "Not found folder for session {session_order_id}"
            )))
        }

        if let Err(e) = fs::remove_dir_all(path_buf).await {
            log::error!("Error when delete session folder but we already delete the session record, so skipping...: {e}");
        }

        Ok(())
    }

    async fn generate_resource_paths(
        &self,
        session_order_id: u64,
        resource_names: HashMap<u64, String>
    ) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        let workdir = PathBuf::from(self.path_resolver.get_session_dir_path(session_order_id).await);
        let mut result = HashMap::new();
        let mut used_names = HashSet::new();

        for (resource_id, resource_name) in resource_names {
            let mut candidate_name = resource_name.clone();
            let mut counter = 1;

            while used_names.contains(&candidate_name) {
                candidate_name = generate_new_filename(&resource_name, counter);
                counter += 1;
            }

            used_names.insert(candidate_name.clone());

            let path = workdir.join(&candidate_name);
            let absolute_path = path.to_string_lossy().to_string();
            let resolved_path = self.path_resolver.get_local_resource_path(absolute_path).await;
            result.insert(resource_id, resolved_path);
        }

        Ok(result)
    }
}

fn generate_new_filename(original_name: &str, counter: u32) -> String {
    let path = Path::new(original_name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(original_name);
    let ext = path.extension().and_then(|e| e.to_str());

    if let Some(ext) = ext {
        format!("{stem}-{counter}.{ext}")
    } else {
        format!("{stem}-{counter}")
    }
}
