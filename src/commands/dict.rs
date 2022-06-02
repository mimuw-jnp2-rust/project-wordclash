use crate::{Context, Error};
use crate::dict::wordmatch;

/// Look up a word in the dictionary
#[poise::command(prefix_command, slash_command, hide_in_help, category = "Dictionary")]
pub async fn lookup(
    ctx: Context<'_>,
    #[description="Word to look up in dictionary"] word: String
) -> Result<(), Error> {
    if ctx.data().dict.contains(&word) {
        ctx.say("Found in the dictionary").await?;
    } else {
        ctx.say("Not found in the dictionary").await?;
    }

    Ok(())
}

/// See how well a word matches another word
/// 
/// For testing purposes.
#[poise::command(prefix_command, slash_command, hide_in_help, category = "Dictionary")]
pub async fn testmatch(
    ctx: Context<'_>,
    #[description="Base word"] base: String,
    #[description="Word to test"] word: String
) -> Result<(), Error> {
    if base.len() != word.len() {
        ctx.say("Word lengths mismatched!").await?;
        return Ok(());
    }
    
    let wmatch = wordmatch::match_word(&base, &word);
    use wordmatch::MatchLetter;
    use poise::serenity::utils::MessageBuilder;
    ctx.send(|m| {
        m.content(MessageBuilder::new()
            .push_line("Match status:")
            .push_mono(wmatch
                .iter()
                .zip(word.chars())
                .map(|(m, c)| {
                    match m {
                        MatchLetter::Null  => format!(" {} ", c),
                        MatchLetter::Close => format!(":{}:", c),
                        MatchLetter::Exact => format!("[{}]", c),
                    }
                })
                .collect::<Vec<String>>()
                .join(" "))
            .build())
    }).await?;

    Ok(())
}