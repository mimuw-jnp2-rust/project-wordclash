use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use tokio::sync::RwLock as TokioRwLock;
use std::sync::RwLock;
use rand::prelude::*;
pub use data::*;

use std::env;

mod commands;
mod constants;
mod dict;
mod game;
mod data;
// use serde::{Deserialize, Serialize};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, CtxData, Error>;

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
            commands::mpgame::worduel_challenge(),
            commands::mpgame::worduel_accept(),
            commands::mpgame::worduel_send(),
            commands::mpgame::worduel_forfeit(),
            commands::misc::roll_dice(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            ..Default::default()
        },
        ..Default::default()
    };

    let framework = poise::Framework::build()
        .token(env::var(TOKEN_VARNAME).expect(&token_errstr))
        .user_data_setup(move |_ctx, _ready, _fw| {
            Box::pin(async move {
                Ok(CtxData::new(
                    dict::load_dictionary().await,
                ))
            })
        })
        .options(options)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        );

    framework.run().await.unwrap();
}
