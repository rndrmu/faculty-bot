#[macro_use]
extern crate diesel;

mod database;

use diesel::pg::PgConnection;
use diesel::r2d2;
use std::{collections::HashSet, env, sync::Arc, str};
use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardManager},
    client::Context,
    framework::{
        standard::{macros::hook, DispatchError},
        StandardFramework,
    },
    http::Http,
    model::{
        channel::Message, event::ResumedEvent, gateway::Activity, gateway::Ready, id::UserId,
        interactions::Interaction,
    },
    prelude::*,
};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use database::establish_connection;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}



struct Handler;
struct RawHandler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        println!("Resumed");
    }
}


#[tokio::main]
async fn main() {
    println!("Use Intents: {:?}", GatewayIntents::all().bits);
    dotenv::dotenv().expect("Failed to load .env file");
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    let db = establish_connection();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let http = Http::new_with_token(&token);

    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // We will fetch your bot's owners and id
    let framework = StandardFramework::new()
        .configure(|c| {
            c.owners(owners)
                .prefix(env::var("PREFIX").unwrap_or(String::from("&")).as_str())
                .allow_dm(false)
        });

    let mut client = Client::builder(&token)
        .application_id(u64::from(bot_id))
        .framework(framework)
        .event_handler(Handler)
        .intents(GatewayIntents::all())
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    // start tokio-tungstenite server

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Error registering CTRL + C Handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    println!("[INFO] Starting Discord Bot");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
