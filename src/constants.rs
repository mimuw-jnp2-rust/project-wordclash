use std::time::Duration;

pub const MIN_WORDSIZE: usize = 4;
pub const MAX_WORDSIZE: usize = 8;
pub const WORDUEL_VIEWSEP: &str = " \u{2502} ";
// How long does each invite type take to expire?
pub const TIMED_INVITE_EXPIRY: Duration = Duration::from_secs(300);
pub const TURN_INVITE_EXPIRY: Duration = Duration::from_secs(900);
// How long do timed games take to be interrupted early?
// Turn-based games do not expire with time once accepted
pub const TIMED_GAME_EXPIRY: Duration = Duration::from_secs(600);
// How often does the cleanup task run?
// It's more or less a stop-the-world cleanup mechanism unless I switch to concurrent maps.
// Too rare will make garbage stick around longer, too frequent will slow the bot down.
// Effectively limits the granularity of the three above constants.
pub const CLEANUP_INTERVAL: Duration = Duration::from_secs(30);