use rocket::{http::{Cookie, CookieJar}, response::Redirect, serde::json::Json};
use rocket_dyn_templates::Template;
use serde::Deserialize;

use super::structs::{Code, Email, Response};

use crate::web::{Roles, User};


#[post("/verify/sendMail", format = "application/json", data = "<email>")]
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

#[post("/verify/checkCode", format = "application/json", data = "<code>")]
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


#[get("/auth/discord")]
pub fn discord_auth() -> Redirect {
    let client_id = std::env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID must be set");
    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI").expect("DISCORD_REDIRECT_URI must be set");
    let redirect_uri = format!("{}", redirect_uri);
    let discord_auth_url = format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify+guilds", client_id, redirect_uri);
    Redirect::to(discord_auth_url)
}

#[get("/auth/discord/callback?<code>")]
pub async fn discord_callback(code: String, jar: &CookieJar<'_>) -> Result<Template, rocket::http::Status> {
    let client = reqwest::Client::new();
    
    // Get OAuth tokens
    let token_response = get_discord_token(&client, &code).await
        .map_err(|_| rocket::http::Status::InternalServerError)?;
    
    // Get user info
    let user_info = get_discord_user(&client, &token_response.access_token).await
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    let guilds = get_discord_guilds(&client, &token_response.access_token).await
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    // check if they are a member of the correct guild (specified in .env DISCORD_SERVER_ID)
    let guild_id = std::env::var("DISCORD_SERVER_ID").expect("DISCORD_SERVER_ID must be set");
    let guild_id = guild_id.parse::<u64>().expect("DISCORD_SERVER_ID must be a number");
    let is_member = guilds.iter().any(|guild| guild.id.parse::<u64>().unwrap() == guild_id);

    if !is_member {
        return Ok(Template::render("noAccess", &{
            let mut context = std::collections::HashMap::new();
            context.insert("serverName", "HS Kempten".to_string());
            context
        }));
    }


    // Create user token and set cookie
    let user = User::new(user_info.id.parse().unwrap());
    // if id is 242385294123335690 give admin role
    let token = if user_info.id == "242385294123335690" {
        user.create_token(Roles::Admin)
    } else {
        user.create_token(Roles::User)
    };
    
    jar.add(Cookie::build(("token", token))
        .path("/")
        .secure(true)
        .http_only(true));

    // Render template
    Ok(Template::render("discord_callback", &{
        let mut context = std::collections::HashMap::new();
        context.insert("user_id", user_info.id.clone());
        context.insert("username", user_info.username.clone());
        context.insert("avatar", user_info.get_avatar_url());
        context
    }))
}

#[derive(Deserialize)]
pub struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
pub struct UserInfo {
    id: String,
    username: String,
    avatar: String,
}

impl UserInfo {
    fn get_avatar_url(&self) -> String {
        let ext = if self.avatar.starts_with("a_") { "gif" } else { "png" };
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.{}",
            self.id, self.avatar, ext
        )
    }
}

async fn get_discord_token(client: &reqwest::Client, code: &str) -> Result<TokenResponse, reqwest::Error> {
    client.post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", std::env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID must be set")),
            ("client_secret", std::env::var("DISCORD_CLIENT_SECRET").expect("DISCORD_CLIENT_SECRET must be set")),
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", std::env::var("DISCORD_REDIRECT_URI").expect("DISCORD_REDIRECT_URI must be set")),
            ("scope", "identify".to_string()),
        ])
        .send()
        .await?
        .json()
        .await
}

async fn get_discord_user(client: &reqwest::Client, access_token: &str) -> Result<UserInfo, reqwest::Error> {
    client.get("https://discord.com/api/users/@me")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?
        .json()
        .await
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(unused)]
#[non_exhaustive]
pub struct Guild {
    id: String,
    name: String,
    icon: Option<String>,
    owner: bool,
    permissions: u64,
}

pub async fn get_discord_guilds(client: &reqwest::Client, access_token: &str) -> Result<Vec<Guild>, reqwest::Error> {
    client.get("https://discord.com/api/users/@me/guilds")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?
        .json::<Vec<Guild>>()
        .await
}

use reqwest::Client;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct DiscordOAuthClient {
    client: Client,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    api_base: String,
}

impl DiscordOAuthClient {
    
    #[allow(unused)]
    pub fn new() -> Result<Self, std::env::VarError> {
        Ok(Self {
            client: Client::new(),
            client_id: std::env::var("DISCORD_CLIENT_ID")?,
            client_secret: std::env::var("DISCORD_CLIENT_SECRET")?,
            redirect_uri: std::env::var("DISCORD_REDIRECT_URI")?,
            api_base: "https://discord.com/api".to_string(),
        })
    }

    #[allow(unused)]
    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, reqwest::Error> {
        let form = [
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("grant_type", &"authorization_code".to_string()),
            ("code", &code.to_string()),
            ("redirect_uri", &self.redirect_uri),
            ("scope", &"identify".to_string()),
        ];

        self.client.post(format!("{}/oauth2/token", self.api_base))
            .form(&form)
            .send()
            .await?
            .json()
            .await
    }

    #[allow(unused)]
    pub async fn get_current_user(&self, access_token: &str) -> Result<UserInfo, reqwest::Error> {
        self.get_authenticated_resource("users/@me", access_token).await
    }

    #[allow(unused)]
    pub async fn get_user_guilds(&self, access_token: &str) -> Result<Vec<Guild>, reqwest::Error> {
        self.get_authenticated_resource("users/@me/guilds", access_token).await
    }

    async fn get_authenticated_resource<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        access_token: &str
    ) -> Result<T, reqwest::Error> {
        self.client.get(format!("{}/{}", self.api_base, endpoint))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?
            .json()
            .await
    }

    
}