// Wordle implementation proper.

pub mod side;
pub use side::GameSide;
pub mod multiplayer;
pub use multiplayer::GameMP;
use poise::serenity_prelude::UserId;
use std::collections::{HashSet, HashMap};
use std::time;

#[derive(Debug, Clone)]
pub struct Invite {
    pub expiry: time::SystemTime,
}

// Per-player data
pub struct PlayerData {
    pub timed_game: ActiveGame,
    timed_challenges: HashSet<UserId>,
    pub turn_games: HashSet<UserId>,
    turn_challenges: HashSet<UserId>,
}

impl PlayerData {
    pub fn new() -> PlayerData {
        PlayerData {
            timed_game: ActiveGame::None,
            timed_challenges: HashSet::new(),
            turn_games: HashSet::new(),
            turn_challenges: HashSet::new(),
        }
    }

    // Inserts the invite if no invite is waiting from this user.
    // Else swaps out the existing invite..
    pub fn invite_timed(&mut self, id: UserId, invite: Invite) {
        todo!()
    }

    pub fn list_timed(&self) -> &HashSet<UserId> {
        &self.timed_challenges
    }

    pub fn accept_timed(&mut self, id: UserId) -> Option<bool> {
        self.timed_challenges.get(&id).map(|c| {
            if !matches!(self.timed_game, ActiveGame::None) {
                return false;
            }
            self.timed_game = ActiveGame::Multiplayer(id);
            true
        })
    }
    
    pub fn invite_turn(&mut self, id: UserId) -> bool {
        !self.turn_games.contains(&id) && {
            self.turn_challenges.insert(id);
            true
        }
    }

    pub fn list_turn(&self, id: UserId) -> &HashSet<UserId> {
        &self.turn_challenges
    }

    pub fn accept_turn(&mut self, id: UserId) -> Option<bool> {
        self.turn_challenges.get(&id).map(|c| {
            self.turn_games.insert(id);
            true
        })
    }
}

pub enum ActiveGame {
    None,
    Multiplayer(UserId),
}
