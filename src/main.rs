#![warn(clippy::pedantic)]
#![allow(clippy::non_ascii_literal)]

#[macro_use]
extern crate lazy_static;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use std::env;

pub mod bot;

#[group]
#[commands(ping, log, initiate, mov, show, capture, stepup, stepdown, parachute)]
struct General;

struct Handler;
#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
                .group(&GENERAL_GROUP),
        )
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn log(ctx: &Context, msg: &Message) -> CommandResult {
    let content = {
        let log = bot::LOG.lock().unwrap();
        log.join("\n")
    };
    msg.reply(ctx, content).await?;
    Ok(())
}

use cetkaik_core::absolute::{parse_coord, Side};
use cetkaik_core::{Color, Profession};
use render_cerke_board::{Field, OperationError};

#[command]
async fn show(ctx: &Context, msg: &Message) -> CommandResult {
    render_current(ctx, msg).await
}

async fn render_current(ctx: &Context, msg: &Message) -> CommandResult {
    let map = serde_json::json!({
        "content": "Loading...",
        "tts": false,
    });
    ctx.http.send_message(msg.channel_id.0, &map).await?;
    {
        let field = bot::FIELD.lock().unwrap();

        field.render(Side::IASide).save("iaside.png").unwrap();
        field.render(Side::ASide).save("aside.png").unwrap();
    }
    ctx.http
        .send_files(
            msg.channel_id.0,
            vec!["./iaside.png", "./aside.png"],
            serde_json::Map::new(),
        )
        .await?;
    Ok(())
}

#[command]
async fn initiate(ctx: &Context, msg: &Message) -> CommandResult {
    {
        let mut field = bot::FIELD.lock().unwrap();
        *field = Field::new();
    }
    render_current(ctx, msg).await
}

use serenity::framework::standard::CommandError;

async fn report_error<T>(ctx: &Context, msg: &Message, report: &str) -> Result<T, CommandError> {
    msg.channel_id.say(&ctx.http, report).await?;
    Err(report.to_string().into())
}

async fn if_none_report_error<T>(
    ctx: &Context,
    msg: &Message,
    a: Option<T>,
    report: &str,
) -> Result<T, CommandError> {
    match a {
        None => report_error(ctx, msg, report).await,
        Some(k) => Ok(k),
    }
}

async fn expect_at_least_how_many(
    ctx: &Context,
    msg: &Message,
    how_many_expected: usize,
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
        (input.len() > how_many_expected).as_some(()),
        &format!(
            "Not enough arguments. Expected: {}, got: {}",
            how_many_expected,
            input.len() - 1
        ),
    )
    .await?;

    Ok(input)
}

fn get_scp(
    opt_side: Option<Side>,
    opt_color: Option<Color>,
    opt_prof: Option<Profession>,
    lf: &render_cerke_board::LogicalField,
) -> Result<(Side, Color, Profession), &'static str> {
    if lf.f.ia_side_hop1zuo1.is_empty() && lf.f.a_side_hop1zuo1.is_empty() {
        return Err("No piece found in either sides' hop1zuo1");
    }
    if let (Some(s), Some(c), Some(p)) = (opt_side, opt_color, opt_prof) {
        // If all filled, trust them
        return Ok((s, c, p));
    } else if lf.f.a_side_hop1zuo1.is_empty() || opt_side == Some(Side::IASide) {
        // must be ia_side
        if lf.f.ia_side_hop1zuo1.is_empty() {
            return Err("No piece found in IASides' hop1zuo1");
        }

        let candidates: Vec<_> =
            lf.f.ia_side_hop1zuo1
                .iter()
                .filter(|pi| matcher(pi.color, opt_color) && matcher(pi.prof, opt_prof))
                .collect();

        let (c, p) = match &candidates[..] {
            [] => return Err("No piece in IASide's hop1zuo1 matches the description"),
            [pi] => (pi.color, pi.prof),
            [pi, ..] => {
                if is_all_same(&candidates) {
                    (pi.color, pi.prof)
                } else {
                    return Err(
                        "Not enough info to identify the piece. Add color/profession and try again",
                    );
                }
            }
        };

        return Ok((Side::IASide, c, p));
    } else if lf.f.ia_side_hop1zuo1.is_empty() || opt_side == Some(Side::ASide) {
        // must be a_side
        if lf.f.a_side_hop1zuo1.is_empty() {
            return Err("No piece found in ASides' hop1zuo1");
        }

        let candidates: Vec<_> =
            lf.f.a_side_hop1zuo1
                .iter()
                .filter(|pi| matcher(pi.color, opt_color) && matcher(pi.prof, opt_prof))
                .collect();

        let (c, p) = match &candidates[..] {
            [] => return Err("No piece in ASide's hop1zuo1 matches the description"),
            [pi] => (pi.color, pi.prof),
            [pi, ..] => {
                if is_all_same(&candidates) {
                    (pi.color, pi.prof)
                } else {
                    return Err(
                        "Not enough info to identify the piece. Add color/profession and try again",
                    );
                }
            }
        };

        return Ok((Side::ASide, c, p));
    } else {
        // Neither is empty. Gotta search from both.

        let mut candidates1: Vec<_> =
            lf.f.a_side_hop1zuo1
                .iter()
                .filter_map(|pi| {
                    if matcher(pi.color, opt_color) && matcher(pi.prof, opt_prof) {
                        Some((Side::ASide, pi))
                    } else {
                        None
                    }
                })
                .collect();

        let candidates2: Vec<_> =
            lf.f.ia_side_hop1zuo1
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

        return match &candidates1[..] {
            [] => Err("No piece in hop1zuo1 matches the description"),
            [(s, pi)] => Ok((*s, pi.color, pi.prof)),
            [(s, pi), ..] => {
                if is_all_same(&candidates1) {
                    Ok((*s, pi.color, pi.prof))
                } else {
                    Err("Not enough info to identify the piece. Add side/color/profession and try again")
                }
            }
        }
    }
}

#[command]
async fn parachute(ctx: &Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 1).await?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )
    .await?;

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
                    )
                    .await?;
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
                    )
                    .await?;
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
                    )
                    .await?;
                }
            } else {
                opt_side = Some(si);
            }
        } else {
            report_error(ctx, msg, &format!("unrecognizable option: {}", s)).await?;
        }
    }

    let opt_modified_field = {
        let mut field = bot::FIELD.lock().unwrap();
        let lf = field.to_logical();

        match get_scp(opt_side, opt_color, opt_prof, &lf) {
            Err(e) => Err(e),
            Ok((side, color, profession)) => {
                println!(
                    "parachute: dst {:?}, side: {:?}, color: {:?}, prof: {:?}",
                    dst, side, color, profession
                );
                Ok(field.from_hop1zuo1(dst, side, color, profession))
            }
        }
    };

    match opt_modified_field {
        Err(e) => report_error(ctx, msg, e).await?,
        Ok(modified_field) => {
            scold_operation_error(ctx, msg, modified_field).await?;
        }
    };

    render_current(ctx, msg).await
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
async fn stepdown(ctx: &Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 1).await?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )
    .await?;

    println!("stepping down and reaching the dst {:?}", dst);

    let modified_field = {
        let mut field = bot::FIELD.lock().unwrap();
        field.descend_from_stepping(dst)
    };
    scold_operation_error(ctx, msg, modified_field).await?;

    render_current(ctx, msg).await
}

#[command]
async fn stepup(ctx: &Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 2).await?;
    let src = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )
    .await?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[2]),
        &format!(
            "The second argument is incorrect. Expected a coordinate, got: {}",
            input[2]
        ),
    )
    .await?;

    println!(
        "moving from the src {:?} and stepping the dst {:?}",
        src, dst
    );

    let modified_field = {
        let mut field = bot::FIELD.lock().unwrap();
        field.step_on_occupied(dst, src)
    };
    scold_operation_error(ctx, msg, modified_field).await?;

    render_current(ctx, msg).await
}

#[command]
async fn capture(ctx: &Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 1).await?;
    let src = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )
    .await?;

    println!("capturing; src: {:?}", src);

    let modified_field = {
        let mut field = bot::FIELD.lock().unwrap();
        field.move_to_opponent_hop1zuo1(src)
    };

    scold_operation_error(ctx, msg, modified_field).await?;
    render_current(ctx, msg).await
}

#[command]
async fn mov(ctx: &Context, msg: &Message) -> CommandResult {
    let input = expect_at_least_how_many(ctx, msg, 2).await?;
    let src = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[1]),
        &format!(
            "The first argument is incorrect. Expected a coordinate, got: {}",
            input[1]
        ),
    )
    .await?;

    let dst = if_none_report_error(
        ctx,
        msg,
        parse_coord(&input[2]),
        &format!(
            "The second argument is incorrect. Expected a coordinate, got: {}",
            input[2]
        ),
    )
    .await?;

    println!("moving; src: {:?},  dst: {:?}", src, dst);

    let modified_field = {
        let mut field = bot::FIELD.lock().unwrap();
        field.move_to_empty_square(dst, src)
    };
    scold_operation_error(ctx, msg, modified_field).await?;

    render_current(ctx, msg).await
}

async fn scold_operation_error(
    ctx: &Context,
    msg: &Message,
    a: Result<(), OperationError>,
) -> Result<(), CommandError> {
    match a {
        Err(x) => {
            let report = format!("Invalid move. Reason: {:?}", x);
            msg.channel_id.say(&ctx.http, &report).await?;
            Err(report.into())
        }
        Ok(()) => Ok(()),
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let content = format!("Pong! {}", msg.content);
    use serenity::model::misc::Mentionable;
    let mut gen = msg.author.mention();
    gen.push_str(": ");
    gen.push_str(&content);

    let map = serde_json::json!({
        "content": gen,
        "tts": false,
    });

    ctx.http.send_message(msg.channel_id.0, &map).await?;
    ctx.http
        .send_files(msg.channel_id.0, vec!["./icon.png"], serde_json::Map::new())
        .await?;

    msg.reply(ctx, content).await?;

    let mut log = bot::LOG.lock().unwrap();
    log.push(msg.content.to_string());

    Ok(())
}
