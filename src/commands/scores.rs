use crate::{Context, Error};
use super::util::*;
use poise::serenity_prelude as serenity;

/// List up to top n scores in the leaderboard
/// n maximum 50, minimum 1, default 10.
#[poise::command(slash_command, category = "Worduel", rename = "wd_top", ephemeral)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Result size"] count: Option<usize>,
) -> Result<(), Error> {
    let count = count.unwrap_or(10);
    if count < 1 || count > 50 {
        return Err(CmdError::Misc("You cannot list this many top players".to_string()).into());
    }

    let scores = ctx.data().scores().list_top(count).await;
    let mut result = serenity::MessageBuilder::new();
    for i in 0..scores.len() {
        let (user, score) = &scores[i];
        result.push(i+1).push(". ");
        result.user(user);
        result.push(": ").push(score).push(" pts\n");
    }
    if scores.len() == 0 {
        result.push("No one has won a game yet");
    }
    ctx.say(result.build()).await?;
    Ok(())
}