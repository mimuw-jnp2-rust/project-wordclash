use crate::constants;
use crate::game::*;
use crate::{Context, Error, UserData};
use poise::serenity_prelude as serenity;
use super::util::*;
use std::fmt::Write;

/// Challenge an user to a Worduel
///
/// Supplied word must be within reasonable length bounds
/// and appear in the dictionary.
#[poise::command(slash_command, category = "Worduel", ephemeral)]
pub async fn worduel_challenge(
    ctx: Context<'_>,
    #[description = "Challenged user"] user: serenity::User,
    #[description = "Challenge word"] word: String,
) -> Result<(), Error> {
    if word == "crashity" {
        return Err(std::fmt::Error.into()).into();
    }
    let word = queries::ensure_word(&ctx.data().dict, &word)?;
    if user.eq(ctx.author()) {
        return Err(CmdError::BadAccept.into());
    }

    let own_id = ctx.author().id;
    let other_id = user.id;
    let mut udlock = ctx.data().userdata.write().await;

    if let Some(userdata2) = udlock.get(&user.id) {
        match userdata2.player.game {
            ActiveGame::None => {},
            _ => return Err(CmdError::TargetInGame.into()),
        }
    }

    let mut mplock = ctx.data().mpgames.write().await;
    {
        // Own data scope
        let userdata1 = udlock.entry(own_id).or_insert_with(UserData::new);
        if !matches!(userdata1.player.game, ActiveGame::None) {
            return Err(CmdError::SelfInGame.into());
        }
        mplock.insert(
            own_id,
            GameMP::create(own_id, other_id, word.to_lowercase()),
        );
        userdata1.player.game = ActiveGame::Multiplayer(own_id);
    }
    // Access opponent data
    udlock
        .entry(other_id)
        .or_insert_with(UserData::new)
        .player
        .game = ActiveGame::Multiplayer(own_id);
    let gamedata = mplock.get(&own_id).unwrap(); // why wouldn't it exist?

    ctx.channel_id().send_message(&ctx.discord().http, |m| {
        m.content(
            serenity::MessageBuilder::new()
                .push("You have been challenged to a Worduel, ")
                .user(user)
                .push("!")
                .build(),
        )
        .embed(|e| {
            e.title("Worduel challenge")
                .description(format!(
                    "Word length: {}\nMax guesses: {}",
                    gamedata.get_word_length(),
                    gamedata.get_max_guesses(),
                ))
                .color((255, 204, 11))
        })
    })
    .await?;
    Ok(())
}

/// Accept a Worduel invitation
///
/// The word you specify will be what the inviter has to guess.
#[poise::command(slash_command, category = "Worduel", ephemeral)]
pub async fn worduel_accept(
    ctx: Context<'_>,
    #[description = "Response word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;

    let (userdata, other_id) = queries::unwrap_game_id(udlock.get_mut(&own_id))?;

    if other_id == own_id {
        return Err(CmdError::BadAccept.into());
    }
    let other_user = other_id.to_user(&ctx.discord().http).await?;

    let mut mplock = ctx.data().mpgames.write().await;
    let gamedata = match mplock.get_mut(&other_id) {
        Some(d) => d,
        None => {
            userdata.player.game = ActiveGame::None;
            return Err(CmdError::GameDeleted.into());
        }
    };
    if gamedata.get_word_length() != word.len() {
        return Err(CmdError::BadWordLength(word.len()).into());
    }
    if !gamedata.respond(word) {
        return Err(CmdError::BadAccept.into());
    }
    ctx.channel_id().send_message(&ctx.discord().http, |m| {
        m.content(
            serenity::MessageBuilder::new()
                .push("Challenge accepted, ")
                .user(other_user)
                .push("!")
                .build()
        )
    })
    .await?;
    Ok(())
}

/// Send a guess to the current Worduel
///
/// On your side, of course.
/// The game ends for you if you get an exact match
/// or if you run out of guesses.
#[poise::command(slash_command, category = "Worduel", ephemeral)]
pub async fn worduel_send(
    ctx: Context<'_>,
    #[description = "Sent word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;
    let mut mplock = ctx.data().mpgames.write().await;

    let (progress, stateline, content, views, game_id, other_id) = {
        let (userdata, game_id) = queries::unwrap_game_id(udlock.get_mut(&own_id))?;

        let gamedata = match mplock.get_mut(&game_id) {
            Some(d) => d,
            None => {
                userdata.player.game = ActiveGame::None;
                return Err(CmdError::GameDeleted.into());
            }
        };
        // If we got gamedata, this should REALLY not be None.
        let player_index = gamedata.match_user(own_id).unwrap();
        let enemy_id = gamedata.get_user_id(1 - player_index);

        use multiplayer::GameProgress::*;
        if matches!(gamedata.get_progress(), Waiting) {
            return Err(CmdError::GameStarted(false).into());
        }

        let success = gamedata.send_guess(player_index, word.to_lowercase());
        let progress = gamedata.get_progress().clone();

        let mut state = serenity::MessageBuilder::new();
        match progress {
            Waiting => state.push("Waiting (this should not appear)"),
            Started => state.push("Both players active, game in progress"),
            Ending(i) => state
                .push("Player ")
                .push(i.to_string())
                .push(" finished in ")
                .push(
                    gamedata
                        .get_end(i)
                        .map(|e| format!("{} seconds", (e - gamedata.get_start()).as_secs()))
                        .unwrap_or_else(|| "some time".to_string()),
                )
                .push(", game in progress"),
            Over(None) => state.push("Game over (draw)"),
            Over(Some(i)) => {
                let id = gamedata.get_user_id(i);
                let scores = gamedata.get_score();
                state
                    .push("Game over (winner: ")
                    .user(id);
                write!(state.0, ", score: {}:{})", scores[i], scores[1 - i])?;
                &mut state
            }
        };
        let views = gamedata.render_views(constants::WORDUEL_VIEWSEP);

        let mut content = serenity::MessageBuilder::new();
        match progress {
            Over(Some(i)) => content
                .push("Game over, ")
                .user(enemy_id)
                .push(", the victor is ")
                .user(gamedata.get_user_id(i))
                .push("!"),
            Over(None) => content
                .push("Game over, ")
                .user(enemy_id)
                .push(", this duel ended in a draw."),
            _ => {
                if success {
                    content.push("Word has been sent!")
                } else {
                    content.push("Word rejected, wait for the other side to finish.")
                }
            }
        };
        (
            progress,
            state.build(),
            content.build(),
            views,
            game_id,
            enemy_id,
        )
    };

    if matches!(progress, multiplayer::GameProgress::Over(_)) {
        mplock.remove(&game_id);
        if let Some(udata) = udlock.get_mut(&own_id) {
            udata.player.game = ActiveGame::None;
        }
        if let Some(udata2) = udlock.get_mut(&other_id) {
            udata2.player.game = ActiveGame::None;
        }
    }

    ctx.send(|m| {
        m.content(content).embed(|e| {
            e.title("Worduel status")
                .field("Game state", stateline, true)
                .color((255, 204, 11))
                .description(format!("```\n{}\n```", views))
        }).ephemeral(false)
    })
    .await?;
    Ok(())
}

/// Forfeit from a Worduel
///
/// Can also be used to reject invitations.
/// To make sure you don't accidentally forfeit,
/// you have to specify the enemy's tag in the command invocation.
#[poise::command(slash_command, category = "Worduel")]
pub async fn worduel_forfeit(
    ctx: Context<'_>,
    #[description = "Enemy username"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;
    let mut mplock = ctx.data().mpgames.write().await;

    let user_unwrapped = match user {
        None => return Err(CmdError::ForfeitBadUser.into()),
        Some(u) => u,
    };

    let (stateline, content, views, game_id, other_id) = {
        let (userdata, game_id) = queries::unwrap_game_id(udlock.get_mut(&own_id))?;

        let gamedata = match mplock.get_mut(&game_id) {
            Some(d) => d,
            None => {
                userdata.player.game = ActiveGame::None;
                return Err(CmdError::GameDeleted.into());
            }
        };
        let player_index = gamedata.match_user(own_id).unwrap();
        let enemy_id = gamedata.get_user_id(1 - player_index);

        if user_unwrapped.id != enemy_id {
            return Err(CmdError::ForfeitBadUser.into());
        }

        use multiplayer::GameProgress::*;
        let progress = gamedata.get_progress().clone();

        let mut state = serenity::MessageBuilder::new();
        match progress {
            Waiting => state.push("Waiting for acceptance"),
            Started => state.push("Both players active, game in progress"),
            Ending(i) => state
                .push("Player ")
                .push(i.to_string())
                .push(" finished in ")
                .push(
                    gamedata
                        .get_end(i)
                        .map(|e| format!("{} seconds", (e - gamedata.get_start()).as_secs()))
                        .unwrap_or_else(|| "some time".to_string()),
                )
                .push(", game in progress"),
            Over(_) => state.push("Game over"),
        };
        let views = gamedata.render_views(constants::WORDUEL_VIEWSEP);

        let mut content = serenity::MessageBuilder::new();
        match progress {
            Waiting => content
                .user(enemy_id)
                .push(", your invitation has been rejected."),
            _ => content
                .user(enemy_id)
                .push(", your opponent has forfeited this game."),
        };
        userdata.player.game = ActiveGame::None;
        (state.build(), content.build(), views, game_id, enemy_id)
    };

    mplock.remove(&game_id);
    if let Some(udata2) = udlock.get_mut(&other_id) {
        udata2.player.game = ActiveGame::None;
    }

    ctx.send(|m| {
        m.content(content).embed(|e| {
            e.title("Worduel status before forfeit")
                .field("Last game state", stateline, true)
                .color((255, 204, 11))
                .description(format!("```\n{}\n```", views))
        })
    })
    .await?;
    Ok(())
}

/// Show the letter usage in your current game.
///
/// This is a display-only keyboard, you can't use it for input.
#[poise::command(slash_command, category = "Worduel", ephemeral)]
pub async fn worduel_keyboard(ctx: Context<'_>) -> Result<(), Error> {
    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;
    let mut mplock = ctx.data().mpgames.write().await;

    let (userdata, game_id) = queries::unwrap_game_id(udlock.get_mut(&own_id))?;

    let gamedata = match mplock.get_mut(&game_id) {
        Some(d) => d,
        None => {
            userdata.player.game = ActiveGame::None;
            return Err(CmdError::GameDeleted.into());
        }
    };
    let player_index = gamedata.match_user(game_id).unwrap();

    let keyboard = gamedata.render_keyboard(player_index);

    ctx.send(|m| m.content(keyboard)).await?;
    Ok(())
}
