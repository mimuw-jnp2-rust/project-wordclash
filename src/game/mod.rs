// Wordle implementation proper.

pub mod side;
pub use side::GameSide;
pub mod multiplayer;
pub use multiplayer::GameMP;
use poise::serenity_prelude::UserId;

// Per-player data
pub struct PlayerData {
    pub game: ActiveGame,
}

impl PlayerData {
    pub fn new() -> PlayerData {
        PlayerData {
            game: ActiveGame::None,
        }
    }
}

pub enum ActiveGame {
    None,
    Multiplayer(UserId),
}
