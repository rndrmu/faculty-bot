#![allow(unused_variables, unused_mut, dead_code)]

use std::sync::Arc;

use crate::{
    config::FacultyManagerMealplanConfig,
    prelude::Error,
    structs::{self},
    Data,
};
use chrono::{Datelike, Timelike};
use influxdb2::models::DataPoint;
use poise::serenity_prelude::{self as serenity, Mentionable, ShardId};
use rss::Channel;
use tracing::info;

struct TaskConfig {
    pub notify_role: serenity::RoleId,
    pub post_mealplan: bool,
    pub post_on_day: chrono::Weekday,
    pub post_at: chrono::NaiveTime,
    pub mealplan_settings: FacultyManagerMealplanConfig,
    pub post_channel: serenity::ChannelId,
}

struct TaskConfigRss {
    pub map: std::collections::HashMap<serenity::ChannelId, String>,
    pub clean_regex: regex::Regex,
    pub timeout_hrs: u64,
}

/// Posts the mensa plan for the current week
pub async fn post_mensaplan(ctx: serenity::Context, data: Data) -> Result<(), Error> {
    let task_conf = TaskConfig {
        notify_role: data.config.roles.mealplannotify,
        post_mealplan: data.config.mealplan.post_mealplan,
        post_on_day: data.config.mealplan.post_on_day,
        post_at: data.config.mealplan.post_at_hour,
        mealplan_settings: data.config.mealplan.clone(),
        post_channel: data.config.channels.mealplan,
    };

    loop {
        let now = chrono::Local::now();
        let weekday = now.weekday();
        let hour = now.hour();

        if weekday == task_conf.post_on_day && hour == task_conf.post_at.hour() {
            let mensa_plan = crate::utils::fetch_mensaplan(&task_conf.mealplan_settings.url)
                .await
                .unwrap();
            let today = now.date_naive().format("%Y-%m-%d").to_string();

            let mensaplan_posted = sqlx::query_as::<sqlx::Postgres, structs::Mensaplan>(
                "SELECT * FROM mensaplan WHERE date = $1",
            )
            .bind(&today)
            .fetch_optional(&data.db)
            .await
            .map_err(Error::Database)
            .unwrap()
            .map(|row| row.posted)
            .unwrap_or(false);

            if mensaplan_posted {
                info!("Mensaplan already posted today");
            } else {
                let mut channel = task_conf.post_channel;
                let mut msg = channel
                    .send_message(&ctx, |f| {
                        f.content(format!("{}", task_conf.notify_role.mention()))
                            .add_file(serenity::AttachmentType::Bytes {
                                data: std::borrow::Cow::Borrowed(&mensa_plan),
                                filename: "mensaplan.png".to_string(),
                            })
                            .components(|c| {
                                c.create_action_row(|r| {
                                    r.create_button(|b| {
                                        b.style(serenity::ButtonStyle::Primary)
                                            .label("Get Notified on new plans!")
                                            .emoji(serenity::ReactionType::Custom {
                                                animated: false,
                                                id: serenity::EmojiId(960491878048993300),
                                                name: Some("gulasch".to_string()),
                                            })
                                            .custom_id("mensaplan_notify_button")
                                    })
                                })
                            })
                    })
                    .await
                    .map_err(Error::Serenity);

                if let Ok(msg) = &mut msg {
                    if let Err(e) = msg.crosspost(&ctx).await.map_err(Error::Serenity) {
                        tracing::error!("Failed to crosspost mensaplan: {:?}", e);
                    }
                }

                let sql_res = sqlx::query("INSERT INTO mensaplan (date, posted) VALUES ($1, $2)")
                    .bind(&today)
                    .bind(true)
                    .execute(&data.db)
                    .await
                    .map_err(Error::Database);
            }
        } else {
            info!("Not posting mensaplan today");
        }

        info!("Sleeping for 5 minutes");
        tokio::time::sleep(tokio::time::Duration::from_secs(
            data.config.mealplan.check * 60,
        ))
        .await;
    }
}

pub async fn post_rss(ctx: serenity::Context, data: Data) -> Result<(), Error> {
    let conf = TaskConfigRss {
        map: data.config.rss_settings.rss_feed_data,
        clean_regex: regex::Regex::new(r"\\n(if wk med|all)").unwrap(),
        timeout_hrs: data.config.rss_settings.rss_check_interval_hours,
    };
    let db = data.db.clone();

    loop {
        for (channel_id, feed_url) in conf.map.iter() {
            let channel = fetch_feed(feed_url).await.unwrap();
            let items = channel.items();

            tracing::info!("Checking up on {} rss items", items.len());

            for item in items {
                let title = item.title().unwrap();
                let link = item.link().unwrap();
                let description = item.description().unwrap();
                let date_ = item.pub_date().unwrap();

                // parse to chrono Local
                let date = chrono::DateTime::parse_from_rfc2822(date_).unwrap();

                // to combat spam, filter out old items (all before July 11th 2024)
                if date
                    < chrono::DateTime::parse_from_rfc2822("Thu, 11 Jul 2024 00:00:00 +0200")
                        .unwrap()
                {
                    continue;
                } else {
                    tracing::info!("Found new rss item");
                }

                let sql_res = sqlx::query_as::<sqlx::Postgres, structs::Rss>(
                    "SELECT * FROM posted_rss WHERE rss_title = $1 AND channel_id = $2",
                )
                .bind(title)
                .bind(channel_id.0 as i64)
                .fetch_optional(&db)
                .await
                .map_err(Error::Database)
                .unwrap();

                if let Some(exists) = sql_res {
                    info!("An already posted rss item");
                    let curr_chan = channel_id;
                    let msg = curr_chan
                        .message(&ctx, exists.message_id as u64)
                        .await
                        .map_err(Error::Serenity)
                        .unwrap();
                    let embed = msg.embeds.first().unwrap();

                    let this_date = embed
                        .timestamp
                        .as_ref()
                        .unwrap()
                        .parse::<chrono::DateTime<chrono::Local>>()
                        .unwrap();
                    let item_date = date.with_timezone(&chrono::Local);

                    // compare dates and post update if newer
                    if this_date < item_date {
                        update_posts(
                            &ctx,
                            &db,
                            channel_id,
                            &conf,
                            title,
                            link,
                            description,
                            &item_date,
                            &msg,
                        )
                        .await?;
                    }
                } else {
                    // because let-else won't let me not return from this
                    // post
                    println!("Posting new rss item");
                    if let Err(why) = post_item(
                        &ctx,
                        &db,
                        channel_id,
                        &conf,
                        title,
                        link,
                        description,
                        &date.with_timezone(&chrono::Local),
                    )
                    .await
                    {
                        tracing::error!("Failed to post rss: {:?}", why);
                    }
                }

                tracing::info!("Posting in channel: {}", channel_id.0);
            }
        }

        info!("Sleeping for {} hours", conf.timeout_hrs);
        tokio::time::sleep(tokio::time::Duration::from_secs(conf.timeout_hrs * 60 * 60)).await;
    }
}

async fn fetch_feed(feed: impl Into<String>) -> Result<Channel, Error> {
    let bytestream = reqwest::get(feed.into())
        .await
        .map_err(Error::NetRequest)?
        .bytes()
        .await
        .map_err(Error::NetRequest)?;
    let channel = Channel::read_from(&bytestream[..]).map_err(Error::Rss)?;

    Ok(channel)
}

async fn update_posts(
    ctx: &serenity::Context,
    db: &sqlx::PgPool,
    channel_id: &serenity::model::id::ChannelId,
    conf: &TaskConfigRss,
    title: &str,
    link: &str,
    description: &str,
    date_: &chrono::DateTime<chrono::Local>,
    msg: &serenity::model::channel::Message,
) -> Result<(), Error> {
    let msg_result = channel_id
        .send_message(ctx, |f| {
            f.content(format!(
                "Der letzte Post im Planungsportal wurde aktualisiert · {}",
                title
            ))
            .embed(|e| {
                e.title(title)
                    .url(link)
                    .description(conf.clean_regex.replace_all(description, ""))
                    .timestamp(date_.to_rfc3339())
                    .color(0xb00b69)
            })
            .components(|c| {
                c.create_action_row(|a| {
                    a.create_button(|b| {
                        b.label("Open in Browser")
                            .style(serenity::ButtonStyle::Link)
                            .url(link)
                    })
                })
            })
            .reference_message(msg)
        })
        .await
        .map_err(Error::Serenity);

    if let Ok(msg) = msg_result {
        if let Err(why) = sqlx::query(
            "UPDATE posted_rss SET message_id = $1 WHERE rss_title = $2 AND channel_id = $3",
        )
        .bind(msg.id.0 as i64)
        .bind(title)
        .bind(channel_id.0 as i64)
        .execute(db)
        .await
        .map_err(Error::Database)
        {
            tracing::error!("Failed to update rss message id: {:?}", why);
        }
    };

    Ok(())
}

async fn post_item(
    ctx: &serenity::Context,
    db: &sqlx::PgPool,
    channel_id: &serenity::model::id::ChannelId,
    conf: &TaskConfigRss,
    title: &str,
    link: &str,
    description: &str,
    date: &chrono::DateTime<chrono::Local>,
) -> Result<(), Error> {
    let msg = channel_id
        .send_message(&ctx, |f| {
            f.content(format!("Neue Nachricht im Planungsportal · {}", title))
                .embed(|e| {
                    e.title(title)
                        .url(link)
                        .description(conf.clean_regex.replace_all(description, ""))
                        .timestamp(date.to_rfc3339())
                        .color(0xb00b69)
                })
                .components(|c| {
                    c.create_action_row(|a| {
                        a.create_button(|b| {
                            b.label("Open in Browser")
                                .style(serenity::ButtonStyle::Link)
                                .url(link)
                        })
                    })
                })
        })
        .await
        .map_err(Error::Serenity);

    if let Ok(msg) = msg {
        if let Err(why) = sqlx::query(
            "INSERT INTO posted_rss (rss_title, channel_id, message_id) VALUES ($1, $2, $3)",
        )
        .bind(title)
        .bind(channel_id.0 as i64)
        .bind(msg.id.0 as i64)
        .execute(db)
        .await
        .map_err(Error::Database)
        {
            tracing::error!("Failed to insert rss message id: {:?}", why);
        }
    }

    Ok(())
}

pub async fn log_latency_to_influx(
    ctx: &serenity::Context,
    sm: Arc<serenity::Mutex<serenity::ShardManager>>,
    influx: &influxdb2::Client,
) -> Result<(), Error> {
    loop {
        info!("Logging latency to influx");
        let shard = ctx.shard_id;
        let locked = sm.lock().await;
        let runner = locked.runners.lock().await;
        let latency = runner
            .get(&ShardId(shard))
            .unwrap()
            .latency
            .unwrap_or(std::time::Duration::from_nanos(0));

        let points = vec![DataPoint::builder("latency")
            .field("latency", latency.as_millis() as i64)
            .timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap())
            .build()
            .unwrap()];

        influx
            .write("facultymanager", futures::stream::iter(points))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
