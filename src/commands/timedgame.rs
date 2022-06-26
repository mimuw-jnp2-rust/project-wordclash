use crate::constants;
use crate::game::*;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use super::util::*;

/// Challenge an user to a timed Worduel
///
/// Supplied word must be within reasonable length bounds
/// and appear in the dictionary.
/// Alternatively, if it's an integer, a random word of that length will be chosen.
#[poise::command(slash_command, category = "Worduel", rename = "wd_challenge", ephemeral)]
pub async fn challenge(
    ctx: Context<'_>,
    #[description = "Challenged user"] user: serenity::User,
    #[description = "Challenge word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    let game_id = ctx.data().challenge_player(
        ctx.author().id, user.id, word.clone(), GameVariant::Timed
    ).await?;

    let mplock = ctx.data().mpgames.read().await;
    let gamedata = mplock.get(&game_id).unwrap(); // why wouldn't it exist?

    ctx.say(format!("Created game with word: {}", word)).await?;

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
/// Same rules as for challenges.
#[poise::command(slash_command, category = "Worduel", rename = "wd_accept", ephemeral)]
pub async fn accept(
    ctx: Context<'_>,
    #[description = "Chosen challenger"] user: serenity::User,
    #[description = "Response word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    ctx.data().accept_invite(ctx.author().id, user.id, word.clone(), GameVariant::Timed).await?;

    ctx.say(format!("Responded to game with word: {}", word)).await?;

    ctx.channel_id().send_message(&ctx.discord().http, |m| {
        m.content(
            serenity::MessageBuilder::new()
                .push("Your challenge has been accepted by ")
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

/// Reject a Worduel invite
#[poise::command(slash_command, category = "Worduel", rename = "wd_reject", ephemeral)]
pub async fn reject(
    ctx: Context<'_>,
    #[description = "Challenger being rejected"] user: serenity::User,
) -> Result<(), Error> {
    let game = ctx.data().reject_invite(ctx.author().id, user.id, GameVariant::Timed).await?;

    ctx.say(match game {
        None => "Rejected invite, game void".to_string(),
        Some(g) => format!("Rejected invite, word was: {}", g.get_baseword(1))
    }).await?;

    ctx.channel_id().send_message(&ctx.discord().http, |m| {
        m.content(
            serenity::MessageBuilder::new()
                .push("Your challenge to ")
                .user(ctx.author().id)
                .push(" has been rejected.")
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
pub async fn send(
    ctx: Context<'_>,
    #[description = "Sent word"] word: String,
) -> Result<(), Error> {
    let word = queries::ensure_word(&ctx.data().dict, &word)?;

    let own_id = ctx.author().id;

    let (stateline, content, views) = 
        ctx.data().act_on_timed(own_id, |_ud, _gid, gamedata, remove| {
            use multiplayer::GameProgress::*;

            if matches!(gamedata.get_progress(), multiplayer::GameProgress::Waiting) {
                return Err(CmdError::GameStarted(false).into());
            }
            let player_index = gamedata.match_user(own_id).unwrap();
            let enemy_id = gamedata.get_user_id(1-player_index);

            let success = gamedata.send_guess(player_index, word.to_lowercase());
            let progress = gamedata.get_progress().clone();

            let views = gamedata.render_views(constants::WORDUEL_VIEWSEP);

            let mut content = serenity::MessageBuilder::new();
            match progress {
                Over(res) => {
                    remove();
                    match res {
                        Some(i) => content
                            .push("Game over, ")
                            .user(enemy_id)
                            .push(", the victor is ")
                            .user(gamedata.get_user_id(i))
                            .push("!"),
                        None => content
                            .push("Game over, ")
                            .user(enemy_id)
                            .push(", this duel ended in a draw."),
                    }
                },
                _ => {
                    if success {
                        content.push("Word has been sent!")
                    } else {
                        content.push("Word rejected, wait for the other side to finish.")
                    }
                }
            };
            Ok((
                gamedata.render_stateline(true),
                content.build(),
                views,
            ))
        }).await?;

    ctx.send(|m| {
        m.content(content).embed(|e| {
            e.title("Worduel status")
                .field("Game state", stateline, true)
                .color((255, 204, 11))
                .description(views)
        }).ephemeral(false)
    })
    .await?;
    Ok(())
}

/// Forfeit from a Worduel
///
/// To make sure you don't accidentally forfeit,
/// you have to specify the enemy's tag in the command invocation.
#[poise::command(slash_command, category = "Worduel", rename = "wd_forfeit", ephemeral)]
pub async fn forfeit(
    ctx: Context<'_>,
    #[description = "Enemy username"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let own_id = ctx.author().id;
    let user_unwrapped = user.ok_or(CmdError::ForfeitBadUser)?;

    let (stateline, content, views) = ctx.data().act_on_timed(own_id, |_, _, gamedata, remove| {
        let player_index = gamedata.match_user(own_id).unwrap();
        let enemy_id = gamedata.get_user_id(1 - player_index);

        if user_unwrapped.id != enemy_id {
            return Err(CmdError::ForfeitBadUser.into());
        }

        use multiplayer::GameProgress::*;
        let views = gamedata.render_views(constants::WORDUEL_VIEWSEP);

        let mut content = serenity::MessageBuilder::new();
        match gamedata.get_progress() {
            Waiting => content
                .user(enemy_id)
                .push(", the game has been given up on by ")
                .push(&ctx.author().name),
            _ => content
                .user(enemy_id)
                .push(", your opponent has forfeited this game."),
        };
        remove();
        Ok((gamedata.render_stateline(false), content.build(), views))
    }).await?;

    ctx.send(|m| {
        m.content(content).embed(|e| {
            e.title("Worduel status before forfeit")
                .field("Last game state", stateline, true)
                .color((255, 204, 11))
                .description(views)
        }).ephemeral(false)
    })
    .await?;
    Ok(())
}

/// Show the letter usage in your current timed game.
///
/// This is a display-only keyboard, you can't use it for input.
#[poise::command(slash_command, category = "Worduel", rename = "wd_kb", ephemeral)]
pub async fn keyboard(ctx: Context<'_>) -> Result<(), Error> {
    let own_id = ctx.author().id;

    let keyboard = ctx.data().act_on_timed(own_id, |_, _, gamedata, _| {
        let player_index = gamedata.match_user(own_id).unwrap();
        Ok(gamedata.render_keyboard(player_index))
    }).await?;

    ctx.send(|m| m.content(keyboard)).await?;
    Ok(())
}
