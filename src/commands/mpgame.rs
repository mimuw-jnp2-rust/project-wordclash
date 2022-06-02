use crate::config;
use crate::game::*;
use crate::{Context, Error, UserData};
use poise::serenity_prelude as serenity;
use serenity::SerenityError::Other as AuxError;

/// Challenge an user to a Worduel
///
/// Supplied word must be within reasonable length bounds
/// and appear in the dictionary.
#[poise::command(slash_command, category = "Worduel")]
pub async fn worduel_challenge(
    ctx: Context<'_>,
    #[description = "Challenged user"] user: serenity::User,
    #[description = "Challenge word"] word: String,
) -> Result<(), Error> {
    if word.len() < config::MIN_WORDSIZE || word.len() > config::MAX_WORDSIZE {
        ctx.say(format!(
            "**Error:** Word length {} not allowed!",
            word.len()
        ))
        .await?;
        return Ok(());
    }
    if !ctx.data().dict.contains(&word) {
        ctx.say("**Error:** Word not in dictionary!").await?;
        return Ok(());
    }
    if user.eq(ctx.author()) {
        ctx.say(
            "**Error:** Cannot challenge yourself! Maybe there will be a singleplayer mode later.",
        )
        .await?;
        return Ok(());
    }

    let own_id = ctx.author().id;
    let other_id = user.id;
    let mut udlock = ctx.data().userdata.write().await;

    if let Some(userdata2) = udlock.get(&user.id) {
        match userdata2.player.game {
            ActiveGame::None => {}
            _ => {
                ctx.say("**Error:** This player is already in a game.")
                    .await?;
                return Ok(());
            }
        }
    }

    let mut mplock = ctx.data().mpgames.write().await;
    {
        // Own data scope
        let userdata1 = udlock.entry(own_id).or_insert_with(UserData::new);
        if !matches!(userdata1.player.game, ActiveGame::None) {
            ctx.say("**Error:** You're already in a game. Finish or forfeit if you want to start a new one.").await?;
            return Ok(());
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

    ctx.send(|m| {
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
#[poise::command(slash_command, category = "Worduel")]
pub async fn worduel_accept(
    ctx: Context<'_>,
    #[description = "Response word"] word: String,
) -> Result<(), Error> {
    if word.len() < config::MIN_WORDSIZE || word.len() > config::MAX_WORDSIZE {
        ctx.say(format!(
            "**Error:** Word length {} not allowed!",
            word.len()
        ))
        .await?;
        return Ok(());
    }
    if !ctx.data().dict.contains(&word) {
        ctx.say("**Error:** Word not in dictionary!").await?;
        return Ok(());
    }

    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;

    let (userdata, other_id) = if let Some(udata) = udlock.get_mut(&own_id) {
        match udata.player.game {
            ActiveGame::Multiplayer(oid) => (udata, oid),
            _ => {
                ctx.say("**Error:** No game to accept.").await?;
                return Ok(());
            }
        }
    } else {
        ctx.say("**Error:** No game to accept.").await?;
        return Ok(());
    };

    if other_id == own_id {
        ctx.say("**Error:** You can't accept your own challenge!")
            .await?;
        return Ok(());
    }
    let other_user = other_id.to_user(&ctx.discord().http).await?;

    let mut mplock = ctx.data().mpgames.write().await;
    let gamedata = match mplock.get_mut(&other_id) {
        Some(d) => d,
        None => {
            ctx.say("**Error:** Game assigned, but deleted.").await?;
            userdata.player.game = ActiveGame::None;
            return Ok(());
        }
    };
    if gamedata.get_word_length() != word.len() {
        ctx.say("**Error:** Word length does not match challenge word.")
            .await?;
        return Ok(());
    }
    if !gamedata.respond(word.to_lowercase()) {
        ctx.say("**Error:** Failed to accept. Do you actually have an invitation waiting?")
            .await?;
        return Ok(());
    }
    ctx.send(|m| {
        m.content(
            serenity::MessageBuilder::new()
                .push("Challenge accepted, ")
                .user(other_user)
                .push("!")
                .build(),
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
#[poise::command(slash_command, category = "Worduel")]
pub async fn worduel_send(
    ctx: Context<'_>,
    #[description = "Sent word"] word: String,
) -> Result<(), Error> {
    if word.len() < config::MIN_WORDSIZE || word.len() > config::MAX_WORDSIZE {
        ctx.say(format!(
            "**Error:** Word length {} not allowed!",
            word.len()
        ))
        .await?;
        return Ok(());
    }
    if !ctx.data().dict.contains(&word) {
        ctx.say("**Error:** Word not in dictionary!").await?;
        return Ok(());
    }

    let own_id = ctx.author().id;
    let mut udlock = ctx.data().userdata.write().await;
    let mut mplock = ctx.data().mpgames.write().await;

    let (progress, stateline, content, views, game_id, other_id) = {
        let (userdata, game_id) = if let Some(udata) = udlock.get_mut(&own_id) {
            match udata.player.game {
                ActiveGame::Multiplayer(oid) => (udata, oid),
                _ => {
                    ctx.say("**Error:** Not in a game.").await?;
                    return Ok(());
                }
            }
        } else {
            ctx.say("**Error:** Not in a game.").await?;
            return Ok(());
        };

        let gamedata = match mplock.get_mut(&game_id) {
            Some(d) => d,
            None => {
                ctx.say("**Error:** Game assigned, but deleted.").await?;
                userdata.player.game = ActiveGame::None;
                return Ok(());
            }
        };
        let player_index = gamedata.match_user(own_id).map(Ok).unwrap_or(Err(AuxError(
            "User ID bound to game, but not actually in game!",
        )))?;
        let enemy_id = gamedata.get_user_id(1 - player_index);

        use multiplayer::GameProgress::*;
        if matches!(gamedata.get_progress(), Waiting) {
            ctx.say("**Error:** Not in a game.").await?;
            return Ok(());
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
                    .user(id)
                    .push(", score: ")
                    .push(scores[i])
                    .push(':')
                    .push(scores[1 - i])
                    .push(')')
            }
        };
        let views = gamedata.render_views(config::WORDUEL_VIEWSEP);

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
        })
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
        None => {
            ctx.say("To prevent accidental forfeiture, mention your opponent in this command.")
                .await?;
            return Ok(());
        }
        Some(u) => u,
    };

    let (stateline, content, views, game_id, other_id) = {
        let (userdata, game_id) = if let Some(udata) = udlock.get_mut(&own_id) {
            match udata.player.game {
                ActiveGame::Multiplayer(oid) => (udata, oid),
                _ => {
                    ctx.say("**Error:** Not in a game.").await?;
                    return Ok(());
                }
            }
        } else {
            ctx.say("**Error:** Not in a game.").await?;
            return Ok(());
        };

        let gamedata = match mplock.get_mut(&game_id) {
            Some(d) => d,
            None => {
                ctx.say("**Error:** Game assigned, but deleted.").await?;
                userdata.player.game = ActiveGame::None;
                return Ok(());
            }
        };
        let player_index = gamedata.match_user(own_id).map(Ok).unwrap_or(Err(AuxError(
            "User ID bound to game, but not actually in game!",
        )))?;
        let enemy_id = gamedata.get_user_id(1 - player_index);

        if user_unwrapped.id != enemy_id {
            ctx.say("Specify your enemy's name specifically to forfeit a duel.")
                .await?;
            return Ok(());
        }

        use multiplayer::GameProgress::*;
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
            Over(_) => state.push("Game over"),
        };
        let views = gamedata.render_views(config::WORDUEL_VIEWSEP);

        let mut content = serenity::MessageBuilder::new();
        match progress {
            Waiting => content
                .user(enemy_id)
                .push(", your invitation has been rejected."),
            _ => content
                .user(enemy_id)
                .push(", your opponent has forfeited this game."),
        };
        (state.build(), content.build(), views, game_id, enemy_id)
    };

    mplock.remove(&game_id);
    if let Some(udata) = udlock.get_mut(&own_id) {
        udata.player.game = ActiveGame::None;
    }
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
