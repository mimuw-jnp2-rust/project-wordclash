// Wordle implementation proper.
use std::time::Instant;
use poise::serenity_prelude as serenity;
use serenity::UserId;
use crate::dict::wordmatch::*;

// Per-side data.
pub struct GameSide {
    id: UserId,
    baseword: String,
    guesses: Vec<(String, Vec<MatchLetter>)>,
}

impl GameSide {
    pub fn with_id(uid: UserId) -> GameSide {
        GameSide {
            id: uid,
            guesses: Vec::new(),
            baseword: String::new(),
        }
    }
    
    pub fn count_guesses(&self) -> usize {
        self.guesses.len()
    }

    // Returns score, assuming that the last guess is winning.
    // seconds: time advantage over opposite player, in seconds (0 if last)
    pub fn calculate_score(&self, seconds: u64, max_guesses: usize) -> u64 {
        seconds + (1 + max_guesses - std::cmp::min(
            self.guesses.len(), max_guesses
        )) as u64 * 5
    }

    // Returns true if guess results in victory..
    pub fn push_guess(&mut self, guess: String) -> bool {
        let wmatch = match_word(&self.baseword, &guess);
        self.guesses.push((guess, wmatch));
        self.victorious()
    }
    
    // See if the last guess is an exact match for every letter.
    // That's the win condition.
    pub fn victorious(&self) -> bool {
        self.guesses.last()
            .map_or(false,
            |g| g.1 // within match vector
                .iter()
                .all(|&e| e == MatchLetter::Exact)) // test if all matches exact
    }
}

// Game progress
// Used to remember whether a game has started and whether it's over
#[derive(Debug, Clone)]
pub enum GameProgress {
    Waiting,
    Started,
    Ending(usize), // .0 indicates player who finished
    Over(Option<usize>), // .0 indicates winning player if Some, or draw if None
}

// Per-game data
pub struct GameData {
    side: [GameSide; 2],
    start: Instant,
    end: [Option<Instant>; 2],
    progress: GameProgress,
    score: [u64; 2],
    max_guesses: usize,
}

impl GameData {
    // Start of a game.
    pub fn create(id_self: UserId, id_challenged: UserId, word: String) -> GameData {
        let mut out = GameData {
            side: [GameSide::with_id(id_self), GameSide::with_id(id_challenged)],
            start: Instant::now(),
            end: [None, None],
            progress: GameProgress::Waiting,
            score: [0, 0],
            max_guesses: word.len() + 1,
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
    pub fn respond(&mut self, word: String) -> bool {
        if word.len() != self.get_word_length() {
            return false;
        }
        self.side[0].baseword = word;
        self.progress = GameProgress::Started;
        self.start = Instant::now();
        true
    }

    fn calculate_scores(&mut self) {
        for end in self.end {
            if end.is_none() {
                return;
            }
        }
        let spans = self.end.map(|e| e.unwrap().duration_since(self.start));
        // The last of the two ends
        let max_end = spans.iter()
            .max()
            .unwrap();
        // Duration to add before as_secs to achieve "rounding up" behavior
        // Equal to one second minus one smallest unit of duration (ns)
        let near_second = std::time::Duration::from_nanos(999_999_999);
        // Second count based on spans, used as input for score calculation
        let mut secscores = spans
            .map(|s| (*max_end - s + near_second).as_secs());

        // Cache victory result
        let victory: Vec<_> = self.side.iter().map(|s| s.victorious()).collect();
        if !victory.iter().all(|x| *x) {
            secscores = [0, 0];
        }

        for i in 0..2 {
            if !victory[i] {
                self.score[i] = 0;
            } else {
                self.score[i] = self.side[i]
                    .calculate_score(secscores[i], self.max_guesses);
            }
        }
    }
    // Send a guess.
    // Returns true if accepted. Adjusts progress.
    pub fn send_guess(&mut self, id: UserId, guess: String) -> bool {
        let index_option = self.match_user(id);
        if index_option.is_none() {
            return false;
        }
        let index = index_option.unwrap();
        if self.side[index].baseword.len() != guess.len() {
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
            },
            GameProgress::Ending(other) => {
                if other == index {
                    return false;
                }
                let finished = self.side[index].push_guess(guess);

                if finished || self.side[index].guesses.len() == self.max_guesses {
                    self.end[index] = Some(Instant::now());
                    self.calculate_scores();

                    use GameProgress::Over;
                    use std::cmp::Ordering::*;
                    match self.score[0].cmp(&self.score[1]) {
                        Less => { self.progress = Over(Some(1)); },
                        Equal => { self.progress = Over(None); },
                        Greater => { self.progress = Over(Some(0)); },
                    }
                }
                true
            }
            _ => false
        }
    }
    
    pub fn render_view(&self, index: usize) -> String {
        let mut out = String::with_capacity(self.max_guesses * 20);
        let empty_line: String = (0..self.get_word_length() * 3)
            .map(|_| ' ')
            .collect();
        let side = &self.side[index];

        for i in 0..self.max_guesses {
            if let Some(row) = side.guesses.get(i) {
                row.0.chars()
                    .zip(row.1.iter())
                    .for_each(|(c, m)| {
                        use MatchLetter::*;
                        let a_str = match m {
                            Null =>  format!(" {} ", c).to_uppercase(),
                            Close => format!(":{}:", c).to_uppercase(),
                            Exact => format!("[{}]", c).to_uppercase(),
                        };
                        out.push_str(&a_str);
                    });
            } else {
                out.push_str(&empty_line);
            }
            if i+1 < self.max_guesses {
                out.push('\n');
            }
        };
        out
    }

    pub fn render_views(&self) -> String {
        self.render_view(0)
            .split('\n')
            .zip(self.render_view(1)
                .split('\n'))
            .map(|(a, b)| {
                [a, b].join(" \u{250a} ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_game() {
        let u1 = UserId(1011);
        let u2 = UserId(1013);
        let mut game = GameData::create(u1, u2, "north".to_string());
        assert!(matches!(game.progress, GameProgress::Waiting));
        game.respond("slide".to_string());
        assert!(matches!(game.progress, GameProgress::Started));

        assert_eq!(game.send_guess(u1, "tower".to_string()), true);
        assert_eq!(game.send_guess(u2, "trial".to_string()), true);
        println!("Game state:\n{}", game.render_views());
        assert!(matches!(game.progress, GameProgress::Started));
        
        assert!(game.send_guess(u1, "lease".to_string()));
        assert!(game.send_guess(u2, "rites".to_string()));
        assert!(game.send_guess(u1, "slide".to_string()));
        assert!(matches!(game.progress, GameProgress::Ending(0)));
        assert!(game.send_guess(u2, "porty".to_string()));
        assert!(matches!(game.progress, GameProgress::Ending(0)));
        println!("Game state:\n{}", game.render_views());
        
        assert!(matches!(game.progress, GameProgress::Ending(0)));
        assert!(game.send_guess(u2, "worth".to_string()));
        assert!(game.send_guess(u2, "forth".to_string()));
        assert!(game.send_guess(u2, "north".to_string()));
        println!("Game state: \n{}", game.render_views());
        assert!(matches!(game.progress, GameProgress::Over(Some(0))));
        
        println!("Scores: {}, {}", game.score[0], game.score[1]);
        // Subject to change with changes in score calculation.
        // Comment out and edit the following line at will.
        assert_eq!(game.score, [21, 5]);
    }

    #[test]
    fn rejections() {
        let u1 = UserId(118_999_881_999_119_7253);
        let u2 = UserId(1_800_434_2637);
        let mut game = GameData::create(u1, u2, "ounce".to_string());
        assert!(matches!(game.progress, GameProgress::Waiting));
        game.respond("scout".to_string());
        assert!(matches!(game.progress, GameProgress::Started));
        let view1 = game.render_views();

        assert_eq!(game.send_guess(u1, "quince".to_string()), false);
        assert_eq!(game.send_guess(u2, "rows".to_string()),   false);
        
        assert_eq!(game.send_guess(UserId(112), "steed".to_string()), false);
        
        assert_eq!(game.render_views(), view1);
        assert!(matches!(game.progress, GameProgress::Started));
    }
}