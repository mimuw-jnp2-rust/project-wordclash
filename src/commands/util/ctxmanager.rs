use poise::serenity_prelude as serenity;
use serenity::UserId;
use crate::constants;
use crate::game::*;
use crate::data::*;
use super::queries;
use std::time::SystemTime;
use super::{CmdError, CmdResult};

impl CtxData {
    pub async fn challenge_player(&self,
        own_id: UserId, enemy_id: UserId, word: String, variant: GameVariant
    ) -> CmdResult<GameId> {
        use GameVariant::*;
        if own_id == enemy_id {
            return Err(CmdError::ChallengedSelf);
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

        let userdata = udlock.entry(own_id).or_default();
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
    
    pub async fn reject_invite(&self,
        own_id: UserId, enemy_id: UserId, variant: GameVariant
    ) -> CmdResult<Option<GameMP>> {
        let mut udlock = self.userdata.write().await;
        let userdata = udlock.entry(own_id).or_default();

        let game_id = userdata.player
            .list(variant)
            .get(&enemy_id)
            .ok_or(CmdError::NoInvite)? // important point 1
            .game;
        let mut mplock = self.mpgames.write().await;

        userdata.player.remove_invite(variant, enemy_id).ok_or(CmdError::NoInvite)?;
        Ok(mplock.remove(&game_id))
    }
    
    // Perform a function on an active (caller-bound) timed game. 
    // Takes a function which has to take four parameters:
    // - userdata, game_id, gamedata: obvious
    // - remove: function to call if the game is to be removed
    pub async fn act_on_timed<T, F: FnOnce(&mut UserData, GameId, &mut GameMP, &mut dyn FnMut(bool))-> CmdResult<T>> (&self,
        own_id: UserId, f: F
    ) -> CmdResult<T> {
        let mut udlock = self.userdata.write().await;
        let mut mplock = self.mpgames.write().await;

        let (userdata, game_id) = queries::unwrap_timedgame_id(udlock.get_mut(&own_id))?;

        let gamedata = mplock.get_mut(&game_id).ok_or_else(|| {
            userdata.player.timed_game = None;
            CmdError::GameDeleted
        })?;

        // If we got gamedata, this should REALLY not be None.
        let player_index = gamedata.match_user(own_id).unwrap();
        let enemy_id = gamedata.get_user_id(1 - player_index);

        let mut should_remove = false;
        let mut should_commit_scores = false;
        let res = f(userdata, game_id, gamedata, &mut |scores| {
            should_remove = true;
            should_commit_scores = scores;
        });
        if should_commit_scores {
            self.scores().add_from_game(gamedata).await;
        }
        if should_remove {
            mplock.remove(&game_id);
            userdata.player.timed_game = None;
            if let Some(udata2) = udlock.get_mut(&enemy_id) {
                udata2.player.timed_game = None;
            }
        }
        res
    }
    
    // Perform a function on an active (caller-bound) turn-based game. 
    // Takes a function which has to take four parameters:
    // - userdata, game_id, gamedata: obvious
    // - remove: function to call if the game is to be removed
    //     Takes a bool indicating whether or not to commit the score.
    pub async fn act_on_turnbased<T, F: FnOnce(&mut UserData, GameId, &mut GameMP, &mut dyn FnMut(bool))-> CmdResult<T>> (&self,
        own_id: UserId, enemy_id: UserId, f: F
    ) -> CmdResult<T> {
        let mut udlock = self.userdata.write().await;
        let mut mplock = self.mpgames.write().await;

        let (userdata, game_id) = queries::unwrap_turngame_id(udlock.get_mut(&own_id), enemy_id)?;

        let gamedata = mplock.get_mut(&game_id).ok_or_else(|| {
            userdata.player.turn_games.remove(&enemy_id);
            CmdError::GameDeleted
        })?;

        let mut should_remove = false;
        let mut should_commit_scores = false;
        let res = f(userdata, game_id, gamedata, &mut |scores| {
            should_remove = true;
            should_commit_scores = scores;
        });
        if should_commit_scores {
            self.scores().add_from_game(gamedata).await;
        }
        if should_remove {
            mplock.remove(&game_id);
            userdata.player.turn_games.remove(&enemy_id);
            if let Some(udata2) = udlock.get_mut(&enemy_id) {
                udata2.player.turn_games.remove(&own_id);
            }
        }
        res
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::dict;
    use crate::commands::util::CmdError;
    use dict::{Dictionary, DictSet};
    use GameVariant::*;
    use multiplayer::GameProgress::*;
    
    fn get_dict() -> Dictionary {
        let micro_dictset =
            ["rover", "tower", "ready", "tears", "river", "smile", "quick", "slate", "sheet", "rails"]
                .iter()
                .map(|s| s.to_string())
                .collect::<DictSet>();
        Dictionary::new(micro_dictset)
    }

    #[tokio::test]
    async fn test_full_game() {
        let u1 = UserId::from(44210404);
        let u2 = UserId::from(44210405);
        let u3 = UserId::from(44210406);
        let ctx = CtxData::new(get_dict());
        assert!(ctx.accept_invite(u2, u1, "tower".to_string(), TurnBased).await.is_err());
        assert!(ctx.challenge_player(u1, u2, "rails".to_string(), TurnBased).await.is_ok());
        assert!(ctx.accept_invite(u2, u1, "tower".to_string(), TurnBased).await.is_ok());
        
        assert!(matches!(ctx.act_on_turnbased(u3, u2, |_, _, _, _| Ok(())).await,
            Err(CmdError::NoGame)));
        
        for w in ["slate", "sheet", "tears", "tower"] {
            assert!(ctx.act_on_turnbased(u1, u2, |_, _, g, _| {
                assert!(matches!(g.get_progress(), Started));
                g.send_guess(0, w.to_string());
                Ok(())
            }).await.is_ok());
        }
        for w in ["river", "ready", "rails"] {
            assert!(ctx.act_on_turnbased(u2, u1, |_, _, g, r| {
                assert!(matches!(g.get_progress(), Ending(0)));
                g.send_guess(1, w.to_string());
                match g.get_progress() {
                    Ending(_) => {},
                    Over(Some(_)) => {r(true);},
                    _ => {assert!(false);},
                }
                Ok(())
            }).await.is_ok());
        }
        assert!(matches!(ctx.act_on_turnbased(u2, u1, |_, _, g, _| {
            println!("{:?}", g.get_progress());
            Ok(())
        }).await, Err(CmdError::NoGame)));
        let scores = ctx.scores().list_top(10).await;
        assert!(scores.len() == 2);
        assert!(scores[0].0 == u2);
        assert!(scores[1].0 == u1);
    }
}