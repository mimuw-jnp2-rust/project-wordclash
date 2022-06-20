use crate::game;
use tokio::sync::RwLock as TokioRwLock;
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
    pub mpgames: TokioRwLock<HashMap<UserId, game::GameMP>>,
    pub userdata: TokioRwLock<HashMap<UserId, UserData>>,
}

impl CtxData {
}