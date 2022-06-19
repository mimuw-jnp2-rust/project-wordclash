use crate::game;

pub struct UserData {
    pub player: game::PlayerData,
}

impl UserData {
    pub fn new() -> UserData {
        UserData {
            player: game::PlayerData::new(),
        }
    }
}

impl Default for UserData {
    fn default() -> Self {
        Self::new()
    }
}
