extern crate serenity;

use std::env;

use serenity::prelude::*;
use serenity::async_trait;
use serenity::model::gateway::Ready;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }
}

fn main() {

}