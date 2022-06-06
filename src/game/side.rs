use crate::dict::wordmatch::*;
use poise::serenity_prelude as serenity;
use serenity::UserId;
use std::collections::HashMap;

// Per-side data.
pub struct GameSide {
    pub id: UserId,
    pub baseword: String,
    pub guesses: Vec<(String, Vec<MatchLetter>)>,
    pub keyboard: HashMap<char, MatchLetter>,
}

impl GameSide {
    pub fn with_id(uid: UserId) -> GameSide {
        GameSide {
            id: uid,
            guesses: Vec::new(),
            baseword: String::new(),
            keyboard: HashMap::new(),
        }
    }

    // Returns score, assuming that the last guess is winning.
    // seconds: time advantage over opposite player, in seconds (0 if last)
    pub fn calculate_score(&self, seconds: u64, max_guesses: usize) -> u64 {
        seconds + (1 + max_guesses - std::cmp::min(self.guesses.len(), max_guesses)) as u64 * 5
    }

    // Returns true if guess results in victory..
    pub fn push_guess(&mut self, guess: String) -> bool {
        let wmatch = match_word(&self.baseword, &guess);
        guess
            .chars()
            .zip(wmatch.iter())
            .map(|(c, e)| self.keyboard.insert(c, *e));
        self.guesses.push((guess, wmatch));
        self.victorious()
    }

    // See if the last guess is an exact match for every letter.
    // That's the win condition.
    pub fn victorious(&self) -> bool {
        self.guesses.last().map_or(false, |g| {
            g.1 // within match vector
                .iter()
                .all(|&e| e == MatchLetter::Exact)
        }) // test if all matches exact
    }
}
