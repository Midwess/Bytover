use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::modules::AppModule;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::{AppModel, BitBridge};
use crate::entities::device::DeviceInfo;
use crate::entities::peer::Peer;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct P2PModel {
    pub device: Option<DeviceInfo>,
    pub me: Option<Peer>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct P2PViewModel {
    pub me: Option<PeerViewModel>,
}

pub struct P2PModule;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum P2PEvent {
    Launch { auto_launch: bool },
    UpdateMe { new_peer: Peer },
}

impl AppModule<BitBridge> for P2PModule {
    type Event = P2PEvent;
    type ViewModel = P2PViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            P2PEvent::Launch { auto_launch } => {
                model.environment.auto_launch_nearby = auto_launch;
                Command::new(|it| async move {
                    it.app().start_nearby_server(auto_launch).await;
                })
                .then_render()
            }
            P2PEvent::UpdateMe { new_peer } => {
                model.p2p.me = Some(new_peer);
                Command::render()
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            me: model.p2p.me.as_ref().map(PeerViewModel::from),
        }
    }
}
