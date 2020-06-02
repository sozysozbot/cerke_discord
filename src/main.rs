#[macro_use]
extern crate lazy_static;

use serenity::client::Client;
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler};

pub mod bot;

#[group]
#[commands(ping, log, initiate)]
struct General;

use std::env;

struct Handler;

impl EventHandler for Handler {}

fn main() {
    // Login with a bot token from the environment
    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), Handler)
        .expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
            .group(&GENERAL_GROUP),
    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
fn log(ctx: &mut Context, msg: &Message) -> CommandResult {
    let log = bot::LOG.lock().unwrap();
    let content = log.join("\n");
    msg.reply(ctx, content)?;
    Ok(())
}

#[command]
fn initiate(ctx: &mut Context, msg: &Message) -> CommandResult {
    use std::fs::File;
    use render_cerke_board::*;


    let map = serde_json::json!({
        "content": "Loading...",
        "tts": false,
    });
    ctx.http.send_message(msg.channel_id.0, &map)?;

    let mut field = bot::FIELD.lock().unwrap();
    *field = Field::new();

    field.render(Side::IASide).save("iaside.png").unwrap();
    field.render(Side::ASide).save("aside.png").unwrap();

    let iaside = File::open("./iaside.png")?;
    let aside = File::open("./aside.png")?;

    ctx.http.send_files(msg.channel_id.0,vec![(&iaside, "iaside.png"), (&aside, "aside.png")], serde_json::Map::new())?;
    Ok(())
}

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    let content = format!("Pong! {}", msg.content);
    use serde_json;
    use std::fs::File;
    use serenity::model::misc::Mentionable;
    let mut gen = msg.author.mention();
    gen.push_str(": ");
    gen.push_str(&content);

    let map = serde_json::json!({
        "content": gen,
        "tts": false,
    });

    let file = File::open("./icon.png")?;

    ctx.http.send_message(msg.channel_id.0, &map)?;
    ctx.http.send_files(msg.channel_id.0,vec![(&file, "icon.png")], serde_json::Map::new())?;

    msg.reply(ctx, content)?;

    let mut log = bot::LOG.lock().unwrap();
    log.push(msg.content.to_string());

    Ok(())
}
