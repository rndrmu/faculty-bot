
use rocket_dyn_templates::Template;
use rocket::serde::{Serialize, json::Json};
use serde::Deserialize;



#[get("/")]
pub fn index() -> &'static str {
    "Hello, world!"
}

#[get("/verify")]
pub fn verify() -> Template {
    Template::render("reverify", &{})
}

#[derive(Serialize, Deserialize)]
struct Email {
    email: String,
}

#[post("/api/verify/sendMail", format = "application/json", data = "<email>")]
pub fn send_mail(email: Json<Email>) -> Json<Response<String>> {
    println!("Email: {}", email.email);
    Json(Response {
        data: "SUCCESS".to_string(),
        status: 200,
        message: "Email sent".to_string(),
    })
}


#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Response<T> {
    data: T,
    status: u16,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct Code {
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
            message: "Invalid code".to_string(),
        })
    } else {
        Json(Response {
            data: "SUCCESS".to_string(),
            status: 200,
            message: "Code is valid".to_string(),
        })
    }
}