/**
 * Query wrapper functions for Discord commands.
 * Designed to wrap around other functions and emit CmdErrors on failure.
 */
use super::errors::{CmdError, CmdResult};
use crate::constants;
use crate::dict;
use crate::UserData;
use crate::game;

/**
 * Check if a given word length is acceptable.
 * Only returns anything meaningful – that is, an error – if not.
 */
pub fn test_length(l: usize) -> CmdResult<()> {
    if l > constants::MAX_WORDSIZE || l < constants::MIN_WORDSIZE {
        return Err(CmdError::BadWordLength(l));
    }
    Ok(())
}

/**
 * Ensure a word is in the dictionary, return it (lowercase) if so.
 * Also handles random word queries!
 */
pub fn ensure_word(d: &dict::Dictionary, s: &str) -> CmdResult<String> {
    if let Ok(len) = s.parse::<usize>() {
        // Get a random word instead.
        test_length(len)?;
        return d.random_with_len(len)
            .ok_or(CmdError::BadWordLength(s.len()))
            .map(|s| s.to_lowercase());
    }
    test_length(s.len())?;
    if !d.contains(s) {
        return Err(CmdError::WordNotFound(s.into()));
    }
    Ok(s.to_lowercase())
}

/**
 * Extract user data of own user and ID of game owner from get_mut, assuming the user is in a game.
 */
pub fn unwrap_game_id(userdata: Option<&mut UserData>) -> CmdResult<(&mut UserData, game::GameId)> {
    userdata.and_then(|udata| {
        udata.player.timed_game.map(|gid| (udata, gid))
    }).ok_or(CmdError::NoGame)
}