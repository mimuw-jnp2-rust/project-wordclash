use std::time::Duration;

pub const MIN_WORDSIZE: usize = 4;
pub const MAX_WORDSIZE: usize = 8;
pub const WORDUEL_VIEWSEP: &str = " \u{2502} ";
pub const TIMED_INVITE_EXPIRY: Duration = Duration::from_secs(300);
pub const TURN_INVITE_EXPIRY: Duration = Duration::from_secs(600);
pub const GAME_EXPIRY: Duration = Duration::from_secs(600);
