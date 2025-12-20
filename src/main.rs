use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use aes_gcm::{
    Aes256Gcm,
    Key,
    Nonce,
    aead::{Aead, KeyInit}, // Or `Aes128Gcm`
};
use eframe::egui::{
    self, CentralPanel, ComboBox, Context, FontFamily, FontId, RichText, ScrollArea, TextStyle,
    TopBottomPanel, UiKind,
};
use rusqlite::Connection;
const DB_PATH: &str = "_pmdb.db";
const AES_KEY: &str = "CornbreadCornbreadCornbreadCornb"; // 32 chars

mod db_utils;
mod models;
mod utils;
use crate::{
    db_utils::{add_entries, create_db, get_creds, get_current_accounts},
    models::{Account, Credential, CredentialDetails, CredentialInput},
};

#[derive(Default)]
struct App {
    conn: Option<Arc<Mutex<Connection>>>,
    accounts: Vec<Account>,
    name_input: String,
    password_input: String,
    description_input: String,
    selected_value: Option<usize>, // Index in Vec
    cred: Option<Credential>,
    // Boolean UI State Fields
    show_preferences_dialog: bool,
    show_popup: bool,
    show_edit_dialog: bool,
    popup_start_time: Option<Instant>,
}



impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        set_styles(ctx);
        self.show_top_bar(ctx);
        println!("Updating");

        CentralPanel::default().show(ctx, |ui| {
            self.show_account_form(ui);
            ui.add_space(20.0);
            self.show_combo_box(ui);
            self.handle_state_dialogs(ctx);
            if let Some(cred) = &self.cred.clone() {
                ui.separator();
                ui.small(cred.name.clone());
                ui.small(cred.description.clone().unwrap_or_default());
                let key = Key::<Aes256Gcm>::from_slice(AES_KEY.as_bytes());
                let cipher = Aes256Gcm::new(&key);
                println!("After cipher");
                let unwrapped = &cred.nonce.clone().unwrap();
                let nonce = Nonce::from_slice(&unwrapped);
                let res = cipher.decrypt(&nonce, cred.password_crypto.as_ref());
                match res {
                    Ok(bytes) => {
                        if self.show_popup {
                            // println!("Show Popup True");
                            self.display_popup(ctx, &bytes, cred);
                        }

                        // ScrollArea::vertical().show(ui, |ui| {
                        //     ui.small(cred.details.updated_at.clone().to_string());
                        //     ui.separator();
                        //     ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        //         ui.heading(
                        //             egui::RichText::new(String::from_utf8(bytes.into()).unwrap())
                        //                 .color(egui::Color32::RED),
                        //         );
                        //     });
                        // });
                    }
                    Err(e) => println!("{}", e),
                }
            }
        });
    }
}

// fn load_init_sql() -> std::io::Result {
//     fs::read_to_string("./init.sql")
// }

fn main() -> Result<(), eframe::Error> {
    // let (background_event_sender, background_event_receiver) = channel::();
    // let (event_sender, event_receiver) = channel::();
    // ...
    // std::thread::spawn(move || {
    //     while let Ok(event) = background_event_receiver.recv() {
    //         let sender = event_sender.clone();
    //         handle_events(event, sender);
    //     }
    // });
    let conn = Connection::open(DB_PATH).unwrap();
    let _ = create_db(&conn);
    println!("DB Created");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_resizable(true)
            .with_inner_size([620.0, 440.0])
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    // eframe::run_native("app_name", options, Box::new(|_cc| Ok(Box::<App>::default())))
    eframe::run_native(
        "Password Manager",
        options,
        Box::new(|_cc| {
            Ok(Box::new(App {
                conn: Some(Arc::new(Mutex::new(conn))),
                ..Default::default()
            }))
        }),
    )
}

fn set_styles(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Monospace)),
        (TextStyle::Body, FontId::new(18.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(14.0, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
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

impl App {
    // fn handle_gui_events(&mut self) {
    //     while let Ok(event) = self.event_receiver.try_recv() {
    //         match event {
    //             Event::SetPetImage(pet_image) => {
    //                 self.app_state.pet_image = pet_image;
    //             }
    //             Event::SetSelectedPet(pet) => self.app_state.selected_pet = pet,
    //             Event::SetPets(pets) => {
    //                 if let Some(ref selected_pet) = self.app_state.selected_pet {
    //                     if !pets.iter().any(|p| p.id == selected_pet.id) {
    //                         self.app_state.selected_pet = None;
    //                     }
    //                 }
    //                 self.app_state.pets = pets;
    //             }
    //             _ => (),
    //         };
    //     }
    // }
    fn handle_state_dialogs(&mut self, ctx: &Context) {
    if self.show_edit_dialog {
        self.edit_dialog(ctx);
    }
    if self.show_preferences_dialog {
        self.preferences_dialog(ctx);
    }
}
    fn show_top_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Edit Credential").clicked() {
                        self.show_edit_dialog = true;
                        println!("Set Edit to True");
                        ui.close_kind(UiKind::Menu)
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("Preferences").clicked() {
                        self.show_preferences_dialog = true;
                        ui.close_kind(UiKind::Menu)
                    }
                })
            })
        });
    }
    fn edit_dialog(&mut self, ctx: &Context) {
        let mut accounts: Vec<Account> = vec![];
        if let Some(conn) = &self.conn {
            let res = get_current_accounts(conn.clone());
            match res {
                Ok(accts) => accounts = accts,
                Err(e) => println!("{}", e),
            }
        } else {
            print!("No Conn");
        }
        self.accounts = accounts.clone();
        egui::Window::new("Edit Credential")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("Edit Credential").size(14.0));
                for acct in accounts {
                    if ui.button(&acct.name).clicked() {
                        println!("Wanting to edit {}", acct.name);
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.show_edit_dialog = false; // Close dialog
                }
                // Add any other elements here
            });
    }
    fn preferences_dialog(&mut self, ctx: &Context) {
        egui::Window::new("Preferences")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("Preferences").size(14.0));
                if ui.button("Cancel").clicked() {
                    self.show_preferences_dialog = false; // Close dialog
                }
            });
    }
    fn show_popup(&mut self) {
        self.show_popup = true;
        self.popup_start_time = Some(Instant::now());
        // Schedule the first check for hiding after 10s
        // self.ctx.request_repaint_after(Duration::from_secs(10)); // (This needs context access)
    }
    fn display_popup(&mut self, ctx: &Context, bytes: &Vec<u8>, cred: &Credential) {
        let mut remaining: Duration = Duration::from_secs(0);
        if let Some(start_time) = self.popup_start_time {
            if Instant::now().duration_since(start_time) >= Duration::from_secs(10) {
                println!("Setting popups to false");
                self.show_popup = false;
                self.popup_start_time = None; // Reset timer
            } else {
                println!("Time not up yet");
                remaining = Duration::from_secs(10) - Instant::now().duration_since(start_time)
                // Keep requesting repaint until time is up
                // ctx.request_repaint_after(Duration::from_millis(100)); // Check more frequently
            }
        }
        let str = format!("Remaining: {}", remaining.as_secs());
        egui::Window::new(str)
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                if let Ok(str) = String::from_utf8(bytes.to_vec()) {
                    ui.label(RichText::new(cred.name.clone()).size(20.0));
                    ui.label(RichText::new(str).size(20.0));
                } else {
                    ui.label(RichText::new("Could not parse password").size(20.0));
                }
                // Add any other elements here
            });

    }
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
                        let input_vec = vec![CredentialInput {
                            name: self.name_input.clone(),
                            password: self.password_input.clone(),
                            description: self.description_input.clone(),
                        }];
                        dbg!(&input_vec);
                        match &self.conn {
                            Some(conn) => {
                                println!("We have a conn!");
                                let _ = add_entries(&conn, input_vec);
                                // self.accounts.push((self.name_input.clone(), self.password_input.clone()));
                                self.name_input.clear();
                                self.password_input.clear();
                                self.description_input.clear();
                            }
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
        let mut accounts: Vec<Account> = vec![];
        if let Some(conn) = &self.conn {
            let res = get_current_accounts(conn.clone());
            match res {
                Ok(accts) => accounts = accts,
                Err(e) => println!("{}", e),
            }
        } else {
            print!("No Conn");
        }
        self.accounts = accounts;
        ComboBox::from_label("Select Account")
            .selected_text(if let Some(index) = self.selected_value {
                if let Some(acc) = self.accounts.get(index) {
                    &acc.name
                } else {
                    "Select me"
                }
            } else {
                "Select me"
            })
            .show_ui(ui, |ui| {
                for (i, acc) in self.accounts.clone().iter().enumerate() {
                    if ui
                        .selectable_value(
                            &mut self.selected_value, // What it is now
                            Some(i), // What selected value will be when this is clicked
                            &acc.name,
                        )
                        .clicked()
                    {
                        if let Some(acc) = self.accounts.clone().get(i) {
                            // Fetch whatever details to display upon user selection
                            // get_feed();
                            self.show_popup(); // Move this somewhere better
                            match &self.conn {
                                Some(conn) => {
                                    match get_creds(&conn, &acc.name) {
                                        Ok(cred) => self.cred = Some(cred),
                                        Err(e) => println!("{}", e.to_string()),
                                    };
                                }
                                None => println!("No Conn"),
                            }
                        }
                    }
                }
            });
    }
}
