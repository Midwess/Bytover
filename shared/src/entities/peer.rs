use schema::devlog::bitbridge::PeerMessage;
use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::Record;
use rand::seq::{IndexedRandom, SliceRandom};

use crate::entities::device::DeviceInfo;

// Peer is represent for the information that you want other
// people to know about
#[derive(Debug, Clone, Record, Serialize, Deserialize, PartialEq, Eq, SurrealDerive)]
pub struct Peer {
    pub id: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub email: Option<String>,
    pub device: DeviceInfo
}

impl Peer {
    pub fn id(&self) -> u128 {
        self.id.parse::<u128>().expect("Failed to parse peer id")
    }

    pub fn random_avatar() -> String {
        let animals = [
            "Penguin", "Rabbit", "Sheep", "Squirrel", "Tiger",
            "Bear", "Cat", "Chicken", "Cow", "Deer", "Dog",
            "Elephant", "Fox", "Giraffe", "Koala", "Lion",
            "Owl", "Panda",
        ];

        let mut rng = rand::thread_rng();
        let chosen_animal = animals.choose(&mut rng).unwrap_or(&"Panda"); // Default to Panda if slice is empty (shouldn't happen here)

        format!(
            "https://cdn.devlog.studio/public/animal_avatars/{}.png",
            chosen_animal
        )
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
