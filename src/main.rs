use dotenv::dotenv;
use log::error;
use once_cell::sync::OnceCell;
use rust_fuzzy_search::fuzzy_search_sorted;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use simplelog::*;

use std::collections::HashMap;
use std::env;
use std::fs::OpenOptions;

mod definition;
mod scraper;

static DEF_HASHMAP: OnceCell<HashMap<String, String>> = OnceCell::new();
static DEF_STRLIST: OnceCell<Vec<&str>> = OnceCell::new();

#[group]
#[commands(explain)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    // load environment variables from .env
    dotenv().expect("Failed to invoke dotenv()");

    // initialise a file logger
    WriteLogger::init(
        LevelFilter::Warn,
        Config::default(),
        OpenOptions::new()
            .write(true)
            .create(true)
            .open("riinguist.log")
            .expect("Failed to open a file"),
    )
    .expect("Failed to initialize a logger");

    // initialise definition globals
    DEF_HASHMAP.set(scraper::get_definitions().await).ok();
    DEF_STRLIST
        .set(DEF_HASHMAP.get().unwrap().keys().map(|x| &**x).collect())
        .ok();

    // basic bot framework
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&GENERAL_GROUP);

    // login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not found");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Failed to create a client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("{:?}", why);
    }
}

#[command]
async fn explain(ctx: &Context, msg: &Message) -> CommandResult {
    // anything that follows the !explain command
    let arg = msg.content.replace("!explain", "").trim().to_owned();

    if arg.is_empty() {
        // nothing to explain
        msg.reply(
            ctx,
            "Please provide a Riichi Mahjong term for me to explain.",
        )
        .await?;
    } else {
        // fuzzy search for the best fit
        let mut result_lst: Vec<(f32, &str)> =
            fuzzy_search_sorted(&arg, &DEF_STRLIST.get().unwrap()[..])
                .into_iter()
                .filter(|x| x.1 >= 0.65)
                .map(|(a, b)| (b, a))
                .collect();

        // sort the list in a descending order, best match & shortest word first
        result_lst.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));

        let res = if !result_lst.is_empty() {
            DEF_HASHMAP
                .get()
                .unwrap()
                .get(result_lst.first().unwrap().1)
                .unwrap()
        } else {
            "No such term was found."
        };

        // echo the result (or lack thereof)
        msg.reply(ctx, res).await?;
    }

    Ok(())
}
