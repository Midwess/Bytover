use chrono::Utc;
use matchbox_protocol::PeerId;
use schema::devlog::bitbridge::PeerMessage;
use serde::{Deserialize, Serialize};

use crate::entities::device::DeviceInfo;

// Peer is represent for the information that you want other
// people to know about
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub email: Option<String>,
    pub device: DeviceInfo
}

impl Peer {
    pub fn id(&self) -> uuid::Uuid {
        // The id is always be String with uuid format, so we can unwrap safely
        self.id.clone().parse().unwrap_or_default()
    }

    pub fn peer_id(&self) -> PeerId {
        self.id().into()
    }

    pub fn random_avatar() -> String {
        let animals = [
            "Penguin", "Rabbit", "Sheep", "Squirrel", "Tiger", "Bear", "Cat", "Chicken", "Cow", "Deer", "Dog", "Elephant", "Fox",
            "Giraffe", "Koala", "Lion", "Owl", "Panda"
        ];

        let rng = (Utc::now().timestamp_millis() % (animals.len() as i64)) as usize;
        let chosen_animal = animals[rng];

        format!("https://pub-13678040a05e4d5eaa3d4afbb253827c.r2.dev/public/animal_avatars/{chosen_animal}.png")
    }
}

impl From<PeerMessage> for Peer {
    fn from(value: PeerMessage) -> Self {
        Self {
            id: value.peer_id,
            name: value.name,
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into()
        }
    }
}

impl From<Peer> for PeerMessage {
    fn from(value: Peer) -> Self {
        Self {
            peer_id: value.id,
            name: value.name,
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into()
        }
    }
}
