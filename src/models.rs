use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use eframe::egui::{self, Color32, FontFamily};
use rusqlite::Connection;

use crate::forms::ColorScheme;

pub enum Event {
    SetCreds(Vec<Credential>),
    GetCredFromDB(egui::Context, Arc<Mutex<Connection>>, String, i32),
    SetSelectedCred(Option<Credential>),
    InsertCredToDB(egui::Context, Arc<Mutex<Connection>>, CredentialInput, i32),
    DeleteCredFromDB(egui::Context, Arc<Mutex<Connection>>, String, i32),
}

#[derive(Debug)]
pub struct CredentialInput {
    pub id: Option<i32>, // Edits
    pub user_id: i32,
    pub name: String,
    pub password: String,
    pub description: String,
}

#[derive(Clone, Debug, Default)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub pin: String,
    pub preferences: UserPreference,
}

#[derive(Clone, Debug, Default)]
pub struct UserPreferenceInput {
    pub user_id: i32,
    pub font_family: FontFamily,
    pub color_scheme: ColorScheme,
    pub font_size: f32,
}

#[derive(Clone, Debug, Default)]
pub struct UserPreference {
    // pub user_id: i32,
    pub font_family: FontFamily,
    pub color_scheme: ColorScheme,
    pub font_size: f32,
}

#[derive(Clone, Debug)]
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
    pub id: Option<i32>, // Edits
    pub user_id: i32,
    pub name: String,
    pub password_crypto: Vec<u8>,
    pub nonce: Option<Vec<u8>>,
    pub description: Option<String>,
    pub details: CredentialDetails
}

