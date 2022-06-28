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
    
    pub async fn with_read<F: FnOnce(&HashMap<UserId, u64>)> (&self, f: F) {
        let guard = self.scores.read().await;
        f(&guard)
    }

    pub async fn with_write<F: FnOnce(&mut HashMap<UserId, u64>)> (&self, f: F) {
        let mut guard = self.scores.write().await;
        f(&mut guard)
    }
    
    pub async fn list_top(&self, count: usize) -> Vec<(UserId, u64)> {
        let guard = self.scores.read().await;
        // Complexity's pretty weak, but it works
        let mut res = guard.iter().map(|(k, v)| (*k, *v))
            .collect::<Vec<_>>();
        res.sort_by(|a, b| b.cmp(a));
        res.iter().take(count).copied().collect()
    }

    pub async fn add(&self, player: UserId, score: u64) {
        self.with_write(|guard| {
            *guard.entry(player).or_default() += score;
        }).await;
    }
    
    pub async fn add_from_game(&self, game: &game::GameMP) {
        self.with_write(|guard| {
            let score = game.get_score();
            for i in 0..=1 {
                *guard.entry(game.get_user_id(i)).or_default() += score[i];
            }
        }).await;
    }

    pub async fn get(&self, player: UserId) -> Option<u64> {
        let guard = self.scores.read().await;
        guard.get(&player).copied()
    }
}