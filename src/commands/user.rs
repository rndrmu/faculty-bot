use crate::{
    prelude::{translations::Lang, Error},
    structs::{self, CodeEmailPair},
    utils::CurrentEmail,
    Context,
};
use poise::serenity_prelude as serenity;

/// Base command for verification specific commands
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    rename = "verify",
    name_localized("de", "verifizieren"),
    description_localized("de", "Verifiziere dich mit deiner Studierenden E-Mail Adresse"),
    guild_only,
    subcommands("init", "code")
)]
pub async fn verify(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}


/// Verify yourself with your student email address
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    guild_only,
    name_localized("de", "init"),
    description_localized("de", "Verifiziere dich mit deiner Studierenden E-Mail Adresse"),
    ephemeral,
)]
pub async fn init(
    ctx: Context<'_>,
    #[description = "Your student email address (must be ending in @stud.hs-kempten.de)"]
    #[description_localized(
        "de",
        "Deine Studierenden E-Mail Adresse (muss mit @stud.hs-kempten.de enden)"
    )]
    #[name_localized("de", "email-adresse")]
    #[rename = "email"]
    email_used: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await.map_err(Error::Serenity)?;

    let lang = match ctx.locale() {
        Some("de") => Lang::De,
        Some("ja") => Lang::Ja,
        _ => Lang::En,
    };

    // check if email is valid
    if !email_used.ends_with("@stud.hs-kempten.de") {
        return Err(Error::WithMessage(lang.invalid_email().into()));
    }

    // check if email is already in use
    let pool = &ctx.data().db;
    let user = sqlx::query_as::<sqlx::Postgres, structs::VerifiedUsers>(
        "SELECT * FROM verified_users WHERE user_email = $1",
    )
    .bind(&email_used)
    .fetch_optional(pool)
    .await
    .map_err(Error::Database)?;

    if user.is_some() {
        return Err(Error::WithMessage(lang.err_already_verified().into()));
    }

    let code = crate::utils::generate_verification_code();

    let user_id = ctx.author().id;
    ctx.data().email_codes.insert(user_id, CodeEmailPair { code: code.clone(), email: email_used.clone() });

    let emilia = ctx.data().email_task.clone();

    let email = CurrentEmail::new(
        email_used.clone(),
        ctx.author().id,
        ctx.author().name.clone(),
        code.clone(),
    );

    if let Err(why) = emilia.send(email).await {
        println!("Error sending email: {:?}", why);
        ctx.send(|msg| {
            msg.embed(|embed| {
                embed.description(
                    "##  Es ist ein Fehler aufgetreten. Bitte versuche es später erneut. \n\n\
                        ",
                );
                embed
            })
        })
        .await
        .map_err(Error::Serenity)?;
    }

    //let a = send_email(&email, ctx.author().id, &ctx.author().name).await;

    ctx.send(|msg| {
        msg.embed(|embed| {
            embed.description(lang.code_email_enqueued(email_used));
            embed
        })
    })
    .await
    .map_err(Error::Serenity)?;

    // send code for debug reasons
    ctx.send(|msg| {
        msg.embed(|embed| {
            embed.description(format!("Your code is: {}", code));
            embed
        })
    }).await.map_err(Error::Serenity)?;

    /*
    let mmail = crate::utils::find_discord_tag(&ctx.author().tag()).await;

    let _mail_found = match mmail {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err(Error::WithMessage("Could not find a mail containing your discord tag. Please try again. Contact an admin if this error persists.".into()));
        }
        Err(e) => {
            return Err(Error::WithMessage(format!("An error occured while trying to find your mail. Please try again. Contact an admin if this error persists. Error: {}", e)));
        }
    };

    // check if user is already verified
    let pool = &ctx.data().db;
    let user_id = ctx.author().id.0 as i64;

    let user = sqlx::query("SELECT * FROM verified_users WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::Database)?;

    if user.is_some() {
        return Err(Error::WithMessage("You are already verified".to_string()));
    } else {
        sqlx::query("INSERT INTO verified_users (user_id, user_email) VALUES ($1, $2)")
            .bind(user_id)
            .bind(email)
            .execute(pool)
            .await
            .map_err(Error::Database)?;

        ctx.say("You are now verified!")
            .await
            .map_err(Error::Serenity)?;

        // give them the verified role
        let verified_role = ctx.data().config.roles.verified;

        let mem = ctx.author_member().await.unwrap();
        mem.into_owned()
            .add_role(&ctx.serenity_context(), verified_role)
            .await
            .map_err(Error::Serenity)?;
    } */

    Ok(())
}

/// Enter the verification code you received via email to verify yourself
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    guild_only,
    name_localized("de", "code"),
    description_localized("de", "Gib den Code ein, den du per E-Mail erhalten hast, um dich zu verifizieren"),
    ephemeral
)]
pub async fn code(
    ctx: Context<'_>,
    #[description = "The code you received via email"]
    #[description_localized("de", "Der Code, den du per E-Mail erhalten hast")]
    #[rename = "code"]
    supplied_code: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await.map_err(Error::Serenity)?;

    let lang = match ctx.locale() {
        Some("de") => Lang::De,
        Some("ja") => Lang::Ja,
        _ => Lang::En,
    };

    let user_id = ctx.author().id;
    let pool = &ctx.data().db;

    let code = ctx.data().email_codes.get(&user_id);

    if code.is_none() {
        return Err(Error::WithMessage("You have not requested a code yet, please use the `/verify init` command to do so.".into()));
    }

    let code_key = code.unwrap();


    let actual_code = code_key.code == supplied_code;

    if !actual_code {
        return Err(Error::WithMessage("The code you entered is not correct. Please try again.".into()));
    }

    let user = sqlx::query_as::<sqlx::Postgres, structs::VerifiedUsers>(
        "SELECT * FROM verified_users WHERE user_id = $1",
    )
    .bind(user_id.0 as i64)
    .fetch_optional(pool)
    .await
    .map_err(Error::Database)?;

    if user.is_some() {
        return Err(Error::WithMessage(lang.err_already_verified().into()));
    }

    sqlx::query("INSERT INTO verified_users (user_id, user_email) VALUES ($1, $2)")
        .bind(user_id.0 as i64)
        .bind(code_key.email.clone())
        .execute(pool)
        .await
        .map_err(Error::Database)?;

    ctx.send(|msg| {
        msg.embed(|embed| {
            embed.description("You are now verified!")
        })
    })
    .await
    .map_err(Error::Serenity)?;

    // give them the verified role
    let verified_role = ctx.data().config.roles.verified;

    let mem = ctx.author_member().await.unwrap();
    mem.into_owned()
        .add_role(&ctx.serenity_context(), verified_role)
        .await
        .map_err(Error::Serenity)?;

    Ok(())
}

/// Show the Top 10 users by XP
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    name_localized("de", "leaderboard"),
    description_localized("de", "Zeige die besten 10 Nutzer anhand ihrer XP")
)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let pool = &ctx.data().db;
    let users = sqlx::query_as::<sqlx::Postgres, structs::UserXP>(
        "SELECT * FROM user_xp ORDER BY user_xp DESC LIMIT 10",
    )
    .fetch_all(pool)
    .await
    .map_err(Error::Database)?;

    let mut leaderboard = String::new();
    for (i, user) in users.iter().enumerate() {
        let user_discord = serenity::UserId(user.user_id as u64)
            .to_user(&ctx.serenity_context())
            .await
            .map_err(Error::Serenity)?;
        leaderboard.push_str(&format!(
            "{}. {} - {} XP\n",
            i + 1,
            // pomelo-fy affected users (replace #0000 discriminator with empty string and prefix username with an @)
            user_discord.tag().replace("#0000", ""),
            user.user_xp
        ));
    }

    ctx.send(|f| {
        f.embed(|e| {
            e.title("Leaderboard");
            e.description(leaderboard);
            e
        });
        f
    })
    .await
    .map_err(Error::Serenity)?;

    Ok(())
}

/// Show your XP
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    name_localized("de", "xp"),
    description_localized("de", "Zeige deine XP")
)]
pub async fn xp(ctx: Context<'_>) -> Result<(), Error> {
    let pool = &ctx.data().db;
    let user_id = ctx.author().id.0 as i64;

    let user = sqlx::query_as::<sqlx::Postgres, structs::UserXP>(
        "SELECT * FROM user_xp WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(Error::Database)?;

    if let Some(user) = user {
        ctx.send(|f| {
            f.embed(|e| {
                e.description(format!(
                    "You have {} xp, that equals to Level {}",
                    user.user_xp, user.user_level
                ))
            });
            f
        })
        .await
        .map_err(Error::Serenity)?;
    } else {
        ctx.send(|f| {
            f.embed(|e| e.description("You have no XP yet"));
            f
        })
        .await
        .map_err(Error::Serenity)?;
    }

    Ok(())
}
