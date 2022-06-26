// Wordle implementation proper.

pub mod side;
pub use side::GameSide;
pub mod multiplayer;
pub use multiplayer::GameMP;
use poise::serenity_prelude::UserId;
use std::collections::HashMap;
use std::sync::atomic;
use std::time;

pub type GameId = u64;
pub type AtomicGameId = atomic::AtomicU64;

#[derive(Debug, Clone)]
pub struct Invite {
    pub expiry: time::SystemTime,
    pub game: GameId,
}

#[derive(Debug, Clone, Copy)]
pub enum GameVariant {
    Timed,
    TurnBased,
}
use GameVariant::*;

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
    pub fn invite(&mut self, variant: GameVariant, id: UserId, invite: Invite) {
        match variant {
            Timed => {self.timed_challenges.insert(id, invite);},
            TurnBased => {self.turn_challenges.insert(id, invite);},
        };
    }

    pub fn list(&self, variant: GameVariant) -> &HashMap<UserId, Invite> {
        match variant {
            Timed => &self.timed_challenges,
            TurnBased => &self.turn_challenges,
        }
    }
    
    /**
     * Remove an invitation of the given variant.
     * Returns it if found.
     */
    pub fn remove_invite(&mut self, variant: GameVariant, id: UserId) -> Option<Invite> {
        match variant {
            Timed => &mut self.timed_challenges,
            TurnBased => &mut self.turn_challenges,
        }.remove(&id)
    }

    // Returns None if no challenge from this user exists,
    // Some(false) if the challenge cannot be accepted,
    // Some(true) if it has been accepted.
    pub fn accept(&mut self, variant: GameVariant, id: UserId) -> Option<bool> {
        self.remove_invite(variant, id).map(|i| match variant {
            Timed => {
                if self.timed_game.is_some() {
                    return false;
                }
                self.timed_game = Some(i.game);
                true
            },
            TurnBased => {
                if self.turn_games.contains_key(&id) {
                    return false;
                }
                self.turn_games.insert(id, i.game);
                true
            },
        })
    }
    
    pub fn clean_invites(&mut self, before: time::SystemTime) {
        self.timed_challenges.retain(|_, v| v.expiry > before);
        self.turn_challenges.retain(|_, v| v.expiry > before);
    }
}