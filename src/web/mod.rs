pub mod routes;
pub mod auth;


use core::prelude;
use std::time;

use auth::{AdminUser, Roles, User};
use rocket::response::Redirect;
use rocket_dyn_templates::Template;
use rocket::serde::{Serialize, json::Json};
use serde::Deserialize;



#[get("/")]
pub fn index() -> Template {
    Template::render("home", &{})
}

#[get("/reverify")]
pub fn reverify() -> Template {
    Template::render("reverify", &{})
}

#[get("/verify")]
pub fn verify() -> Template {
    Template::render("verify", &{})
}

#[get("/login")]
pub fn login() -> Template {
    Template::render("login", &{})
}

#[get("/logout")]
pub fn logout() -> Template {
    Template::render("logout", &{})
}

#[derive(Deserialize)]
pub struct Email {
    email: String,
}

#[post("/api/verify/sendMail", format = "application/json", data = "<email>")]
pub fn send_mail(email: Json<Email>) -> Json<Response<String>> {
    println!("Email: {}", email.email);

    let email_regex = regex::Regex::new(r"^[a-zA-Z0-9_.+-]+@stud.hs-kempten.de$").unwrap();
    // check if the email is valid
    if !email_regex.is_match(&email.email) {
        return Json(Response {
            data: "FAILTHIS".to_string(),
            status: 400,
            message: "Ungültige E-Mail Adresse".to_string(),
        });
    }

    Json(Response {
        data: "SUCCESS".to_string(),
        status: 200,
        message: "E-Mail wurde erfolgreich versendet".to_string(),
    })
}


#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Response<T> {
    data: T,
    status: u16,
    message: String,
}

#[derive(Deserialize)]
pub  struct Code {
    code: String,
}

#[post("/api/verify/checkCode", format = "application/json", data = "<code>")]
pub fn check_code(code: Json<Code>) -> Json<Response<String>> {
    println!("Code: {}", code.code);
    println!("Code == FAILTHIS: {}", code.code == "FAILTHIS");
    if code.code == "FAILTHIS" {
        Json(Response {
            data: "FAILTHIS".to_string(),
            status: 400,
            message: "Ungültiger Code".to_string(),
        })
    } else {
        Json(Response {
            data: "SUCCESS".to_string(),
            status: 200,
            message: "Code is valid".to_string(),
        })
    }
}

use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{self, Outcome, Request, FromRequest};


/// Admin dashboard
#[get("/admin")]
pub fn admin(user: AdminUser<'_>) -> Template {
    Template::render("admin", &{})
}

#[catch(404)]
pub fn not_found(req: &Request) -> Template {
    Template::render("404", &{})
}

#[catch(401)]
pub fn unauthorized(req: &Request) -> Template {
    Template::render("401", &{})
}

#[get("/api/auth/discord")]
pub fn discord_auth() -> Redirect {
    let client_id = std::env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID must be set");
    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI").expect("DISCORD_REDIRECT_URI must be set");
    let redirect_uri = format!("{}", redirect_uri);
    let discord_auth_url = format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify", client_id, redirect_uri);
    Redirect::to(discord_auth_url)
}

#[get("/api/auth/discord/callback?<code>")]
pub async fn discord_callback(
    code: String,
    jar: &CookieJar<'_>,
) -> Template {
    println!("Code: {}", code);

    let client_id = std::env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID must be set");
    let client_secret = std::env::var("DISCORD_CLIENT_SECRET").expect("DISCORD_CLIENT_SECRET must be set");
    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI").expect("DISCORD_REDIRECT_URI must be set");

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"));

    let client = reqwest::Client::new();

    let response = client.post("https://discord.com/api/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "authorization_code".to_string()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("scope", "identify".to_string()),
        ])
        .send()
        .await
        .unwrap();

    let response_json: serde_json::Value = response.json().await.unwrap();
    let access_token = response_json["access_token"].as_str().unwrap();

    let user_response = client.get("https://discord.com/api/users/@me")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    let user_response_json: serde_json::Value = user_response.json().await.unwrap();
    let user_id = user_response_json["id"].as_str().unwrap();
    let username = user_response_json["username"].as_str().unwrap();
    let mut pfp = user_response_json["avatar"].as_str().unwrap().to_string();
    
    // get link to pfp (gif or png)
    if pfp.starts_with("a_") {
        pfp = format!("https://cdn.discordapp.com/avatars/{}/{}.gif", user_id, pfp);
    } else {
        pfp = format!("https://cdn.discordapp.com/avatars/{}/{}.png", user_id, pfp);
    }

    // we want to create a token dor them 
    let user = User::new(user_id.parse().unwrap());
    let token = user.create_token(Roles::Admin);

    println!("User ID: {}", user_id);
    println!("Username: {}", username);

    // set cookies 
    let cookies = Cookie::build(("token", token))
        .path("/")
        .secure(true)
        .http_only(true);

    jar.add(cookies);

    Template::render("discord_callback", &{
        let mut context = std::collections::HashMap::new();
        context.insert("user_id", user_id);
        context.insert("username", username);
        context.insert("avatar", pfp.as_str());
        context
    })
}