use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use rand::prelude::*;

/// Roll dice
///
/// Allows specifying die size and die count.
#[poise::command(slash_command, rename = "roll", category = "Miscellaneous")]
pub async fn roll_dice(
    ctx: Context<'_>,
    #[description = "Die size"] die_size: Option<u32>,
    #[description = "Die count"] die_count: Option<u16>,
) -> Result<(), Error> {
    let die_size = die_size.unwrap_or(6);
    if die_size <= 1 {
        ctx.say(format!(
            "**Error:** {} is not a valid size of dice.",
            die_size
        ))
        .await?;
        return Ok(());
    }
    let die_count = die_count.unwrap_or(1);
    ctx.say(format!("Rolling {}d{}...", die_count, die_size))
        .await?;

    let roll: u64 = {
        let mut rng = thread_rng();
        let uniform = rand::distributions::Uniform::new_inclusive(1, die_size);
        (0..die_count)
            .map(|_| uniform.sample(&mut rng) as u64)
            .sum()
    };

    let mut output = serenity::MessageBuilder::new();
    output.push("Result: ").push_mono(roll);
    if die_count == 1 && die_size == 20 && roll == 20 {
        output.push("\nDon't let it go to your head.");
    }
    ctx.say(output.build()).await?;

    Ok(())
}
