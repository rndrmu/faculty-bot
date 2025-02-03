use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;
use rocket::http::{CookieJar, Status};
use rocket::request::{Outcome, Request, FromRequest};




#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    id: u64,
    // bitfield for perms
    role: Roles,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Roles {
    Unprivileged,
    User,
    Moderator,
    Admin,
}


impl User {
    pub fn new(id: u64) -> Self {
        User {
            id,
            role: Roles::Unprivileged,
        }
    }

    pub fn create_token(&self, role: Roles) -> String {
        let secret_key = std::env::var("SECRET_KEY").expect("SECRET_KEY must be set");
        let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");
        let mut claims = BTreeMap::new();
        claims.insert("id", self.id);
        claims.insert("exp", chrono::Utc::now().timestamp() as u64 + 86400); // 86400s = 24h
        claims.insert("iat", chrono::Utc::now().timestamp() as u64);
        claims.insert("role", role as u64);


        let token = claims.sign_with_key(&key).unwrap();

        token
        
    }


    pub fn verify_token(token: &str) -> bool {
        let secret_key = std::env::var("SECRET_KEY").expect("SECRET_KEY must be set");
        let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");

        token.verify_with_key(&key)
            .ok()
            .and_then(|claims: BTreeMap<String, u64>| {
                let exp = claims.get("exp")?;
                let current_time = chrono::Utc::now().timestamp() as u64;
                
                if current_time > *exp || !claims.contains_key("id") {
                    None
                } else {
                    Some(true)
                }
            })
            .unwrap_or_else(|| {
                println!("Token verification failed");
                false
            })
    }

    pub fn user_has_role(token: &str, role: Roles) -> bool {
        let secret_key = std::env::var("SECRET_KEY").expect("SECRET_KEY must be set");
        let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");

        token.verify_with_key(&key)
            .ok()
            .and_then(|claims: BTreeMap<String, u64>| {
                let exp = claims.get("exp")?;
                let role_claim = claims.get("role")?;
                let current_time = chrono::Utc::now().timestamp() as u64;

                if current_time > *exp {
                    println!("token is expired");
                    None
                } else if role_claim >= &(role as u64) {
                    println!("user has required role");
                    Some(true)
                } else {
                    println!("user does not have required role");
                    None
                }
            })
            .unwrap_or_else(|| {
                println!("token verification failed or missing claims");
                false
            })
    }
    

}

#[allow(unused)]
pub struct AdminUser<'r>(&'r str);
#[allow(unused)]
pub struct AuthenticatedUser<'r>(&'r str);

#[derive(Debug)]
pub enum ApiKeyError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminUser<'r> {
    type Error = ApiKeyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        /// Returns true if `key` is a valid API key string.
        fn is_valid(key: &str) -> bool {
            let is_valid = User::verify_token(key);

            let has_role = User::user_has_role(key, Roles::Admin);

            println!("Is valid: {}", is_valid);

            is_valid && has_role
        }

        let key = req.cookies().get("token").map(|cookie| cookie.value());
        match key {
            None => Outcome::Error((Status::Unauthorized, ApiKeyError::Missing)),
            Some(key) if is_valid(key) => Outcome::Success(AdminUser(key)),
            Some(_) => Outcome::Error((Status::Unauthorized, ApiKeyError::Invalid)),
        }
        
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser<'r> {
    type Error = ApiKeyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        /// Returns true if `key` is a valid API key string.
        fn is_valid(key: &str) -> bool {
            let is_valid = User::verify_token(key);

            let has_role = User::user_has_role(key, Roles::User);

            println!("Is valid: {}", is_valid);

            is_valid && has_role
        }

        let key = req.cookies().get("token").map(|cookie| cookie.value());
        match key {
            None => Outcome::Error((Status::Unauthorized, ApiKeyError::Missing)),
            Some(key) if is_valid(key) => Outcome::Success(AuthenticatedUser(key)),
            Some(_) => Outcome::Error((Status::Unauthorized, ApiKeyError::Invalid)),
        }
    }
}


pub async fn is_logged_in(jar: &CookieJar<'_>) -> bool {
    let key = jar.get("token").map(|cookie| cookie.value());
    match key {
        None => false,
        Some(key) => User::verify_token(key),
    }
}