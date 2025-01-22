pub mod api;
pub mod auth;
pub mod structs;
use auth::{AdminUser, AuthenticatedUser, Roles, User};
use rocket_dyn_templates::Template;





#[get("/")]
pub fn index() -> Template {
    Template::render("home", &{})
}

#[get("/reverify")]
pub fn reverify(_user: AuthenticatedUser<'_>) -> Template {
    Template::render("reverify", &{})
}

#[get("/verify")]
pub fn verify(_user: AuthenticatedUser<'_>) -> Template {
    Template::render("verify", &{})
}

#[get("/switch-account")]
pub fn switch_account(_user: AuthenticatedUser<'_>) -> Template {
    Template::render("switch-account", &{})
}

#[get("/login")]
pub fn login() -> Template {
    Template::render("login", &{})
}

#[get("/logout")]
pub fn logout() -> Template {
    Template::render("logout", &{})
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

