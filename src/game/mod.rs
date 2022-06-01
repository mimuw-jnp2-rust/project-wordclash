// Wordle implementation proper.

pub mod side;
pub use side::GameSide;
pub mod multiplayer;
pub use multiplayer::GameMP;
use poise::serenity_prelude::UserId;

// Per-player data
pub struct PlayerData {
    game: ActiveGame,
}

pub enum ActiveGame {
    None,
    Multiplayer(UserId),
}