use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{self, Outcome, Request, FromRequest};




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
        let iv = std::env::var("IV").expect("IV must be set");

        let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");
        let mut claims = BTreeMap::new();
        claims.insert("id", self.id);
        claims.insert("exp", chrono::Utc::now().timestamp() as u64 + 1000); // expire in 1000 seconds (about 16 minutes)
        claims.insert("iat", chrono::Utc::now().timestamp() as u64);
        claims.insert("role", role as u64);


        let token = claims.sign_with_key(&key).unwrap();

        token
        
    }


    pub fn verify_token(token: &str) -> bool {
        let secret_key = std::env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    
        let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
    
        // verify token signature and deserialize claims
        let claims: Result<BTreeMap<String, u64>, _> = token.verify_with_key(&key);
    
        match claims {
            Ok(claims) => {
                if let Some(exp) = claims.get("exp") {
                    // validate expiration
                    let current_time = chrono::Utc::now().timestamp() as u64;
                    if current_time > *exp {
                        println!("token is expired");
                        return false;
                    }
                }
    
                // ensure required claims exist
                if claims.contains_key("id") {
                    println!("token is valid");
                    return true;
                } else {
                    println!("token is missing required 'id' claim");
                }
            }
            Err(err) => println!("token verification failed: {:?}", err),
        }
    
        false
    }

    pub fn user_has_role(token: &str, role: Roles) -> bool {
        let secret_key = std::env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    
        let key: Hmac<Sha256> = Hmac::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
    
        // verify token signature and deserialize claims
        let claims: Result<BTreeMap<String, u64>, _> = token.verify_with_key(&key);
    
        match claims {
            Ok(claims) => {
                if let Some(exp) = claims.get("exp") {
                    // validate expiration
                    let current_time = chrono::Utc::now().timestamp() as u64;
                    if current_time > *exp {
                        println!("token is expired");
                        return false;
                    }
                }
    
                // ensure required claims exist
                if let Some(role_claim) = claims.get("role") {
                    println!("role claim: {}", role_claim);
                    // has that role or higher (Admin > Moderator > User > Unprivileged)
                    if role_claim >= &(role as u64) {
                        println!("user has required role");
                        return true;
                    } else {
                        println!("user does not have required role");
                    }
                } else {
                    println!("token is missing required 'role' claim");
                }
            }
            Err(err) => println!("token verification failed: {:?}", err),
        }
    
        false
    }
    

}


pub struct AdminUser<'r>(&'r str);
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
