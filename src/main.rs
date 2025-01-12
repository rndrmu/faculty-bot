mod commands;
mod config;
mod eventhandler;
mod structs;
mod tasks;
mod utils;

use chrono::{DateTime, FixedOffset};
use influxdb2::models::{DataPoint, Query};
use influxdb2::{models::WriteDataPoint, Client};

use dashmap::DashMap;
use dotenv::dotenv;
use poise::{
    self,
    serenity_prelude::{self as serenity, GatewayIntents},
};
use sqlx::postgres::PgPoolOptions;
use structs::CodeEmailPair;
use tokio::stream;
use tracing_subscriber::prelude::*;
use utils::CurrentEmail;

pub mod prelude {
    use super::*;
    type GenericError = Box<dyn std::error::Error + Send + Sync>;

    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Error {
        /// Error from the Serenity library, usually Discord related errors
        Serenity(serenity::Error),
        /// Error returned from Postgres (Database)
        Database(sqlx::Error),
        /// Generic error 
        /// 
        /// This simply wraps a Boxed dyn Error and is used when you want to return an error that doesn't fit into any of the other categories
        /// 
        /// The User should not see this error, so please use WithMessage if you want to tell the user something
        Generic(GenericError),
        /// Error returned from IO (Subprocess/File) operations 
        IO(std::io::Error),
        /// Error returned from a network request 
        NetRequest(reqwest::Error),
        /// Error with a custom message (used when a command goes wrong or you want to tell the user something)
        WithMessage(String),
        /// Error from the sqlx migration (not user facing)
        Migration(sqlx::migrate::MigrateError),
        /// Error from the serde_json library (not user facing)
        Serde(serde_json::Error),
        /// Error when parsing goes wrong
        ParseIntError(std::num::ParseIntError),
        /// Rss error
        Rss(rss::Error),
        /// Regex error
        Regex(regex::Error),
        /// Idk bruh, don't ask me
        Unknown,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Serenity(e) => write!(f, "Discord error: {}", e),
                Error::Database(e) => write!(f, "Database error: {}", e),
                Error::Generic(e) => write!(f, "Generic error: {}", e),
                Error::IO(e) => write!(f, "IO error: {}", e),
                Error::NetRequest(e) => write!(f, "NetRequest error: {}", e),
                Error::WithMessage(e) => write!(f, "An error occured: {}", e),
                Error::Migration(e) => write!(f, "Migration error: {}", e),
                Error::Serde(e) => write!(f, "Deserialization error: {}", e),
                Error::ParseIntError(e) => write!(f, "ParseIntError: {}", e),
                Error::Rss(e) => write!(f, "Rss error: {}", e),
                Error::Regex(e) => write!(f, "Regex error: {}", e),
                _ => write!(
                    f,
                    "Unknown error occured, ask the developers for more information"
                ),
            }
        }
    }

    pub mod translations {
        rosetta_i18n::include_translations!();
    }
}

pub type Context<'a> = poise::Context<'a, Data, prelude::Error>;
pub type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, prelude::Error>;

#[derive(Clone)]
pub struct Data {
    pub db: sqlx::Pool<sqlx::Postgres>,
    pub config: config::FacultyManagerConfig,
    pub email_codes: DashMap<serenity::UserId, CodeEmailPair>,
    pub email_task: tokio::sync::mpsc::Sender<CurrentEmail>,
    pub influx: influxdb2::Client,
}

#[tokio::main]
async fn main() -> Result<(), prelude::Error> {
    // only load .env file if it exists
    if std::path::Path::new(".env").exists() {
        dotenv().ok();
    }

    // read config.json
    let config = config::read_config().expect("Failed to read config file");

    // print mealplan post day and time
    println!(
        "Mealplan will be posted on {:?} at {:?}",
        config.mealplan.post_on_day, config.mealplan.post_at_hour
    );

    // setup tracing

    // de-noise tracing by readin the RUST_LOG env var
    let tracing_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(tracing_layer))
        .init();

    tracing::info!("Starting up");

    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let db_url = std::env::var("DATABASE_URL").expect("Expected a database url in the environment");

    let pool = PgPoolOptions::new()
        .max_connections(15)
        .connect(&db_url)
        .await
        .map_err(prelude::Error::Database)?;

    let influx_host = "https://us-east-1-1.aws.cloud2.influxdata.com";
    let influx_org = "faculty_manager";
    let influx_bucket = "faculty_manager";
    let auth_token = std::env::var("INFLUX_TOKEN").expect("Expected a token in the environment");

    let influx_client = influxdb2::Client::new(influx_host, influx_org, auth_token);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<CurrentEmail>(100);

    let _ = tokio::spawn(async move {
        tracing::info!("Starting email task");
        loop {
            if let Some(email) = rx.recv().await {
                if let Err(y) = email.send().await {
                    tracing::error!("Failed to send email: {}", y); // we should probably let the user know that the email failed to send here
                }
            } else {
                tracing::info!("Email queue is empty - waiting for new emails");
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                register(),
                commands::user::verify(),
                commands::user::leaderboard(),
                commands::user::xp(),
                commands::administration::getmail(),
                commands::administration::run_command(),
                commands::administration::set_xp(),
                commands::administration::force_post_mensaplan(),
                commands::administration::rule_command(),
                commands::moderation::pin(),
                commands::moderation::delete_message(),
                commands::moderation::promote_user(),
                commands::moderation::demote_user(),
                commands::help(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(config.prefix.clone()),
                mention_as_prefix: true,
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(
                    async move { eventhandler::event_listener(ctx, event, &framework, data).await },
                )
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                if let Ok(_) =
                    poise::builtins::register_globally(ctx, &framework.options().commands).await
                {
                    tracing::info!("Successfully registered Application Commands globally");
                } else {
                    tracing::error!("Failed to register commands globally");
                }
                Ok(Data {
                    db: pool,
                    config,
                    email_codes: DashMap::new(),
                    email_task: tx,
                    influx: influx_client,
                })
            })
        })
        .token(token)
        .intents(GatewayIntents::all())
        .build()
        .await
        .map_err(prelude::Error::Serenity)?
        .start_autosharded()
        .await
        .map_err(prelude::Error::Serenity)?;

    Ok(())
}

#[poise::command(prefix_command, required_permissions = "MANAGE_GUILD")]
async fn register(ctx: Context<'_>) -> Result<(), prelude::Error> {
    poise::builtins::register_application_commands_buttons(ctx)
        .await
        .map_err(prelude::Error::Serenity)?;
    Ok(())
}