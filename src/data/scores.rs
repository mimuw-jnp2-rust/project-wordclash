use poise::serenity_prelude as serenity;
use serenity::UserId;
use crate::game;
use tokio::sync::RwLock as TokioRwLock;
use std::collections::HashMap;

pub struct ScoreManager {
    scores: TokioRwLock<HashMap<UserId, u64>>,
}

impl ScoreManager {
    pub fn new() -> ScoreManager {
        ScoreManager {
            scores: TokioRwLock::new(HashMap::new()),
        }
    }
    
    pub async fn list_top(&self, count: usize) -> Vec<(UserId, u64)> {
        let guard = self.scores.read().await;
        // Complexity's pretty weak, but it works
        let mut res = guard.iter().map(|(k, v)| (*k, *v))
            .collect::<Vec<_>>();
        res.sort();
        res[0..count].into()
    }

    pub async fn add(&self, player: UserId, score: u64) {
        let mut guard = self.scores.write().await;
        *guard.entry(player).or_default() += score;
    }
    
    pub async fn add_from_game(&self, game: &game::GameMP) {
        let score = game.get_score();
        for i in 0..1 {
            self.add(game.get_user_id(i), score[i]).await;
        }
    }

    pub async fn get(&self, player: UserId) -> Option<u64> {
        let guard = self.scores.read().await;
        guard.get(&player).copied()
    }
}