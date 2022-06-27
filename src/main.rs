use poise::serenity_prelude as serenity;
pub use data::*;

use std::env;
use std::sync::Arc;

mod commands;
mod constants;
mod dict;
mod game;
mod data;
// use serde::{Deserialize, Serialize};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Arc<CtxData>, Error>;

const TOKEN_VARNAME: &str = "DISCORD_TOKEN";

#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "\
This is an example bot made to showcase features of my custom Discord bot framework",
            show_context_menu_commands: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, hide_in_help)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token_errstr: String = format!("Missing token variable ({})", TOKEN_VARNAME);

    let options = poise::FrameworkOptions {
        commands: vec![
            help(),
            register(),
            commands::dict::lookup(),
            commands::dict::testmatch(),
            commands::timedgame::challenge(),
            commands::timedgame::accept(),
            commands::timedgame::send(),
            commands::timedgame::forfeit(),
            commands::timedgame::keyboard(),
            commands::turngame::challenge(),
            commands::turngame::accept(),
            commands::turngame::send(),
            commands::turngame::remind(),
            commands::turngame::forfeit(),
            commands::turngame::keyboard(),
            commands::misc::roll_dice(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            ..Default::default()
        },
        ..Default::default()
    };

    let ctxdata = Arc::new(CtxData::new(dict::load_dictionary()));
    let fwdata = ctxdata.clone();
    let framework = poise::Framework::build()
        .token(env::var(TOKEN_VARNAME).expect(&token_errstr))
        .user_data_setup(move |_ctx, _ready, _fw| {
            Box::pin(async move {
                Ok(fwdata)
            })
        })
        .options(options)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        );

    tokio::task::spawn(async move {
        let mut cleanup_timer = tokio::time::interval(constants::CLEANUP_INTERVAL);
        loop {
            use std::time::SystemTime;
            cleanup_timer.tick().await;
            let mut mplock = ctxdata.mpgames.write().await;
            { // Separate scope to hold udlock for less time
                let mut udlock = ctxdata.userdata.write().await;
                for v in udlock.values_mut() {
                    v.player.clean_invites_then(SystemTime::now(), |invite| {
                        mplock.remove(&invite.game);
                    });
                };
            }
            mplock.retain(|_, g| !g.is_expiring());
        }
    });
    framework.run().await.unwrap();
}
