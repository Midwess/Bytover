use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::core::model_events::LocalResourceEvent;
use crate::app::modules::AppModule;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppModel, BitBridge};
use crate::entities::local_resource::{LocalResourcePath, ResourceType};
use crate::entities::shelf::Shelf;
use crate::repository::local_resource::LocalResourceId;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ShelfModel {
    pub shelf: Shelf,
    pub is_loading: bool
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShelfViewModel {
    pub selected_resources: Vec<SelectedResourceViewModel>,
    pub is_loading: bool
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShelfEvent {
    Launch,
    BeginLoadingResources,
    EndLoadingResources,
    OpenResource(u64),
    AddResources(Vec<ResourceSelection>),
    RemoveResource(u64),

    #[serde(skip)]
    ModelEvent(LocalResourceEvent)
}

pub struct ShelfModule;

impl AppModule<BitBridge> for ShelfModule {
    type ViewModel = ShelfViewModel;
    type Event = ShelfEvent;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            Self::Event::Launch => Command::new_result(|it| async move {
                it.app().load_resources().await
            }),
            Self::Event::BeginLoadingResources => {
                model.shelf.is_loading = true;
                Command::render()
            }
            Self::Event::EndLoadingResources => {
                model.shelf.is_loading = false;
                Command::render()
            }
            Self::Event::AddResources(selections) => {
                let mut commands = vec![];
                let mut filtered_selections = vec![];
                for selection in selections {
                    if model.shelf.shelf.is_exists(&selection.path) {
                        commands.push(Command::operate(DialogOperation::Toast(
                            "Resource was already added before2.".to_owned()
                        )))
                    } else {
                        filtered_selections.push(selection);
                    }
                }

                commands.push(Command::new_result(|it| async move {
                    let app = it.app();
                    app.notify_event(ShelfEvent::BeginLoadingResources);
                    app.new_resources(filtered_selections).await?;
                    app.notify_event(ShelfEvent::EndLoadingResources);

                    Ok(())
                }));

                Command::all(commands)
            }
            Self::Event::RemoveResource(id) => Command::new_result(move |it| async move {
                it.app().remove_resource(id).await
            }),
            Self::Event::ModelEvent(event) => {
                match event {
                    LocalResourceEvent::Add(resource) => {
                        model.shelf.shelf.add_resource(resource);
                    }
                    LocalResourceEvent::Remove(id) => {
                        model.shelf.shelf.remove_resource(&id);
                    }
                    _ => {}
                }

                Command::done()
            }
            Self::Event::OpenResource(id) => {
                let id = LocalResourceId {
                    order_id: Some(id),
                    ..Default::default()
                };

                let Some(resource) = model.shelf.shelf.get(&id) else {
                    return Command::done();
                };

                let resource_path = resource.path.clone();
                Command::new(move |it| async move {
                    let _ = DeviceOperation::open(resource_path).into_future(it.clone()).await;
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        ShelfViewModel {
            selected_resources: model.shelf.shelf.resources.iter().map(SelectedResourceViewModel::from).collect(),
            is_loading: model.shelf.is_loading
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ResourceSelection {
    pub path: LocalResourcePath,
    // This is optional, if it is None, we will detect by Rust code to see if it should be a Folder or a File
    pub r#type: Option<ResourceType>
}
