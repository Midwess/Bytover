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
    pub launched: bool
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct P2PViewModel {
    pub me: Option<PeerViewModel>
}

pub struct P2PModule;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum P2PEvent {
    Launch,
    SetLaunched(bool),
    UpdateMe { new_peer: Peer },
    PeerDisconnected { peer_id: String }
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
            P2PEvent::Launch => {
                if model.p2p.launched {
                    return Command::done();
                }
                model.p2p.launched = true;
                let me = model.p2p.me.clone();
                Command::handle_result(|it| async move { it.app().start_nearby_server(me).await }).then_render()
            }
            P2PEvent::SetLaunched(launched) => {
                model.p2p.launched = launched;
                Command::done()
            }
            P2PEvent::UpdateMe { new_peer } => {
                model.p2p.me = Some(new_peer);
                Command::render()
            }
            P2PEvent::PeerDisconnected { .. } => Command::render()
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            me: model.p2p.me.as_ref().map(PeerViewModel::from)
        }
    }
}
