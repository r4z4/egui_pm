use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use eframe::egui;
use rusqlite::Connection;

pub enum Event {
    SetCreds(Vec<Credential>),
    GetCredFromDB(egui::Context, Arc<Mutex<Connection>>, String),
    SetSelectedCred(Option<Credential>),
    InsertCredToDB(egui::Context, Arc<Mutex<Connection>>, CredentialInput),
    DeleteCredFromDB(egui::Context, Arc<Mutex<Connection>>, String),
}

#[derive(Debug)]
pub struct CredentialInput {
    pub name: String,
    pub password: String,
    pub description: String,
}

#[derive(Clone)]
pub struct Account {
    pub id: i32,
    pub name: String
}

#[derive(Clone, Default, Debug)]
pub struct CredentialDetails {
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Default, Debug)]
pub struct Credential {
    pub name: String,
    pub password_crypto: Vec<u8>,
    pub nonce: Option<Vec<u8>>,
    pub description: Option<String>,
    pub details: CredentialDetails
}

