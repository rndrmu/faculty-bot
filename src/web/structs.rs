use serde::{self, Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Email {
    pub email: String,
}



#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Response<T> {
    pub data: T,
    pub status: u16,
    pub message: String,
}

#[derive(Deserialize)]
pub  struct Code {
    pub code: String,
}
