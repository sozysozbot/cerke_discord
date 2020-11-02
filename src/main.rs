#![warn(clippy::pedantic)]
#![allow(clippy::non_ascii_literal)]

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
#[commands(ping, log, initiate, mov, show, capture, stepup, stepdown, parachute)]
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

use cetkaik_core::absolute::{parse_coord, Side};
use cetkaik_core::{Color, Profession};
use render_cerke_board::{Field, OperationError};
use std::fs::File;

#[command]
fn show(ctx: &mut Context, msg: &Message) -> CommandResult {
    render_current(ctx, msg)
}

fn render_current(ctx: &mut Context, msg: &Message) -> CommandResult {
    let map = serde_json::json!({
        "content": "Loading...",
        "tts": false,
    });
    ctx.http.send_message(msg.channel_id.0, &map)?;

    let field = bot::FIELD.lock().unwrap();

    field.render(Side::IASide).save("iaside.png").unwrap();
    field.render(Side::ASide).save("aside.png").unwrap();

    let side_ia = File::open("./iaside.png")?;
    let side_a_ = File::open("./aside.png")?;

    ctx.http.send_files(
        msg.channel_id.0,
        vec![(&side_ia, "iaside.png"), (&side_a_, "aside.png")],
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

use serenity::framework::standard::CommandError;

fn report_error<T>(ctx: &mut Context, msg: &Message, report: &str) -> Result<T, CommandError> {
    msg.channel_id.say(&ctx.http, report)?;
    Err(CommandError(report.to_string()))
}

fn if_none_report_error<T>(
    ctx: &mut Context,
    msg: &Message,
    a: Option<T>,
    report: &str,
) -> Result<T, CommandError> {
    match a {
        None => report_error(ctx, msg, report),
        Some(k) => Ok(k),
    }
}

fn expect_at_least_how_many(
    ctx: &mut Context,
    msg: &Message,
    howmany_expected: usize,
) -> Result<Vec<String>, CommandError> {
    use boolinator::Boolinator;

    let input: Vec<String> = msg
        .content
        .split_whitespace()
        .map(std::string::ToString::to_string)
        .collect();
    if_none_report_error(
        ctx,
        msg,
        (input.len() > howmany_expected).as_some(()),
        &format!(
            "Not enough arguments. Expected: {}, got: {}",
            howmany_expected,
            input.len() - 1
        ),
    )?;

    Ok(input)
}

#[command]
fn parachute(ctx: &mut Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 1)?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )?;

    let mut opt_prof = None;
    let mut opt_color = None;
    let mut opt_side = None;

    for s in input.iter().skip(2) {
        if let Ok(p) = s.parse::<Profession>() {
            if let Some(old_p) = opt_prof {
                if p != old_p {
                    report_error(
                        ctx,
                        msg,
                        &format!("conflicting profession info: {:?} and {:?}", old_p, p),
                    )?;
                }
            } else {
                opt_prof = Some(p);
            }
        } else if let Ok(c) = s.parse::<Color>() {
            if let Some(old_c) = opt_color {
                if c != old_c {
                    report_error(
                        ctx,
                        msg,
                        &format!("conflicting color info: {:?} and {:?}", old_c, c),
                    )?;
                }
            } else {
                opt_color = Some(c);
            }
        } else if let Ok(si) = s.parse::<Side>() {
            if let Some(old_si) = opt_side {
                if si != old_si {
                    report_error(
                        ctx,
                        msg,
                        &format!("conflicting side info: {:?} and {:?}", old_si, si),
                    )?;
                }
            } else {
                opt_side = Some(si);
            }
        } else {
            report_error(ctx, msg, &format!("unrecognizable option: {}", s))?;
        }
    }

    {
        let mut field = bot::FIELD.lock().unwrap();
        let lf = field.to_logical();

        let (side, color, profession) = {
            if lf.f.ia_side_hop1zuo1.is_empty() && lf.f.a_side_hop1zuo1.is_empty() {
                report_error(ctx, msg, "No piece found in either sides' hop1zuo1")?;
            }
            if let (Some(s), Some(c), Some(p)) = (opt_side, opt_color, opt_prof) {
                // If all filled, trust them
                (s, c, p)
            } else if lf.f.a_side_hop1zuo1.is_empty() || opt_side == Some(Side::IASide) {
                // must be ia_side
                if lf.f.ia_side_hop1zuo1.is_empty() {
                    report_error(ctx, msg, "No piece found in IASides' hop1zuo1")?;
                }

                let candidates: Vec<_> = lf
                    .f.ia_side_hop1zuo1
                    .iter()
                    .filter(|pi| matcher(pi.color, opt_color) && matcher(pi.prof, opt_prof))
                    .collect();

                let (c, p) = match &candidates[..] {
                    [] => report_error(
                        ctx,
                        msg,
                        "No piece in IASide's hop1zuo1 matches the description",
                    )?,
                    [pi] => (pi.color, pi.prof),
                    [pi, ..] => {
                        if is_all_same(&candidates) {
                            (pi.color, pi.prof)
                        } else {
                            report_error(ctx, msg, "Not enough info to identify the piece. Add color/profession and try again")?
                        }
                    }
                };

                (Side::IASide, c, p)
            } else if lf.f.ia_side_hop1zuo1.is_empty() || opt_side == Some(Side::ASide) {
                // must be a_side
                if lf.f.a_side_hop1zuo1.is_empty() {
                    report_error(ctx, msg, "No piece found in ASides' hop1zuo1")?;
                }

                let candidates: Vec<_> = lf
                    .f.a_side_hop1zuo1
                    .iter()
                    .filter(|pi| matcher(pi.color, opt_color) && matcher(pi.prof, opt_prof))
                    .collect();

                let (c, p) = match &candidates[..] {
                    [] => report_error(
                        ctx,
                        msg,
                        "No piece in ASide's hop1zuo1 matches the description",
                    )?,
                    [pi] => (pi.color, pi.prof),
                    [pi, ..] => {
                        if is_all_same(&candidates) {
                            (pi.color, pi.prof)
                        } else {
                            report_error(ctx, msg, "Not enough info to identify the piece. Add color/profession and try again")?
                        }
                    }
                };

                (Side::ASide, c, p)
            } else {
                // Neither is empty. Gotta search from both.

                let mut candidates1: Vec<_> = lf
                    .f.a_side_hop1zuo1
                    .iter()
                    .filter_map(|pi| {
                        if matcher(pi.color, opt_color) && matcher(pi.prof, opt_prof) {
                            Some((Side::ASide, pi))
                        } else {
                            None
                        }
                    })
                    .collect();

                let candidates2: Vec<_> = lf
                    .f.ia_side_hop1zuo1
                    .iter()
                    .filter_map(|pi| {
                        if matcher(pi.color, opt_color) && matcher(pi.prof, opt_prof) {
                            Some((Side::IASide, pi))
                        } else {
                            None
                        }
                    })
                    .collect();

                candidates1.extend(candidates2);

                match &candidates1[..] {
                    [] => report_error(ctx, msg, "No piece in hop1zuo1 matches the description")?,
                    [(s, pi)] => (*s, pi.color, pi.prof),
                    [(s, pi), ..] => {
                        if is_all_same(&candidates1) {
                            (*s, pi.color, pi.prof)
                        } else {
                            report_error(ctx, msg, "Not enough info to identify the piece. Add side/color/profession and try again")?
                        }
                    }
                }
            }
        };

        println!(
            "parachute: dst {:?}, side: {:?}, color: {:?}, prof: {:?}",
            dst, side, color, profession
        );
        scold_operation_error(ctx, msg, field.from_hop1zuo1(dst, side, color, profession))?;
    }

    render_current(ctx, msg)
}

fn is_all_same<T: PartialEq>(arr: &[T]) -> bool {
    arr.windows(2).all(|w| w[0] == w[1])
}

fn matcher<T: Eq + Copy>(a: T, b: Option<T>) -> bool {
    match b {
        None => true,
        Some(x) => x == a,
    }
}

#[command]
fn stepdown(ctx: &mut Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 1)?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )?;

    println!("stepping down and reaching the dst {:?}", dst);

    {
        let mut field = bot::FIELD.lock().unwrap();
        scold_operation_error(ctx, msg, field.descend_from_stepping(dst))?;
    }

    render_current(ctx, msg)
}

#[command]
fn stepup(ctx: &mut Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 2)?;
    let src = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[2]),
        &format!(
            "The second argument is incorrect. Expected a coordinate, got: {}",
            input[2]
        ),
    )?;

    println!(
        "moving from the src {:?} and stepping the dst {:?}",
        src, dst
    );

    {
        let mut field = bot::FIELD.lock().unwrap();
        scold_operation_error(ctx, msg, field.step_on_occupied(dst, src))?;
    }

    render_current(ctx, msg)
}

#[command]
fn capture(ctx: &mut Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 1)?;
    let src = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )?;

    println!("capturing; src: {:?}", src);

    {
        let mut field = bot::FIELD.lock().unwrap();
        scold_operation_error(ctx, msg, field.move_to_opponent_hop1zuo1(src))?;
    }

    render_current(ctx, msg)
}

#[command]
fn mov(ctx: &mut Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 2)?;
    let src = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[2]),
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
            Err(CommandError(report))
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
