use crate::game;
use tokio::sync::RwLock as TokioRwLock;
use std::sync::atomic;
use std::collections::HashMap;
use poise::serenity_prelude as serenity;
use serenity::UserId;
use crate::dict;

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

pub struct CtxData {
    pub dict: dict::Dictionary, // immutable
    pub mpgames: TokioRwLock<HashMap<game::GameId, game::GameMP>>,
    pub userdata: TokioRwLock<HashMap<UserId, UserData>>,
    // Used internally. Generates sequential IDs.
    gameid_gen: game::AtomicGameId,
}

impl CtxData {
    pub fn new(dict: dict::Dictionary) -> CtxData {
        CtxData {
            dict,
            mpgames: TokioRwLock::new(HashMap::new()),
            userdata: TokioRwLock::new(HashMap::new()),
            gameid_gen: game::AtomicGameId::new(0),
        }
    }
    
    pub fn pull_gameid(&self) -> game::GameId {
        self.gameid_gen.fetch_add(1, atomic::Ordering::Relaxed)
    }
}