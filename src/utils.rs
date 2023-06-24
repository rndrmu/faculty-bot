use std::{collections::VecDeque, time::Duration};

use poise::{serenity_prelude as serenity, futures_util::stream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

use crate::prelude::Error;

use askama::Template;
use lettre::{Transport, SmtpTransport, transport::smtp::authentication::Credentials, Message, message::header::ContentType};
use lettre_email::{EmailBuilder, Email};
use lettre::message::{header, MultiPart, SinglePart};

/// Comverts a pdf file to a png buffer
async fn pdf_to_png(filepath: std::path::PathBuf) -> Result<Vec<u8>, Error> {
    let child = Command::new("convert")
        .arg("-density")
        .arg("300")
        .arg(filepath)
        .arg("-flatten")
        .arg("png:-")
        .output()
        .await
        .map_err(Error::IO)?
        .stdout;

    Ok(child)
}

/// Fetch newest Mensaplans from the website
pub async fn fetch_mensaplan<'a>(url: &'a str) -> Result<Vec<u8>, Error> {
    println!("Fetching Mensaplan from {}", url); // be careful not to rape the server
                                                 // download mensaplan
    let response = reqwest::get(url).await.map_err(Error::NetRequest)?;
    let mut file = tokio::fs::File::create("images/mensaplan.pdf")
        .await
        .map_err(Error::IO)?;

    file.write_all(&response.bytes().await.map_err(Error::NetRequest)?)
        .await
        .map_err(Error::IO)?;

    let png = pdf_to_png("images/mensaplan.pdf".into()).await?;
    Ok(png)
}

pub async fn show_levelup_image(user: &serenity::User, level: u16) -> Result<Vec<u8>, Error> {
    let mut file = tokio::fs::File::open("images/banner.png")
        .await
        .map_err(Error::IO)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await.map_err(Error::IO)?;

    let with_text = Command::new("convert")
        .arg("images/banner.png")
        .arg("-font")
        .arg("images/Roboto-Bold.ttf")
        .arg("-gravity")
        .arg("West")
        .arg("-pointsize")
        .arg("35")
        .arg("-fill")
        .arg("white")
        .arg("-draw")
        .arg(format!("text 280,-30 '{} has reached'", user.name))
        .arg("-draw")
        .arg(format!("text 280,45 'LEVEL {}'", level))
        .arg("png:-")
        .output()
        .await;

    if let Ok(with_text) = with_text {
        Ok(with_text.stdout)
    } else {
        Err(Error::WithMessage("Could not convert image".into()))
    }
}

/// Find discord tag in email
pub async fn find_discord_tag(tag: &str) -> imap::error::Result<Option<String>> {
    let domain = std::env::var("SMTP_SERVER").unwrap();
    let username = std::env::var("MAILUSER").unwrap();
    let password = std::env::var("MAILPW").unwrap();
    let tls = native_tls::TlsConnector::builder().build().unwrap();
    // we pass in the domain twice to check that the server's TLS
    // certificate is valid for the domain we're connecting to.
    let client = imap::connect((domain.clone(), 993), domain, &tls).unwrap();

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let mut imap_session = client.login(username, password).map_err(|e| e.0)?;

    // we want to fetch the first email in the INBOX mailbox
    imap_session.select("INBOX")?;

    println!("Selected INBOX");

    // fetch message containing tag in subject
    let message = imap_session.search(format!("SUBJECT \"{}\"", tag))?;

    if message.len() == 0 {
        return Ok(None);
    }

    let message = message.iter().next().unwrap();

    // fetch the email body
    let messages = imap_session.fetch(message.to_string(), "RFC822")?;

    let message = if let Some(m) = messages.iter().next() {
        m
    } else {
        return Ok(None);
    };

    let body = message.body().expect("message did not have a body!");
    let body = std::str::from_utf8(body)
        .expect("message was not valid utf-8")
        .to_string();

    // be nice to the server and log out
    imap_session.logout()?;

    Ok(Some(body))
}

#[derive(Template)]
#[template(path = "verification_email.html")]
struct VerificationEmailTemplate<'a> {
    botname: &'a str,
    code: &'a str,
}

pub async fn send_email(to: impl Into<String>, user_id: serenity::UserId, username: impl Into<String>) -> Result<(), Error> {

    let code = generate_verification_code();
    let mailuser = std::env::var("MAILUSER").unwrap();
    let mailpw = std::env::var("MAILPW").unwrap();
    let smtp_server = std::env::var("SMTP_SERVER").unwrap();

    let email_template = VerificationEmailTemplate {
        botname: "FacultyManager",
        code: &code,
    };

    let receiver = format!("{} <{}>", username.into(), to.into());
    let sender = format!("FacultyManager <{}>", mailuser);

    let email = Message::builder()
        .to(receiver.parse().unwrap_or_else(|_| panic!("Invalid email address: {}", receiver)))
        .from(sender.parse().unwrap_or_else(|_| panic!("Invalid email address: {}", sender)))
        .header(ContentType::TEXT_HTML)
        .subject("Verification Code")
        .body(email_template.render().unwrap())
        .expect("Rendern ist etzala hadde abbeid");      
    
    let creds = Credentials::new(mailuser, mailpw);

    let mailer = SmtpTransport::relay(&smtp_server).unwrap_or_else(|_| panic!("Could not connect to SMTP server {}", smtp_server))
        .credentials(creds)
        .build();


    mailer
        .send(&email).unwrap();


    Ok(())
}

pub fn generate_verification_code() -> String {
// alphanumeric
use rand::Rng;
    let code: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    // encode with hex
   code

}

#[derive(Debug)]
pub struct EmailSender {
    pub tx: tokio::sync::mpsc::Sender<CurrentEmail>,
    pub rx: tokio::sync::mpsc::Receiver<CurrentEmail>,
    mailuser: String,
    mailpw: String,
    smtp_server: String,
}

#[derive(Debug, Clone)]
pub struct CurrentEmail {
    pub to: String,
    pub user_id: serenity::UserId,
    pub code: String,
    pub email: Message,
}

impl CurrentEmail {
    pub fn new(to: impl Into<String>, user_id: serenity::UserId, username: impl Into<String>, code: impl Into<String>) -> Self {
        let recv = to.into();
        let code = code.into();
        let receiver = format!("{} <{}>", username.into(), recv);
        let sender = format!("FacultyManager <{}>", std::env::var("MAILUSER_ADDRESS").unwrap());

        let email_template = VerificationEmailTemplate {
            botname: "FacultyManager",
            code: &code,
        };

        let email = Message::builder()
            .to(receiver.parse().unwrap_or_else(|_| panic!("Invalid email address: {}", receiver)))
            .from(sender.parse().unwrap_or_else(|_| panic!("Invalid email address: {}", sender)))
            .header(ContentType::TEXT_HTML)
            .subject("Verification Code")
            .body(email_template.render().unwrap())
            .expect("Rendern ist etzala hadde abbeid");      
        
        Self {
            to: recv,
            user_id,
            code,
            email,
        }
    }

    pub async fn send(&self) -> Result<(), Error> {
        let creds = Credentials::new(std::env::var("MAILUSER").unwrap(), std::env::var("MAILPW").unwrap());

        let mailer = SmtpTransport::starttls_relay(&std::env::var("SMTP_SERVER").unwrap()).unwrap_or_else(|_| panic!("Could not connect to SMTP server {}", std::env::var("SMTP_SERVER").unwrap()))
            .credentials(creds)
            .build();

        mailer
            .send(&self.email).unwrap();

        Ok(())
    }
}

impl EmailSender {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        Self {
            tx,
            rx,
            mailuser: std::env::var("MAILUSER").unwrap(),
            mailpw: std::env::var("MAILPW").unwrap(),
            smtp_server: std::env::var("SMTP_SERVER").unwrap(),
        }
    }

/*     pub fn send(&mut self, to: impl Into<String>, user_id: serenity::UserId, username: impl Into<String>, code: impl Into<String>) {
        let recv = to.into();
        let code = code.into();
        let receiver = format!("{} <{}>", username.into(), recv);
        let sender = format!("FacultyManager <{}>", self.mailuser);

        let email_template = VerificationEmailTemplate {
            botname: "FacultyManager",
            code: &code,
        };

        let email = Message::builder()
            .to(receiver.parse().unwrap_or_else(|_| panic!("Invalid email address: {}", receiver)))
            .from(sender.parse().unwrap_or_else(|_| panic!("Invalid email address: {}", sender)))
            .header(ContentType::TEXT_HTML)
            .subject("Verification Code")
            .body(email_template.render().unwrap())
            .expect("Rendern ist etzala hadde abbeid");      

        
        
        self.queue.push_back(CurrentEmail {
            to: recv,
            user_id,
            code,
            email,
        });
    } */

    pub async fn run(&mut self) {
        let creds = Credentials::new(self.mailuser.clone(), self.mailpw.clone());

        let mailer = SmtpTransport::relay(&self.smtp_server).unwrap_or_else(|_| panic!("Could not connect to SMTP server {}", self.smtp_server))
            .credentials(creds)
            .build();

        loop {
            if let Some(email) = self.rx.recv().await {
                tracing::debug!("Sending email to {}", email.to);
                mailer
                    .send(&email.email).unwrap();
            }
            tracing::info!("Nothing to do, waiting for new emails...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

pub async fn email_sender(mut rx: tokio::sync::mpsc::Receiver<CurrentEmail>) {
    tracing::info!("Starting email sender...");
    let mailuser = std::env::var("MAILUSER").unwrap();
    let mailpw = std::env::var("MAILPW").unwrap();
    let smtp_server = std::env::var("SMTP_SERVER").unwrap();

    let creds = Credentials::new(mailuser, mailpw);

    let mailer = SmtpTransport::relay(&smtp_server).unwrap_or_else(|_| panic!("Could not connect to SMTP server {}", smtp_server))
        .credentials(creds)
        .build();

    loop {
        tracing::info!("Waiting for new emails...");
        if let Some(email) = rx.recv().await {
            tracing::debug!("Sending email to {}", email.to);
            mailer
                .send(&email.email).unwrap();
        }
        tracing::info!("Nothing to do, waiting for new emails...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}


/// Taken from poise source code thank you kangalioo <3
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderedMap<K, V>(pub Vec<(K, V)>);

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: Eq, V> OrderedMap<K, V> {
    /// Creates a new [`OrderedMap`]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[allow(dead_code)]
    /// Finds a value in the map by the given key
    pub fn get(&self, k: &K) -> Option<&V> {
        self.0
            .iter()
            .find(|entry| &entry.0 == k)
            .map(|entry| &entry.1)
    }

    #[allow(dead_code)]
    /// Inserts a key value pair into the map
    pub fn insert(&mut self, k: K, v: V) {
        match self.0.iter_mut().find(|entry| entry.0 == k) {
            Some(entry) => entry.1 = v,
            None => self.0.push((k, v)),
        }
    }

    /// Finds a value in the map by the given key, or inserts it if it doesn't exist
    pub fn get_or_insert_with(&mut self, k: K, v: impl FnOnce() -> V) -> &mut V {
        match self.0.iter().position(|entry| entry.0 == k) {
            Some(i) => &mut self.0[i].1,
            None => {
                self.0.push((k, v()));
                &mut self.0.last_mut().expect("we just inserted").1
            }
        }
    }
}

impl<K, V> IntoIterator for OrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
