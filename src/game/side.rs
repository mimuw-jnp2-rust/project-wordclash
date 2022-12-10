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

    // Returns score for timed game, assuming that the last guess is winning.
    // seconds: time advantage over opposite player, in seconds (0 if last)
    pub fn calculate_timed_score(&self, seconds: u64, max_guesses: usize) -> u64 {
        seconds + (1 + max_guesses - std::cmp::min(self.guesses.len(), max_guesses)) as u64 * 3
    }

    // Returns score fo turn-based game, assuming that the last guess is winning.
    pub fn calculate_turn_score(&self, max_guesses: usize, diff_guesses: usize, word_length: usize) -> u64 {
        let base: f64 = (max_guesses - std::cmp::min(self.guesses.len(), max_guesses) + 1) as f64;
        (((diff_guesses as f64).powf(1.6) * 4.0 + base)
            * (word_length as f64)/5.0) as u64
    }

    // Returns true if guess results in victory.
    pub fn push_guess(&mut self, guess: String) -> bool {
        let wmatch = match_word(&self.baseword, &guess);
        guess.chars().zip(wmatch.iter()).for_each(|(c, e)| {
            // Without this max, repeated characters might mess with results
            let kbpos = self.keyboard.entry(c).or_insert(MatchLetter::Null);
            *kbpos = std::cmp::max(*kbpos, *e);
        });
        self.guesses.push((guess, wmatch));
        self.victorious()
    }

    // See if the last guess is an exact match for every letter.
    // That's the win condition.
    pub fn victorious(&self) -> bool {
        self.guesses.last().map_or(false, |g| {
            g.1 // within match vector
                .iter()
                .all(|&e| e == MatchLetter::Exact) // test if all matches exact
        })
    }
}
