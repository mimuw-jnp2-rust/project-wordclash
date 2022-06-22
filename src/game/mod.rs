// Wordle implementation proper.

pub mod side;
pub use side::GameSide;
pub mod multiplayer;
pub use multiplayer::GameMP;
use poise::serenity_prelude::UserId;
use std::collections::{HashSet, HashMap};
use std::sync::atomic;
use std::time;

pub type GameId = u64;
pub type AtomicGameId = atomic::AtomicU64;

#[derive(Debug, Clone)]
pub struct Invite {
    pub expiry: time::SystemTime,
    pub game: GameId,
}

// Per-player data
pub struct PlayerData {
    pub timed_game: Option<GameId>,
    timed_challenges: HashMap<UserId, Invite>,
    pub turn_games: HashMap<UserId, GameId>,
    turn_challenges: HashMap<UserId, Invite>,
}

impl PlayerData {
    pub fn new() -> PlayerData {
        PlayerData {
            timed_game: None,
            timed_challenges: HashMap::new(),
            turn_games: HashMap::new(),
            turn_challenges: HashMap::new(),
        }
    }

    // Inserts the invite if no invite is waiting from this user.
    // Else swaps out the existing invite..
    pub fn invite_timed(&mut self, id: UserId, invite: Invite) {
        todo!()
    }

    pub fn list_timed(&self) -> &HashMap<UserId, Invite> {
        &self.timed_challenges
    }

    pub fn accept_timed(&mut self, id: UserId) -> Option<bool> {
        self.timed_challenges.get(&id).map(|c| {
            if self.timed_game.is_some() {
                return false;
            }
            self.timed_game = Some(c.game);
            true
        })
    }
    
    pub fn invite_turn(&mut self, id: UserId, invite: Invite) -> bool {
        !self.turn_games.contains_key(&id) && {
            self.turn_challenges.insert(id, invite);
            true
        }
    }

    pub fn list_turn(&self, id: UserId) -> &HashMap<UserId, Invite> {
        &self.turn_challenges
    }

    pub fn accept_turn(&mut self, id: UserId) -> Option<bool> {
        self.turn_challenges.get(&id).map(|c| {
            self.turn_games.insert(id, c.game);
            true
        })
    }
}