use crate::data::CtxData;
use poise::serenity_prelude as serenity;
use serenity::UserId;
use crate::constants;
use crate::game::*;
use crate::data::*;
use std::time::SystemTime;
use super::{CmdError, CmdResult};

impl CtxData {
    pub async fn challenge_player(&self,
        own_id: UserId, enemy_id: UserId, word: String, variant: GameVariant
    ) -> CmdResult<GameId> {
        use GameVariant::*;
        if own_id == enemy_id {
            return Err(CmdError::BadAccept);
        }

        let mut udlock = self.userdata.write().await;
        let mut mplock = self.mpgames.write().await;

        let (game_id, gamedata) = {
            // Own data scope
            let userdata1 = udlock.entry(own_id).or_default();
            if matches!(variant, Timed) && userdata1.player.timed_game.is_some() {
                return Err(CmdError::SelfInGame);
            }
            let game_id = self.pull_gameid();
            let gamedata = GameMP::create(own_id, enemy_id, word, variant);
            match variant {
                Timed => {userdata1.player.timed_game = Some(game_id);},
                TurnBased => {userdata1.player.turn_games.insert(enemy_id, game_id);}
            }
            (game_id, gamedata)
        };

        mplock.insert(
            game_id,
            gamedata
        );

        // Access opponent data
        udlock
            .entry(enemy_id)
            .or_default()
            .player
            .invite(variant, own_id, Invite {
                game: game_id,
                expiry: SystemTime::now() + match variant {
                    Timed => constants::TIMED_INVITE_EXPIRY,
                    TurnBased => constants::TURN_INVITE_EXPIRY,
                }});
        Ok(game_id)
    }
    
    pub async fn accept_invite(&self,
        own_id: UserId, enemy_id: UserId, word: String, variant: GameVariant
    ) -> CmdResult<GameId> {
        let mut udlock = self.userdata.write().await;

        let mut userdata = udlock.entry(own_id).or_default();
        if userdata.player.timed_game.is_some() {
            return Err(CmdError::SelfInGame.into());
        }

        let game_id = userdata.player
            .list(variant)
            .get(&enemy_id)
            .ok_or(CmdError::NoInvite)? // important point 1
            .game;
        let mut mplock = self.mpgames.write().await;
        let gamedata = mplock.get_mut(&game_id).ok_or(CmdError::GameDeleted)?;

        if !matches!(gamedata.get_progress(), multiplayer::GameProgress::Waiting) {
            userdata.player.remove_invite(variant, enemy_id);
            return Err(CmdError::GameStarted(true).into());
        }
        if gamedata.get_word_length() != word.len() {
            userdata.player.remove_invite(variant, enemy_id);
            return Err(CmdError::BadWordLength(word.len()).into());
        }

        // Unwrapping because [1]
        if !userdata.player.accept(variant, enemy_id).unwrap() {
            return Err(CmdError::BadAccept.into());
        }
        gamedata.respond(word, own_id)?;
        Ok(game_id)
    }
}