pub mod api;
pub mod auth;
pub mod structs;
use auth::{is_logged_in, AdminUser, AuthenticatedUser, Roles, User};
use rocket_dyn_templates::Template;





#[get("/")]
pub async fn index(
    jar: &CookieJar<'_>,
) -> Template {
    let is_logged_in = is_logged_in(jar).await;
    Template::render("home", &{
        let mut context = std::collections::HashMap::new();
        context.insert("is_logged_in", is_logged_in);
        context
    })
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

