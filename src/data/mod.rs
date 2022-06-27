use crate::game;
use tokio::sync::RwLock as TokioRwLock;
use std::sync::atomic;
use std::collections::HashMap;
use poise::serenity_prelude as serenity;
use serenity::UserId;
use crate::dict;

pub struct UserData {
    pub player: game::PlayerData,
    pub score: u64,
}

impl UserData {
    pub fn new() -> UserData {
        UserData {
            player: game::PlayerData::new(),
            score: 0,
        }
    }
}

impl Default for UserData {
    fn default() -> Self {
        Self::new()
    }
}

pub mod scores;

pub struct CtxData {
    pub dict: dict::Dictionary, // immutable
    pub mpgames: TokioRwLock<HashMap<game::GameId, game::GameMP>>,
    pub userdata: TokioRwLock<HashMap<UserId, UserData>>,
    // Used internally. Generates sequential IDs.
    gameid_gen: game::AtomicGameId,
    scores: scores::ScoreManager,
}

impl CtxData {
    pub fn new(dict: dict::Dictionary) -> CtxData {
        CtxData {
            dict,
            mpgames: TokioRwLock::new(HashMap::new()),
            userdata: TokioRwLock::new(HashMap::new()),
            gameid_gen: game::AtomicGameId::new(0),
            scores: scores::ScoreManager::new(),
        }
    }
    
    pub fn pull_gameid(&self) -> game::GameId {
        self.gameid_gen.fetch_add(1, atomic::Ordering::Relaxed)
    }

    pub fn scores(&self) -> &scores::ScoreManager {
        &self.scores
    }
}