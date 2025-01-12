use crate::{
    prelude::Error,
    structs::{self},
    tasks, utils, Data,
};



use influxdb2::models::DataPoint;
use poise::serenity_prelude::{self as serenity, AttachmentType, Mentionable};
use tracing::{debug, info};


pub async fn event_listener(
    ctx: &serenity::Context,
    event: &poise::Event<'_>,
    fw: &poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => {
            info!("Ready! Logged in as {}", data_about_bot.user.name);
            info!("Prefix: {:?}", fw.options.prefix_options.prefix.as_ref());

            // start email task

            // start mensa task & rss task if enabled so they can run in parallel
            if data.config.mealplan.post_mealplan {
                info!("Mensaplan task started");
                let context = ctx.clone();
                let d = data.clone();
                tokio::spawn(async move {
                    tasks::post_mensaplan(context, d).await.unwrap();
                });
            }

            if data.config.rss_settings.post_rss {
                info!("RSS task started");
                let context = ctx.clone();
                let d = data.clone();
                tokio::spawn(async move {
                    tasks::post_rss(context, d).await.unwrap();
                });
            }

            // start logger task
            info!("Logger task started");
            let context = ctx.clone();
            let fw = fw.shard_manager.clone();
            let d = data.influx.clone();
            tokio::spawn(async move {
                tasks::log_latency_to_influx(&context, fw, &d).await.unwrap();
            });
        }

        poise::Event::Message { new_message } => {
            // skip bots and messages starting with the prefix
            if new_message.author.bot
                || new_message
                    .content
                    .starts_with(fw.options.prefix_options.prefix.as_deref().unwrap_or_default())
            {
                return Ok(());
            }
        
            let user_id = i64::from(new_message.author.id);
            let content_len = new_message.content.chars().count();
        
            let mut pool = data.db.acquire().await.map_err(Error::Database)?;
        
            // fetch user data or create defaults
            let user_data = sqlx::query_as::<_, structs::UserXP>(
                "SELECT * FROM user_xp WHERE user_id = $1"
            )
            .bind(user_id)
            .fetch_optional(&mut pool)
            .await
            .map_err(Error::Database)?
            .unwrap_or_else(|| structs::UserXP {
                user_id,
                user_xp: 0.0,
                user_level: 0,
            });
        
            debug!("{}: {}", new_message.author.name, user_data.user_xp);
        
            // scaling formula: xp = base_xp / (1 + scale_factor * level) (logarithmic)
            let scaling_factor = data.config.general.xp_scaling_factor;
            let base_xp = content_len as f64 / data.config.general.chars_for_level as f64;
            let xp_to_add = base_xp / (1.0 + scaling_factor * (user_data.user_level as f64).ln());
        
            // update xp and save to db
            let new_xp = user_data.user_xp + xp_to_add;
            sqlx::query(
                "INSERT INTO user_xp (user_id, user_xp) VALUES ($1, $2)
                ON CONFLICT (user_id) DO UPDATE SET user_xp = $2"
            )
            .bind(user_id)
            .bind(new_xp)
            .execute(&mut pool)
            .await
            .map_err(Error::Database)?;
        
            debug!(
                "{}: {} -> {} | Level: {}",
                new_message.author.name,
                user_data.user_xp,
                new_xp,
                user_data.user_level
            );
        
            // determine if level-up occurred
            let new_level = (new_xp / 100.0).floor() as i32;
            if new_level > user_data.user_level {
                // update level and save
                sqlx::query(
                    "INSERT INTO user_xp (user_id, user_level) VALUES ($1, $2)
                    ON CONFLICT (user_id) DO UPDATE SET user_level = $2"
                )
                .bind(user_id)
                .bind(new_level)
                .execute(&mut pool)
                .await
                .map_err(Error::Database)?;
        
                // generate level-up message
                let img = utils::show_levelup_image(&new_message.author, new_level as u16).await?;
                data.config.channels.xp
                    .send_message(&ctx, |f| {
                        f.content(format!(
                            "congrats {}! you've levelled up to {}!",
                            new_message.author.mention(),
                            new_level
                        ))
                        .add_file(AttachmentType::Bytes {
                            data: std::borrow::Cow::Borrowed(&img),
                            filename: "levelup.png".to_string(),
                        })
                    })
                    .await
                    .map_err(Error::Serenity)?;
            }
        
        }
        
        poise::Event::VoiceStateUpdate { old, new } => {
            let created_channels = sqlx::query_as::<sqlx::Postgres, structs::VoiceChannels>(
                "SELECT * FROM voice_channels",
            )
            .fetch_all(&mut data.db.acquire().await.map_err(Error::Database)?)
            .await
            .map_err(Error::Database)?;

            if let Some(old_chan) = old {
                if old_chan.channel_id == new.channel_id {
                    // user moved in same channel
                    return Ok(());
                }

                // if no one is in the channel anymore, delete it
                let channel = old_chan
                    .channel_id
                    .unwrap_or_default()
                    .to_channel(&ctx)
                    .await
                    .map_err(Error::Serenity)?;
                if let serenity::Channel::Guild(channel) = channel {
                    if channel.name() == data.config.channels.create_channel {
                        return Ok(()); // don't delete the create channel
                    }

                    // also we dont want to delete any other non temp channel
                    if !created_channels
                        .iter()
                        .any(|c| c.channel_id == channel.id.0 as i64)
                    {
                        return Ok(());
                    }

                    if channel
                        .members(&ctx)
                        .await
                        .map_err(Error::Serenity)?
                        .is_empty()
                    {
                        channel.delete(&ctx).await.map_err(Error::Serenity)?;
                        // remove channel from db
                        sqlx::query("DELETE FROM voice_channels WHERE channel_id = $1")
                            .bind(channel.id.0 as i64)
                            .execute(&mut data.db.acquire().await.map_err(Error::Database)?)
                            .await
                            .map_err(Error::Database)?;
                    }
                }
            }

            let new_channel = new
                .channel_id
                .unwrap_or_default()
                .to_channel(&ctx)
                .await
                .map_err(Error::Serenity)?;
            let new_channel = match new_channel {
                serenity::Channel::Guild(channel) => channel,
                _ => return Ok(()),
            };

            if &new_channel.name() == &data.config.channels.create_channel {
                let category = new_channel.parent_id;

                let cc = new
                    .guild_id
                    .unwrap()
                    .create_channel(&ctx, |f| {
                        f.name(format!(
                            "🔊 {}'s Channel",
                            new.member.as_ref().unwrap().display_name()
                        ))
                        .kind(serenity::ChannelType::Voice)
                        .permissions(vec![
                            serenity::PermissionOverwrite {
                                allow: serenity::Permissions::MANAGE_CHANNELS,
                                deny: serenity::Permissions::empty(),
                                kind: serenity::PermissionOverwriteType::Member(
                                    new.member.as_ref().unwrap().user.id,
                                ),
                            },
                        ]);
                        if category.is_some() {
                            f.category(category.unwrap())
                        } else {
                            f
                        }
                    })
                    .await
                    .map_err(Error::Serenity)?;

                sqlx::query("INSERT INTO voice_channels (channel_id, owner_id) VALUES ($1, $2)")
                    .bind(cc.id.0 as i64)
                    .bind(new.member.as_ref().unwrap().user.id.0 as i64)
                    .execute(&mut data.db.acquire().await.map_err(Error::Database)?)
                    .await
                    .map_err(Error::Database)?;

                new.member
                    .as_ref()
                    .unwrap()
                    .move_to_voice_channel(&ctx, cc)
                    .await
                    .map_err(Error::Serenity)?;
            }
        }
        poise::Event::InteractionCreate { interaction } => {
            // filter out button interactions
            if let serenity::Interaction::MessageComponent(button) = interaction {
                match button.data.custom_id.as_str() {
                    "mensaplan_notify_button" => {
                        give_user_mensaplan_role(ctx, button, data).await?
                    }
                    _ => not_implemented(ctx, button).await?,
                }
            }
        }
        _ => {}
    }

    Ok(())
}

async fn give_user_mensaplan_role(
    ctx: &serenity::Context,
    button: &serenity::model::application::interaction::message_component::MessageComponentInteraction,
    bot_data: &Data,
) -> Result<(), Error> {
    let role = bot_data.config.roles.mealplannotify;
    let member = match button.member.as_ref() {
        Some(m) => m,
        None => {
            button
                .create_interaction_response(&ctx, |f| {
                    f.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|f| {
                            f.content("You need to be in a server to use this command")
                        })
                })
                .await
                .map_err(Error::Serenity)?;
            return Ok(());
        }
    };

    member
        .clone() // literally why, go explod rustc
        .add_role(&ctx, role)
        .await
        .map_err(Error::Serenity)?;

    button
        .create_interaction_response(&ctx, |f| {
            f.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|f| {
                    f.flags(serenity::model::application::interaction::MessageFlags::EPHEMERAL)
                    .content("You will now be notified when the mensaplan is updated, feel free to also follow this channel in your own server !!")
                })
        })
        .await
        .map_err(Error::Serenity)?;

    Ok(())
}

/// Generic function to handle not implemented buttons
async fn not_implemented(
    ctx: &serenity::Context,
    button: &serenity::model::application::interaction::message_component::MessageComponentInteraction,
) -> Result<(), Error> {
    button
        .create_interaction_response(&ctx, |f: &mut serenity::CreateInteractionResponse| {
            f.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|f| {
                    f.flags(serenity::model::application::interaction::MessageFlags::EPHEMERAL)
                        .content("This button is not implemented yet, sorry :(")
                })
        })
        .await
        .map_err(Error::Serenity)?;

    Ok(())
}


