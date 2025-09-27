pub mod api;
pub mod auth;
pub mod structs;
use auth::{is_logged_in, AdminUser, AuthenticatedUser, Roles, User};
use rocket_dyn_templates::Template;




#[derive(serde::Serialize)]
struct HomeContext {
    is_logged_in: bool,
    is_admin: bool,
}

#[get("/")]
pub async fn index(
    jar: &CookieJar<'_>,
) -> Template {
    let is_logged_in = is_logged_in(jar).await;
    let is_admin = User::user_has_role(jar.get("token").map(|cookie| cookie.value()).unwrap_or_default(), Roles::Admin);
    
    let ctx = HomeContext {
        is_logged_in,
        is_admin,
    };
    
    Template::render("wip", &ctx)
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
pub fn logout(jar: &CookieJar<'_>) -> Template {
    jar.remove("token");
    
    Template::render("logout", &{})

}

#[get("/setup")]
pub fn setup() -> Template {
    
    Template::render("setup", &{})

}




use rocket::http::CookieJar;
use rocket::request::Request;


/// Admin dashboard
#[get("/admin")]
pub fn admin(_user: AdminUser<'_>) -> Template {
    Template::render("admin", &{})
}

#[catch(404)]
pub fn not_found(_req: &Request) -> Template {
    Template::render("404", &{})
}

#[catch(401)]
pub fn unauthorized(_req: &Request) -> Template {
    Template::render("401", &{})
}

