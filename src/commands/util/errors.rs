use std::{fmt, error};

/// Printable Discord command response.
pub trait CmdResponse {
    fn print<'a>(&self) -> String;
}

/// Invocation-related command error.
#[derive(Debug)]
pub enum CmdError {
    BadWordLength(usize),
    WordNotFound(String),
    NoGame, // no game to operate on
    BadAccept, // cannot accept this game
    SelfInGame, // you're in a game but shouldn't be
    TargetInGame, // opponent's in a game but shouldn't be
    GameDeleted, // game assigned but deleted
    ForfeitBadUser, // didn't mention the right user for a forfeiture
    GameStarted(bool), // game started?(bool) but opposite was expected
    #[allow(dead_code)]
    Misc(String), // unsorted
    #[allow(dead_code)]
    Hard(crate::Error), // pass down!
}

pub type CmdResult<R> = Result<R, CmdError>;

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CmdError::*;
        write!(f, "**Error:** ")?;
        match self {
            BadWordLength(size) => write!(f, "Word length invalid: {}", size),
            WordNotFound(s) => write!(f, "Word not found in dictionary: {}", s),
            NoGame => write!(f, "You are not in a game"),
            BadAccept => write!(f, "Cannot accept this game"),
            SelfInGame => write!(f, "You're already in a game"),
            TargetInGame => write!(f, "Target is already in a game"),
            GameDeleted => write!(f, "Game assigned but deleted"),
            ForfeitBadUser => write!(f, "To forfeit, specify your opponent's name"),
            GameStarted(s) => write!(f, "Game {} started", if *s {"not yet"} else {"already"}),
            Misc(s) => s.fmt(f),
            Hard(e) => {
                write!(f, "An error thrown from Rust was intercepted without unwrapping.
                    This should not appear.
                    {}", e)
            }
        }
    }
}

impl error::Error for CmdError {}