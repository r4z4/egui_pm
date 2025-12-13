use std::sync::{Arc, Mutex};

use eframe::egui::{self, CentralPanel, ComboBox, Context, FontFamily, FontId, ScrollArea, TextStyle, TopBottomPanel};
use rusqlite::{Connection, Row};
use chrono::{DateTime, Utc};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key // Or `Aes128Gcm`
};
const DB_PATH: &str = "./db/_pmdb.db3";
const AES_KEY: &str = "Cornbread";
#[derive(Default)]
struct CredentialDetails {
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[derive(Default)]
struct Credential {
    name: String,
    password: String,
    description: Option<String>,
    details: CredentialDetails
}

#[derive(Default)]
struct App {
    conn: Option<Arc<Mutex<Connection>>>,
    accounts: Vec<(String, String)>,
    name_input: String,
    password_input: String,
    description_input: String,
    selected_value: Option<usize>, // Index in Vec
    cred: Option<Credential>
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        set_styles(ctx);
        show_top_bar(ctx);
        CentralPanel::default().show(ctx, |ui| {
            self.show_account_form(ui);
            self.show_combo_box(ui);
            if let Some(cred) = &self.cred {
                ui.separator();
                ui.heading(cred.name.clone());
                ui.label(cred.description.clone().unwrap_or_default());
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading(cred.details.updated_at.clone().to_string());
                    // for item in cred.details {
                    //     // Display Account details
                    //     ui.heading(item.updated_at.unwrap_or("Never Updated"));
                    // }
                });
            }
            // Debug code to view accounts for now. Clean this up.
            // for acc in &self.accounts {
            //     ui.heading(&acc.0); // Name
            //     ui.heading(&acc.1); // Pwd
            // }
        });
    }
}

fn main() -> Result<(), eframe::Error> {

    let conn = Connection::open(DB_PATH).unwrap();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_resizable(true).with_inner_size([620.0, 440.0]).with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    // eframe::run_native("app_name", options, Box::new(|_cc| Ok(Box::<App>::default())))
    eframe::run_native("app_name", options, Box::new(|_cc| Ok(Box::new(
        App {
            conn: Some(Arc::new(Mutex::new(conn))), 
            ..Default::default()
        }
    ))))
}

fn set_styles(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(30.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(22.0, FontFamily::Monospace)),
        (TextStyle::Body, FontId::new(18.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(14.0, FontFamily::Monospace)),
    ].into();
    ctx.set_style(style);
}

fn show_top_bar(ctx: &Context) {
    TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                }
            })
        })
    });
}

// Function to make HTTP req to get RSS data from internet
// Not using but keeping for reference
// fn get_feed(url: &str) -> Result<Channel, Box<dyn std::error::Error>> {
//     let content = reqwest::blocking::get(url)?.bytes()?;
//     let channel = Channel::read_from(&content[..])?;
//     Ok(channel)
// }
// This is the function we'd use above in ComboBox to fire when user selects an option.
// We will use a DB fetch instead to get Creds from DB (or wherever they are stored)
fn get_creds(conn: &Arc<Mutex<Connection>>, acc: &str) -> Result<Credential, Box<dyn std::error::Error>> {
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT id, name, password_crypto, description, updated_at, created_at
                    FROM credential
                    WHERE name = :name";
    let mut stmt = conn.prepare(select_sql)?;
    let mut rows = stmt.query(&[(":name", acc)])?;

    let mut final_cred: Credential = Credential::default();
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let cred = cred_from_row(row);
        final_cred = cred;
        println!("->> name: {name}");
        println!("->>  row: {row:?}");
    }
    
    // let now: DateTime<Utc> = Utc::now();
    // let details = CredentialDetails { updated_at: now, created_at: now };
    // let cred = Credential {
    //     name: "Wells Fargo".to_string(),
    //     password: "Blouse".to_string(),
    //     description: Some("Wells Fargo Website".to_string()),
    //     details: details,
    // };
    Ok(final_cred)
}

fn cred_from_row(row: &Row) -> Credential {
    let details = CredentialDetails { 
        updated_at: row.get(4).expect("Error"), 
        created_at: row.get(5).expect("Error") 
    };
    Credential {
        name: row.get(1).expect("Error"),
        password: row.get(2).expect("Error"),
        description: row.get(3).expect("Error"),
        details: details
    }
}

fn create_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS credentials (
          credential_id INTEGER PRIMARY KEY,
          name TEXT NOT NULL,
          password_crypto TEXT,
          description TEXT,
          updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
          created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) STRICT",
        (), // Empty list of params
    )?;
    Ok(())
}

fn build_db_credential(input: &CredentialInput) -> Credential {
    // Transformed from a byte array:
    let key = Key::<Aes256Gcm>::from_slice(AES_KEY.as_bytes());
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let now: DateTime<Utc> = Utc::now();    
    let details = CredentialDetails {
        updated_at: now,
        created_at: now,
    };
    let ciphertext = cipher.encrypt(&nonce, input.password.as_ref()).unwrap();
    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();
    assert_eq!(&plaintext, b"plaintext message");
    let stored = String::from_utf8(ciphertext).expect("Invalid UTF-8");
    Credential {
        name: input.name.clone(),
        password: stored,
        description: Some(input.description.clone()),
        details: details
    }
}


fn add_entries(conn: &Arc<Mutex<Connection>>, input_vec: Vec<CredentialInput>) -> Result<(), rusqlite::Error> {
    let now: DateTime<Utc> = Utc::now();
    let conn = conn.lock().unwrap();
    for cred in input_vec.iter() {
        let db_cred = build_db_credential(cred);
        // let org_id = if idx % 2 == 0 { Some(org_id) } else { None };
        conn.execute(
            "INSERT INTO credential (name, password_crypto, description. updated_at, created_at)
            VALUES (?1, ?2, ?3)",
            (&db_cred.name, &db_cred.password, &db_cred.description, now, now),
        )?;
    }
    Ok(())
}

struct CredentialInput {
    name: String,
    password: String,
    description: String,
}

impl App {
    fn show_account_form(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("New Account", |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut self.name_input);
                ui.label("Password");
                ui.text_edit_singleline(&mut self.password_input);
                ui.label("Description");
                ui.text_edit_singleline(&mut self.description_input);
                ui.horizontal(|ui| {
                    if ui.button("Submit").clicked() {
                        let input_vec = vec!(
                            CredentialInput{name: self.name_input.clone(), password: self.password_input.clone(), description: self.description_input.clone()});
                        match &self.conn {
                            Some(conn) => {
                                let _ = add_entries(&conn, input_vec);
                                // self.accounts.push((self.name_input.clone(), self.password_input.clone()));
                                self.name_input.clear();
                                self.password_input.clear();
                                self.description_input.clear();
                            },
                            None => println!("No Conn"),
                        }
                    }                        
                    if ui.button("Clear").clicked() {
                        self.name_input.clear();
                        self.password_input.clear();
                        self.description_input.clear();
                    }
                });
            });
        });
    }
    fn show_combo_box(&mut self, ui: &mut egui::Ui) {
        ComboBox::from_label("Select Account")
                .selected_text(if let Some(index) = self.selected_value {
                        if let Some(acc) = self.accounts.get(index) {
                            &acc.0
                        } else {
                            "Select me"
                        }
                } else {
                        "Select me"
                    }
            ).show_ui(ui, |ui| {
                for (i, acc) in self.accounts.iter().enumerate() {
                        if ui.selectable_value(
                            &mut self.selected_value, // What it is now
                            Some(i), // What selected value will be when this is clicked
                            &acc.0).clicked() {
                            if let Some(acc) = self.accounts.get(i) {
                                // Fetch whatever details to display upon user selection
                                // get_feed();
                                match &self.conn {
                                    Some(conn) => {
                                        match get_creds(&conn, &acc.0) {
                                            Ok(cred) => self.cred = Some(cred),
                                            Err(e) => println!("{}", e.to_string())
                                        };
                                    },
                                    None => println!("No Conn"),
                                }
                            }
                        }
                    }
            });
    }
}
