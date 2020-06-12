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
#[commands(ping, log, initiate, mov, show)]
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

use render_cerke_board::*;
use std::fs::File;

#[command]
fn show(ctx: &mut Context, msg: &Message) -> CommandResult {
    render_current(ctx, msg)
}

fn render_current(ctx:  &mut Context, msg: &Message) -> CommandResult {
    let map = serde_json::json!({
        "content": "Loading...",
        "tts": false,
    });
    ctx.http.send_message(msg.channel_id.0, &map)?;

    let field = bot::FIELD.lock().unwrap();

    field.render(Side::IASide).save("iaside.png").unwrap();
    field.render(Side::ASide).save("aside.png").unwrap();

    let iaside = File::open("./iaside.png")?;
    let aside = File::open("./aside.png")?;

    ctx.http.send_files(
        msg.channel_id.0,
        vec![(&iaside, "iaside.png"), (&aside, "aside.png")],
        serde_json::Map::new(),
    )?;
    Ok(())
}

#[command]
fn initiate(ctx: &mut Context, msg: &Message) -> CommandResult {
    {
        let mut field = bot::FIELD.lock().unwrap();
        *field = Field::new();
    }
    render_current(ctx, msg)
}

fn parse_coord(coord: &str) -> Option<(Row, Column)> {
    if coord.is_empty() || coord.len() > 3 {
        return None;
    }

    let column = match coord.chars().next() {
        None => None, // early return
        Some('C') => Some(Column::C),
        Some('K') => Some(Column::K),
        Some('L') => Some(Column::L),
        Some('M') => Some(Column::M),
        Some('N') => Some(Column::N),
        Some('P') => Some(Column::P),
        Some('T') => Some(Column::T),
        Some('X') => Some(Column::X),
        Some('Z') => Some(Column::Z),
        Some(_) => None,
    }?;

    let row = match &coord[1..coord.len()] {
        "A" => Some(Row::A),
        "AI" => Some(Row::AI),
        "AU" => Some(Row::AU),
        "E" => Some(Row::E),
        "I" => Some(Row::I),
        "O" => Some(Row::O),
        "U" => Some(Row::U),
        "Y" => Some(Row::Y),
        "IA" => Some(Row::IA),
        _ => None,
    }?;

    Some((row, column))
}

use serenity::framework::standard::CommandError;

fn if_none_report_error<T>(
    ctx: &mut Context,
    msg: &Message,
    a: Option<T>,
    report: &str,
) -> Result<T, CommandError> {
    match a {
        None => {
            msg.channel_id.say(&ctx.http, report)?;
            return Err(CommandError(report.to_string()));
        }
        Some(k) => Ok(k),
    }
}

#[command]
fn mov(ctx: &mut Context, msg: &Message) -> CommandResult {
    let input: Vec<&str> = msg.content.split_whitespace().collect();
    use boolinator::Boolinator;
    if_none_report_error(
        ctx,
        msg,
        (input.len() >= 3).as_some(()),
        &format!(
            "Not enough arguments. Expected: 2, got: {}",
            input.len() - 1
        ),
    )?;

    let src = if_none_report_error(
        ctx,
        msg,
        parse_coord(input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(input[2]),
        &format!(
            "The second argument is incorrect. Expected a coordinate, got: {}",
            input[2]
        ),
    )?;

    println!("moving; src: {:?},  dst: {:?}", src, dst);

    {
        let mut field = bot::FIELD.lock().unwrap();
        scold_operation_error(ctx, msg, field.move_to_empty_square(dst, src))?;
    }
    
    render_current(ctx, msg)
}

fn scold_operation_error(
    ctx: &mut Context,
    msg: &Message,
    a: Result<(), OperationError>,
) -> Result<(), CommandError> {
    match a {
        Err(x) => {
            let report = format!("Invalid move. Reason: {:?}", x);
            msg.channel_id.say(&ctx.http, &report)?;
            Err(CommandError(report.to_string()))
        }
        Ok(()) => Ok(()),
    }
}

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    let content = format!("Pong! {}", msg.content);
    use serenity::model::misc::Mentionable;
    use std::fs::File;
    let mut gen = msg.author.mention();
    gen.push_str(": ");
    gen.push_str(&content);

    let map = serde_json::json!({
        "content": gen,
        "tts": false,
    });

    let file = File::open("./icon.png")?;

    ctx.http.send_message(msg.channel_id.0, &map)?;
    ctx.http.send_files(
        msg.channel_id.0,
        vec![(&file, "icon.png")],
        serde_json::Map::new(),
    )?;

    msg.reply(ctx, content)?;

    let mut log = bot::LOG.lock().unwrap();
    log.push(msg.content.to_string());

    Ok(())
}
