use crate::dict::wordmatch::*;
use crate::commands::util::{CmdError, CmdResult};
use poise::serenity_prelude as serenity;
use serenity::UserId;
use std::time::Instant;

use super::side::GameSide;
use super::GameVariant;

// Game progress
// Used to remember whether a game has started and whether it's over
#[derive(Debug, Clone, Copy)]
pub enum GameProgress {
    Waiting,
    Started,
    Ending(usize),       // .0 indicates player who finished
    Over(Option<usize>), // .0 indicates winning player if Some, or draw if None
}

// Per-game data
pub struct GameMP {
    side: [GameSide; 2],
    start: Instant,
    end: [Option<Instant>; 2],
    progress: GameProgress,
    score: [u64; 2],
    max_guesses: usize,
    variant: GameVariant,
}

const PLAYER_CAP: usize = 2;
const EMOJI_WIDTH: usize = 25;
const ALPHA_LENGTH: usize = 26;

impl GameMP {
    // Start of a game.
    pub fn create(id_self: UserId, id_challenged: UserId, word: String, variant: GameVariant) -> GameMP {
        let mut out = GameMP {
            side: [GameSide::with_id(id_self), GameSide::with_id(id_challenged)],
            start: Instant::now(),
            end: [None, None],
            progress: GameProgress::Waiting,
            score: [0, 0],
            max_guesses: word.len() + 1,
            variant
        };
        out.side[1].baseword = word;

        out
    }

    #[inline]
    pub fn get_word_length(&self) -> usize {
        self.side[1].baseword.len()
    }

    // Match an user ID to a side index.
    pub fn match_user(&self, id: UserId) -> Option<usize> {
        for i in 0..2 {
            if self.side[i].id == id {
                return Some(i);
            }
        }
        None
    }

    // Respond to started game with a word for the challenger, and start the game if valid.
    // Includes challenger ID as a sanity check.
    pub fn respond(&mut self, word: String, id: UserId) -> CmdResult<()> {
        if !matches!(self.progress, GameProgress::Waiting) {
            return Err(CmdError::GameStarted(true));
        }
        if word.len() != self.get_word_length() {
            return Err(CmdError::BadWordLength(word.len()));
        }
        if id != self.side[1].id {
            return Err(CmdError::BadAccept);
        }
        self.side[0].baseword = word;
        self.progress = GameProgress::Started;
        self.start = Instant::now();
        Ok(())
    }

    fn calculate_scores(&mut self) {
        for end in self.end {
            if end.is_none() {
                return;
            }
        }
        let spans = self.end.map(|e| e.unwrap().duration_since(self.start));
        // The last of the two ends
        let max_end = spans.iter().max().unwrap();
        // Duration to add before as_secs to achieve "rounding up" behavior
        // Equal to one second minus one smallest unit of duration (ns)
        let near_second = std::time::Duration::from_nanos(999_999_999);
        // Second count based on spans, used as input for score calculation
        let mut secscores = spans.map(|s| (*max_end - s + near_second).as_secs());

        // Cache victory result
        let victory: Vec<_> = self.side.iter().map(|s| s.victorious()).collect();
        if !victory.iter().all(|x| *x) {
            secscores = [0, 0];
        }

        for i in 0..2 {
            if !victory[i] {
                self.score[i] = 0;
            } else {
                self.score[i] = match self.variant {
                    GameVariant::Timed => self.side[i].calculate_timed_score(secscores[i], self.max_guesses),
                    GameVariant::TurnBased => self.side[i].calculate_turn_score(self.max_guesses, self.get_word_length()),
                };
            }
        }
    }
    // Send a guess as player number `index`.
    // Returns true if accepted (which is not an error). Adjusts progress.
    pub fn send_guess(&mut self, index: usize, guess: String) -> bool {
        if index >= PLAYER_CAP || self.get_word_length() != guess.len() {
            return false;
        }
        match self.progress {
            GameProgress::Started => {
                let finished = self.side[index].push_guess(guess);
                // Move on to Ending if finished or if out of guesses
                if finished || self.side[index].guesses.len() == self.max_guesses {
                    self.progress = GameProgress::Ending(index);
                    self.end[index] = Some(Instant::now());
                }
                true
            }
            GameProgress::Ending(other) => {
                if other == index {
                    return false;
                }
                let finished = self.side[index].push_guess(guess);

                if finished || self.side[index].guesses.len() == self.max_guesses {
                    self.end[index] = Some(Instant::now());
                    self.calculate_scores();

                    use std::cmp::Ordering::*;
                    use GameProgress::Over;
                    match self.score[0].cmp(&self.score[1]) {
                        Less => {
                            self.progress = Over(Some(1));
                        }
                        Equal => {
                            self.progress = Over(None);
                        }
                        Greater => {
                            self.progress = Over(Some(0));
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn render_view(&self, index: usize) -> String {
        let mut out = String::with_capacity(self.max_guesses * 20);
        let empty_line: String = (0..self.get_word_length() * 3).map(|_| ' ').collect();
        let side = &self.side[index];

        for i in 0..self.max_guesses {
            if let Some(row) = side.guesses.get(i) {
                row.0.chars().zip(row.1.iter()).for_each(|(c, m)| {
                    use MatchLetter::*;
                    let a_str = match m {
                        Null => format!(" {} ", c).to_uppercase(),
                        Close => format!(":{}:", c).to_uppercase(),
                        Exact => format!("[{}]", c).to_uppercase(),
                    };
                    out.push_str(&a_str);
                });
            } else {
                out.push_str(&empty_line);
            }
            if i + 1 < self.max_guesses {
                out.push('\n');
            }
        }
        out
    }

    pub fn render_view_color(&self, index: usize) -> String {
        let mut out = String::with_capacity(self.max_guesses * EMOJI_WIDTH * self.get_word_length() * 2);
        let empty_line: String = (0..self.get_word_length()).map(|_| ":white_large_square:").collect();
        let side = &self.side[index];

        for i in 0..self.max_guesses {
            if let Some(row) = side.guesses.get(i) {
                let mut l_out = String::with_capacity(EMOJI_WIDTH * self.get_word_length());
                let mut a_out = String::with_capacity(EMOJI_WIDTH * self.get_word_length());
                use std::fmt::Write;
                row.0.chars().for_each(|c| {
                    write!(l_out, ":regional_indicator_{}:\u{200b}", c).unwrap();
                });
                use MatchLetter::*;
                row.1.iter().for_each(|m| match m {
                    Null => a_out.push_str(":black_large_square:"),
                    Close => a_out.push_str(":yellow_square:"),
                    Exact => a_out.push_str(":green_square:"),
                });
                out.push_str(&l_out);
                out.push('\n');
                out.push_str(&a_out);
            } else {
                out.push_str(&empty_line);
                out.push('\n');
                out.push_str(&empty_line);
            }
            if i + 1 < self.max_guesses {
                out.push('\n');
            }
        }
        out
    }
    
    pub fn render_stateline(&self, want_scores: bool) -> String {
        let mut state = serenity::MessageBuilder::new();
        use GameProgress::*;
        use std::fmt::Write;
        match self.progress {
            Waiting => state.push("Waiting"),
            Started => state.push("Both players active, game in progress"),
            Ending(i) => state
                .push("Player ")
                .push(i.to_string())
                .push(" finished in ")
                .push(
                    self
                        .get_end(i)
                        .map(|e| format!("{} seconds", (e - self.get_start()).as_secs()))
                        .unwrap_or_else(|| "some time".to_string()),
                )
                .push(", game in progress"),
            Over(None) => state.push("Game over (draw)"),
            Over(Some(i)) => {
                let id = self.get_user_id(i);
                let scores = self.get_score();
                state.push("Game over");
                if want_scores {
                    state.push("(winner: ").user(id);
                    write!(state.0, ", score: {}:{})", scores[i], scores[1 - i]).unwrap();
                }
                &mut state
            }
        }.build()
    }

    pub fn render_keyboard(&self, index: usize) -> String {
        let rows = vec!["qwertyuiop", "asdfghjkl", "zxcvbnm"];

        let mut out = String::with_capacity(ALPHA_LENGTH * EMOJI_WIDTH);
        let side = &self.side[index];

        for row in rows {
            for letter in row.chars() {
                let emoji_str = format!(":regional_indicator_{}: ", letter);
                out.push_str(&emoji_str);
            }
            out.push('\n');
            for letter in row.chars() {
                let emoji_str = match side.keyboard.get(&letter) {
                    None => ":white_large_square: ",
                    Some(MatchLetter::Null) => ":black_large_square: ",
                    Some(MatchLetter::Close) => ":yellow_square: ",
                    Some(MatchLetter::Exact) => ":green_square: ",
                };
                out.push_str(&emoji_str);
            }
            out.push('\n');
        }
        out
    }

    // Render views side by side, separated with `separator`.
    pub fn render_views(&self, separator: &str) -> String {
        self.render_view_color(0)
            .split('\n')
            .zip(self.render_view(1).split('\n'))
            .map(|(a, b)| [a, b].join(separator))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // Getter methods.
    pub fn get_start(&self) -> Instant {
        self.start
    }

    pub fn get_end(&self, index: usize) -> Option<Instant> {
        self.end.get(index).and_then(|e| *e)
    }

    pub fn get_baseword(&self, index: usize) -> &str {
        self.side[index].baseword.as_str()
    }

    pub fn get_score(&self) -> &[u64; 2] {
        &self.score
    }

    pub fn get_max_guesses(&self) -> usize {
        self.max_guesses
    }

    pub fn get_progress(&self) -> &GameProgress {
        &self.progress
    }

    pub fn get_user_id(&self, index: usize) -> UserId {
        self.side[index].id
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::constants;
    use poise::serenity_prelude as serenity;
    use serenity::UserId;

    #[test]
    fn basic_game() {
        let u1 = UserId(1011);
        let u2 = UserId(1013);
        let mut game = GameMP::create(u1, u2, "north".to_string(), GameVariant::Timed);
        assert!(matches!(game.match_user(u2), Some(1)));
        assert!(matches!(game.match_user(u1), Some(0)));
        assert!(matches!(game.match_user(UserId(1012)), None));

        assert!(matches!(game.get_progress(), GameProgress::Waiting));
        game.respond("slide".to_string(), u2).unwrap();
        assert!(matches!(game.get_progress(), GameProgress::Started));

        assert_eq!(game.send_guess(0, "tower".to_string()), true);
        assert_eq!(game.send_guess(1, "trial".to_string()), true);
        println!(
            "Game state:\n{}",
            game.render_views(constants::WORDUEL_VIEWSEP)
        );
        assert!(matches!(game.get_progress(), GameProgress::Started));

        assert!(game.send_guess(0, "lease".to_string()));
        assert!(game.send_guess(1, "rites".to_string()));
        assert!(game.send_guess(0, "slide".to_string()));
        assert!(matches!(game.get_progress(), GameProgress::Ending(0)));
        assert!(game.send_guess(1, "porty".to_string()));
        assert!(matches!(game.get_progress(), GameProgress::Ending(0)));
        println!(
            "Game state:\n{}",
            game.render_views(constants::WORDUEL_VIEWSEP)
        );

        assert!(matches!(game.get_progress(), GameProgress::Ending(0)));
        assert!(game.send_guess(1, "worth".to_string()));
        assert!(game.send_guess(1, "forth".to_string()));
        assert!(game.send_guess(1, "north".to_string()));
        println!(
            "Game state: \n{}",
            game.render_views(constants::WORDUEL_VIEWSEP)
        );
        assert!(matches!(game.get_progress(), GameProgress::Over(Some(0))));

        let score = *game.get_score();
        println!("Scores: {}, {}", score[0], score[1]);
        // Subject to change with changes in score calculation.
        // Comment out and edit the following line at will.
        assert_eq!(score, [21, 5]);
    }

    #[test]
    fn rejections() {
        let u1 = UserId(118_999_881_999_119_7253);
        let u2 = UserId(1_800_434_2637);
        let mut game = GameMP::create(u1, u2, "ounce".to_string(), GameVariant::Timed);
        assert!(matches!(game.get_progress(), GameProgress::Waiting));
        game.respond("scout".to_string(), u2).unwrap();
        assert!(matches!(game.get_progress(), GameProgress::Started));
        let view1 = game.render_views(constants::WORDUEL_VIEWSEP);

        assert_eq!(game.send_guess(0, "quince".to_string()), false);
        assert_eq!(game.send_guess(1, "rows".to_string()), false);

        assert_eq!(game.send_guess(2, "steed".to_string()), false);

        assert_eq!(game.render_views(constants::WORDUEL_VIEWSEP), view1);
        assert!(matches!(game.get_progress(), GameProgress::Started));
    }
}
