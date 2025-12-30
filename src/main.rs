#![windows_subsystem = "windows"] // Tell Windows to run this as a pure GUI app

use std::{
    fs::OpenOptions, sync::{Arc, Mutex}, time::{Duration, Instant}
};

use aes_gcm::{
    Aes256Gcm,
    Key,
    Nonce,
    aead::{Aead, KeyInit}, // Or `Aes128Gcm`
};
use catppuccin_egui::Theme;
use eframe::{
    egui::{
        self, CentralPanel, Color32, ComboBox, Context, FontFamily, FontId, RichText, TextStyle,
        TopBottomPanel, UiKind,
    },
    epaint::{self},
};
use rusqlite::Connection;
const DB_PATH: &str = "_pmdb.db";

mod db_utils;
mod forms;
mod models;
mod utils;
use crate::{
    db_utils::{
        create_db, get_creds, get_current_accounts, get_current_users, save_pin_to_db, user_from_id,
    },
    forms::{AccountForm, ColorScheme, PreferencesForm},
    models::{Account, Credential, CredentialDetails, User},
};
use dotenvy::dotenv;
use std::env;

#[cfg(target_os = "windows")]
use is_elevated::is_elevated;

#[derive(Default)]
struct AppDisplays {
    show_auth_pin_entry: bool,
    show_admin_setup_menu: bool,
    show_admin_reset_menu: bool,
    show_preferences_dialog: bool,
    show_credential_popup: bool,
    show_invalid_password: bool,
    show_edit_dialog: bool,
    popup_start_time: Option<Instant>,
}

#[derive(Default)]
struct AppForms {
    preferences_form: PreferencesForm,
    account_form: AccountForm,
}

#[derive(Default)]
struct App {
    conn: Option<Arc<Mutex<Connection>>>,
    accounts: Vec<Account>,
    users: Vec<User>,
    selected_value: Option<usize>, // Index in Vec
    selected_user: Option<usize>,
    cred: Option<Credential>,
    pin: String,
    login_pin: String,
    account_edit: Option<String>,
    // authenticated: bool, // Just use current_user presence
    current_user: Option<User>,
    login_user_id: Option<i32>,
    app_displays: AppDisplays,
    forms: AppForms,
}

fn decrypt(cred: &Credential) -> String {
    let aes_key: &str = &env::var("AES_KEY").expect("AES_KEY must be set in .env file");
    let key = Key::<Aes256Gcm>::from_slice(aes_key.as_bytes());
    let cipher = Aes256Gcm::new(&key);
    println!("After cipher");
    let unwrapped = &cred.nonce.clone().unwrap();
    let nonce = Nonce::from_slice(&unwrapped);
    let res = cipher.decrypt(&nonce, cred.password_crypto.as_ref());
    match res {
        Ok(bytes) => {
            if let Ok(str) = String::from_utf8(bytes.to_vec()) {
                str
            } else {
                println!("Error Decrypting from Res");
                "".to_string()
            }
        }
        Err(e) => {
            println!("Error Decrypting: {}", e);
            "".to_string()
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "linux")]
        if is_root::is_root() {
            println!("You are running this program as an root");
            self.admin_menu(ctx); // No bool for this as it just always shows for now
            if self.app_displays.show_admin_reset_menu {
                self.admin_setup_menu(ctx);
            } else if self.app_displays.show_admin_setup_menu {
                self.admin_setup_menu(ctx);
            } else {
                todo!();
            }
        };
        #[cfg(target_os = "windows")]
        if is_elevated::is_elevated() {
            println!("You are running this program as an admin");
            self.admin_menu(ctx); // No bool for this as it just always shows for now
            if self.show_admin_reset_menu {
                self.admin_setup_menu(ctx);
            } else if self.show_admin_setup_menu {
                self.admin_setup_menu(ctx);
            } else {
                todo!();
            }
        };
        set_styles(ctx, self.current_user.clone());
        self.show_top_bar(ctx);
        println!("Updating");
        CentralPanel::default().show(ctx, |ui| {
            if let Some(_current_user) = &self.current_user {
                App::account_form(self, ui, None);
                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);
                
                #[cfg(target_os = "windows")]
                if is_elevated::is_elevated() {
                    ui.small("Running as Admin");
                };
                
                // ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                //     self.show_combo_box(ui);
                // });
                
                self.combo_box(ui);
                self.handle_state_dialogs(ctx);
                if let Some(cred) = &self.cred.clone() {
                    ui.separator();
                    let pw_str = decrypt(cred);
                    // ui.small(cred.name.clone());
                    // ui.small(cred.description.clone().unwrap_or_default());
                    if self.app_displays.show_credential_popup {
                        self.credential_popup(ctx, pw_str, cred);
                    }
                    self.show_popup_timer(ctx, ui);
                }
            } else {
                self.auth_screen(ctx, ui);
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
    dotenv().ok();
    // let conn = Connection::open(DB_PATH).unwrap();
    let data_dir = "data";
    let db_path = format!("{}/_pmdb.db", data_dir);

    // Create the data directory if it doesn't exist
    std::fs::create_dir_all(data_dir).expect("Failed to create data directory");

    let open_options = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&db_path);
    let _res = match open_options {
        Ok(_path) => {

            println!("DB Created");
        }
        Err(e) => {
            eprintln!("Error opening database: {}", e);
        }
    };
    let conn = Connection::open(db_path).unwrap();
    let _ = create_db(&conn);
    // This results in an error: `Trying to write to read_only DB`
    // TODO: find out how to init in different directory
    // Make DB file hidden. On UNIX, dotfiles already hidden, Windows need to set attribute.
    // let path = Path::new(DB_PATH);
    // let res = hf::hide(&path);
    // match res {
    //     Ok(res) => println!("Success: {:?}", res),
    //     Err(e) => println!("Error: {}", e),
    // };
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

fn get_theme(cs: ColorScheme) -> Theme {
    dbg!(&cs);
    match cs {
        ColorScheme::Light => catppuccin_egui::LATTE,
        ColorScheme::Dark => catppuccin_egui::MOCHA,
        _ => catppuccin_egui::FRAPPE,
    }
}

fn set_styles(ctx: &Context, current_user: Option<User>) {
    dbg!(&current_user);
    let font_family = {
        if let Some(user) = &current_user {
            user.preferences.font_family.clone()
        } else {
            FontFamily::Monospace
        }
    };
    let theme = {
        if let Some(user) = current_user {
            get_theme(user.preferences.color_scheme)
        } else {
            catppuccin_egui::LATTE
        }
    };
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, font_family.clone())),
        (TextStyle::Button, FontId::new(14.0, font_family.clone())),
        (TextStyle::Body, FontId::new(18.0, font_family.clone())),
        (TextStyle::Small, FontId::new(14.0, font_family.clone())),
    ]
    .into();
    ctx.set_style(style);
    catppuccin_egui::set_theme(&ctx, theme);
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
        if self.app_displays.show_edit_dialog {
            self.edit_dialog(ctx);
        }
        if self.app_displays.show_preferences_dialog {
            self.preferences_dialog(ctx);
        }
    }
    fn show_popup_timer(&mut self, ctx: &Context, _ui: &mut egui::Ui) {
        let str: String = {
            if self.app_displays.show_credential_popup {
                if let Some(start_time) = self.app_displays.popup_start_time {
                    if Instant::now().duration_since(start_time) >= Duration::from_secs(10) {
                        "Times up!".to_string()
                    } else {
                        println!("Time not up yet");
                        let remaining =
                            Duration::from_secs(10) - Instant::now().duration_since(start_time);
                        format!("Remaining: {}", remaining.as_secs())
                    }
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            }
        };
        TopBottomPanel::bottom("timer").show(ctx, |ui| {
            ui.small(str);
        });
    }
    fn show_top_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.add_enabled(self.current_user.is_some(), egui::Button::new("Edit Credential")).clicked() {
                        self.app_displays.show_edit_dialog = true;
                        println!("Set Edit to True");
                        ui.close_kind(UiKind::Menu)
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.add_enabled(self.current_user.is_some(), egui::Button::new("Preferences")).clicked() {
                        self.app_displays.show_preferences_dialog = true;
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
                if let Some(acct_name) = &self.account_edit {
                    println!("Wanting to edit {}", acct_name);
                    if let Some(conn) = &self.conn {
                        if let Some(user) = &self.current_user {
                            let cred_res = get_creds(&conn.clone(), &acct_name, user.id);
                            match cred_res {
                                Ok(cred) => {
                                    App::account_form(self, ui, Some(cred));
                                }
                                Err(_e) => {
                                    println!("Error");
                                }
                            }
                        }
                    }
                } else {   
                    ui.label(RichText::new("Edit Credential").size(14.0));
                    for acct in accounts {
                        if ui.button(&acct.name).clicked() {
                            self.account_edit = Some(acct.name);
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.app_displays.show_edit_dialog = false; // Close dialog
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
                App::settings_form_two(self, ui);
                if ui.button("Cancel").clicked() {
                    self.app_displays.show_preferences_dialog = false; // Close dialog
                }
            });
    }

    fn invalid_password(&mut self, ctx: &Context) {
        egui::Window::new("Preferences")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("Invalid Password").size(14.0));
                if ui.button("Try Again").clicked() {
                    self.app_displays.show_invalid_password = false; // Close dialog
                }
            });
    }

    // #[cfg(target_os = "windows")]
    fn admin_setup_menu(&mut self, ctx: &Context) {
        egui::Window::new("Set up account PIN")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("Please set a 4 digit PIN.").size(14.0));
                ui.horizontal(|ui| {
                    // Limit to 4 characters, only allow digits, single line
                    ui.add(
                        egui::TextEdit::singleline(&mut self.pin)
                            .char_limit(4)
                            .hint_text("1234"),
                    );
                });
                if ui.button("Submit").clicked() {
                    // Save PIN to DB & Show Message
                    match &self.conn {
                        Some(conn) => {
                            println!("We have a conn!");
                            let _ = save_pin_to_db(&conn, self.pin.clone());
                        }
                        None => println!("No Conn"),
                    }
                    ui.small("You can now use this PIN to access and use Password Manager");
                    self.app_displays.show_preferences_dialog = false; // Close dialog
                }
                if ui.button("Cancel").clicked() {
                    self.app_displays.show_preferences_dialog = false; // Close dialog
                }
                // // Validate: Only allow numeric input
                // if !self.pin.chars().all(|c| c.is_digit(10)) {
                //     self.pin.retain(|c| c.is_digit(10));
                // }
            });
    }

    // #[cfg(target_os = "windows")]
    // fn admin_reset_menu(&mut self, ctx: &Context) {
    //     egui::Window::new("Reset")
    //         .collapsible(false)
    //         .movable(false)
    //         .show(ctx, |ui| {
    //             ui.label(RichText::new("Preferences").size(14.0));
    //             if ui.button("Cancel").clicked() {
    //                 self.show_preferences_dialog = false; // Close dialog
    //             }
    //         });
    // }

    // #[cfg(target_os = "windows")]
    fn admin_menu(&mut self, ctx: &Context) {
        let is_set_up = false;
        if is_set_up {
            egui::Window::new("Already set up")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.small("You are already set up to use password manager. If you would like to reset privileges, click reset below.");
                if ui.button("Reset").clicked() {
                    self.app_displays.show_admin_reset_menu = true; // Close dialog
                }
            });
        } else {
            self.app_displays.show_admin_setup_menu = true;
        }
    }
    fn credential_popup(&mut self, ctx: &Context, pw_str: String, cred: &Credential) {
        egui::Window::new("Temporary Access: 10 seconds")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new(format!("Username: {}", cred.name.clone())).size(14.0))
                    .on_hover_cursor(egui::CursorIcon::Text);
                ui.label(RichText::new(format!("Password: {}", pw_str)).size(14.0))
                    .on_hover_cursor(egui::CursorIcon::Text);
                if ui
                    .button("Close")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.app_displays.show_credential_popup = false;
                }
            });
        if let Some(start_time) = self.app_displays.popup_start_time {
            if Instant::now().duration_since(start_time) >= Duration::from_secs(10) {
                println!("Setting popups to false");
                self.app_displays.show_credential_popup = false;
                self.app_displays.popup_start_time = None; // Reset timer
            } else {
                println!("Time not up yet");
                // Keep requesting repaint until time is up
                // ctx.request_repaint_after(Duration::from_millis(100)); // Check more frequently
            }
        }
    }
    fn auth_screen(&mut self, ctx: &Context, _ui: &mut egui::Ui) {
        let my_frame = egui::containers::Frame {
            shadow: eframe::epaint::Shadow {
                offset: [0, 0],
                blur: 0,
                spread: 0,
                color: Color32::BLACK,
            },
            fill: Color32::LIGHT_BLUE,
            stroke: egui::Stroke::new(2.0, Color32::BLACK),
            inner_margin: crate::epaint::Margin {
                left: 10,
                right: 10,
                top: 10,
                bottom: 10,
            },
            corner_radius: egui::CornerRadius {
                nw: 1,
                ne: 1,
                sw: 1,
                se: 1,
            },
            outer_margin: crate::epaint::Margin {
                left: 10,
                right: 10,
                top: 10,
                bottom: 10,
            },
        };
        egui::CentralPanel::default()
            .frame(my_frame)
            .show(ctx, |ui| {
                if self.app_displays.show_auth_pin_entry {
                    self.auth_pin_entry(ctx);
                } else {
                    let mut users: Vec<User> = vec![];
                    if let Some(conn) = &self.conn {
                        let res = get_current_users(conn.clone());
                        match res {
                            Ok(usrs) => users = usrs,
                            Err(e) => println!("{}", e),
                        }
                    } else {
                        print!("No Conn");
                    }
                    self.users = users;
                    ui.small("Welcome back. Please select user.");
                    ComboBox::from_label("Username")
                        .selected_text(if let Some(index) = self.selected_user {
                            if let Some(usr) = self.users.get(index) {
                                &usr.username
                            } else {
                                "Select One"
                            }
                        } else {
                            "Select One"
                        })
                        .show_ui(ui, |ui| {
                            for (i, usr) in self.users.clone().iter().enumerate() {
                                if ui
                                    .selectable_value(
                                        &mut self.selected_user, // What it is now
                                        Some(i), // What selected value will be when this is clicked
                                        &usr.username,
                                    )
                                    .clicked()
                                {
                                    println!("Clicked");
                                    if let Some(usr) = self.users.clone().get(i) {
                                        println!("Some User");
                                        self.app_displays.show_auth_pin_entry = true;
                                        self.login_user_id = Some(usr.id);
                                        // ctx.request_repaint_after(std::time::Duration::from_secs(10));
                                    } else {
                                        println!("No User ?");
                                    }
                                }
                            }
                        });
                }
            });
    }
    fn auth_pin_entry(&mut self, ctx: &Context) {
        if let Some(login_user_id) = self.login_user_id {
            egui::Window::new("Enter PIN")
                .collapsible(false)
                .movable(false)
                .show(ctx, |ui| {
                    if self.app_displays.show_invalid_password {
                        self.invalid_password(ctx);
                    } else {
                        ui.label(RichText::new("Enter PIN").size(14.0));
                        ui.horizontal(|ui| {
                            // Limit to 4 characters, only allow digits, single line
                            ui.add(
                                egui::TextEdit::singleline(&mut self.login_pin)
                                    .char_limit(4)
                                    .hint_text("0000"),
                            );
                        });
                        if ui.button("Submit").clicked() {
                            // Check PIN

                            match &self.conn {
                                Some(conn) => {
                                    match user_from_id(&conn, login_user_id) {
                                        Ok(pin_user) => {
                                            if self.login_pin == pin_user.pin {
                                                self.current_user = Some(pin_user);
                                            } else {
                                                dbg!(&pin_user.pin);
                                                dbg!(&self.login_pin);
                                                println!("Invalid Password");
                                                self.app_displays.show_invalid_password = true;
                                            }
                                        }
                                        Err(e) => println!("{}", e.to_string()),
                                    };
                                }
                                None => println!("No Conn"),
                            }

                            ui.small("You can now use this PIN to access and use Password Manager");
                        }
                    }
                });
        } else {
            println!("No Login User ID");
        }
    }

    fn combo_box(&mut self, ui: &mut egui::Ui) {
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
                            self.app_displays.show_credential_popup = true;
                            self.app_displays.popup_start_time = Some(Instant::now());
                            let user_id = {
                                if let Some(current_user) = &self.current_user {
                                    current_user.id
                                } else {
                                    println!("No User Here");
                                    0
                                    // panic!("Need to have user here");
                                }
                            };
                            match &self.conn {
                                Some(conn) => {
                                    match get_creds(&conn, &acc.name, user_id) {
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
