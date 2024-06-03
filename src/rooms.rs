use std::{
    collections::HashMap,
    sync::Arc,
};

use tokio::sync::Mutex;
use yrs::Doc;

pub struct RoomState {
    pub state: Doc,
    active: bool,
}

#[derive(Clone)]
pub struct Room {
    pub value: Arc<Mutex<RoomState>>,
}

impl Room {
    pub fn new() -> Self {
        return {
            let doc = Doc::new();
            doc.get_or_insert_map("shapes");
            Self {
                value: Arc::new(Mutex::new(RoomState { state: doc, active: false })) 
            }
        }
    }

    pub async fn activate(&self) {
        let mut room_state = self.value.lock().await;
        room_state.active = true;
    }

    pub async fn deactivate(&self) {
        let mut room_state = self.value.lock().await;
        room_state.active = false;
    }

    pub async fn is_active(&self) -> bool {
        let room_state = self.value.lock().await;
        room_state.active
    }
}

struct State {
    rooms: HashMap<String, Room>,
}

#[derive(Clone)]
pub struct Rooms {
    value: Arc<Mutex<State>>,
}

impl Rooms {
    pub fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(State {
                rooms: HashMap::new(),
            })),
        }
    }

    pub async fn get_room(&self, key: &String) -> Option<Room> {
        let state = self.value.lock().await;
        let room = state.rooms.get(key)?.clone();
        Some(room)
    }

    pub async fn insert(&self, room_id: String, room: Room) {
        let mut state = self.value.lock().await;
        state.rooms.insert(room_id, room);
    }
}
