use crate::constants;
use crate::game::*;
use crate::{Context, Error};
use crate::data::*;
use poise::serenity_prelude as serenity;
use super::util::*;
use std::time;

/// Challenge an user to a timed Worduel
///
/// Supplied word must be within reasonable length bounds
/// and appear in the dictionary.
#[poise::command(slash_command, category = "Worduel", rename = "wd_challenge", ephemeral)]
pub async fn worduel_challenge_timed(
    ctx: Context<'_>,
    #[description = "Challenged user"] user: serenity::User,
    #[description = "Challenge word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    let game_id = ctx.data().challenge_player(ctx.author().id, user.id, word, GameVariant::Timed).await?;

    let mplock = ctx.data().mpgames.read().await;
    let gamedata = mplock.get(&game_id).unwrap(); // why wouldn't it exist?

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
#[poise::command(slash_command, category = "Worduel", rename = "wd_accept", ephemeral)]
pub async fn worduel_accept_timed(
    ctx: Context<'_>,
    #[description = "Chosen challenger"] user: serenity::User,
    #[description = "Response word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    ctx.data().accept_invite(ctx.author().id, user.id, word, GameVariant::Timed).await?;

    ctx.channel_id().send_message(&ctx.discord().http, |m| {
        m.content(
            serenity::MessageBuilder::new()
                .push("Your challenge has been accepted by")
                .push(&ctx.author().name)
                .push(", ")
                .user(user.id)
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
#[poise::command(slash_command, category = "Worduel", rename = "wd_send", ephemeral)]
pub async fn worduel_send_timed(
    ctx: Context<'_>,
    #[description = "Sent word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;
    let mut mplock = ctx.data().mpgames.write().await;

    let (progress, stateline, content, views, game_id, other_id) = {
        let (userdata, game_id) = queries::unwrap_timedgame_id(udlock.get_mut(&own_id))?;

        let gamedata = mplock.get_mut(&game_id).ok_or_else(|| {
            userdata.player.timed_game = None;
            CmdError::GameDeleted
        })?;

        // If we got gamedata, this should REALLY not be None.
        let player_index = gamedata.match_user(own_id).unwrap();
        let enemy_id = gamedata.get_user_id(1 - player_index);

        use multiplayer::GameProgress::*;
        if matches!(gamedata.get_progress(), Waiting) {
            return Err(CmdError::GameStarted(false).into());
        }

        let success = gamedata.send_guess(player_index, word.to_lowercase());
        let progress = gamedata.get_progress().clone();

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
            gamedata.render_stateline(true),
            content.build(),
            views,
            game_id,
            enemy_id,
        )
    };

    if matches!(progress, multiplayer::GameProgress::Over(_)) {
        mplock.remove(&game_id);
        if let Some(udata) = udlock.get_mut(&own_id) {
            udata.player.timed_game = None;
        }
        if let Some(udata2) = udlock.get_mut(&other_id) {
            udata2.player.timed_game = None;
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
#[poise::command(slash_command, category = "Worduel", rename = "wd_forfeit", ephemeral)]
pub async fn worduel_forfeit_timed(
    ctx: Context<'_>,
    #[description = "Enemy username"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;
    let mut mplock = ctx.data().mpgames.write().await;

    let user_unwrapped = user.ok_or(CmdError::ForfeitBadUser)?;

    let (stateline, content, views, game_id, other_id) = {
        let (userdata, game_id) = queries::unwrap_timedgame_id(udlock.get_mut(&own_id))?;

        let gamedata = mplock.get_mut(&game_id).ok_or_else(|| {
            userdata.player.timed_game = None;
            CmdError::GameDeleted
        })?;
        let player_index = gamedata.match_user(own_id).unwrap();
        let enemy_id = gamedata.get_user_id(1 - player_index);

        if user_unwrapped.id != enemy_id {
            return Err(CmdError::ForfeitBadUser.into());
        }

        use multiplayer::GameProgress::*;
        let progress = gamedata.get_progress().clone();

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
        userdata.player.timed_game = None;
        (gamedata.render_stateline(false), content.build(), views, game_id, enemy_id)
    };

    mplock.remove(&game_id);
    if let Some(udata2) = udlock.get_mut(&other_id) {
        udata2.player.timed_game = None;
    }

    ctx.send(|m| {
        m.content(content).embed(|e| {
            e.title("Worduel status before forfeit")
                .field("Last game state", stateline, true)
                .color((255, 204, 11))
                .description(format!("```\n{}\n```", views))
        }).ephemeral(false)
    })
    .await?;
    Ok(())
}

/// Show the letter usage in your current game.
///
/// This is a display-only keyboard, you can't use it for input.
#[poise::command(slash_command, category = "Worduel", rename = "wd_kb", ephemeral)]
pub async fn worduel_keyboard_timed(ctx: Context<'_>) -> Result<(), Error> {
    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;
    let mut mplock = ctx.data().mpgames.write().await;

    let (userdata, game_id) = queries::unwrap_timedgame_id(udlock.get_mut(&own_id))?;

    let gamedata = mplock.get_mut(&game_id).ok_or_else(|| {
        userdata.player.timed_game = None;
        CmdError::GameDeleted
    })?;
    let player_index = gamedata.match_user(own_id).unwrap();

    let keyboard = gamedata.render_keyboard(player_index);

    ctx.send(|m| m.content(keyboard)).await?;
    Ok(())
}
