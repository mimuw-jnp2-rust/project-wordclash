/**
 * Query wrapper functions for Discord commands.
 * Designed to wrap around other functions and emit CmdErrors on failure.
 */
use super::errors::{CmdError, CmdResult};
use crate::constants;
use crate::dict;
use poise::serenity_prelude as serenity;
use crate::UserData;
use crate::game;

/**
 * Ensure a word is in the dictionary, return it (lowercase) if so.
 */
pub fn ensure_word(d: &dict::Dictionary, s: &str) -> CmdResult<String> {
    if s.len() > constants::MAX_WORDSIZE || s.len() < constants::MIN_WORDSIZE {
        return Err(CmdError::BadWordLength(s.len()));
    }
    if !d.contains(s) {
        return Err(CmdError::WordNotFound(s.into()));
    }
    Ok(s.to_lowercase())
}

/**
 * Extract user data of own user and ID of game owner from get_mut, assuming the user is in a game.
 */
pub fn unwrap_game_id(userdata: Option<&mut UserData>) -> CmdResult<(&mut UserData, serenity::UserId)> {
    userdata.and_then(|udata| {
        udata.player.timed_game.map(|gid| (udata, gid))
    }).ok_or(CmdError::NoGame)
}