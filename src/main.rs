#![windows_subsystem = "windows"] // Tell Windows to run this as a pure GUI app

use std::{
    fs::OpenOptions,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use catppuccin_egui::Theme;
use chrono::Local;
use eframe::{
    egui::{
        self, CentralPanel, Color32, ComboBox, Context, FontFamily, FontId, RichText, TextStyle,
        TopBottomPanel, UiKind,
    },
    epaint::{self},
};
use rusqlite::Connection;
const DB_PATH: &str = "_pmdb.db";
use std::io::Write;
mod db_utils;
mod forms;
mod models;
mod utils;
use crate::{
    db_utils::{create_db, get_creds, get_current_accounts, get_current_users},
    forms::{AccountForm, AccountSetupForm, ColorScheme, PreferencesForm},
    models::{Account, Credential, CredentialDetails, User},
    utils::decrypt,
};
use dotenvy::dotenv;

#[cfg(target_os = "windows")]
use is_elevated::is_elevated;

#[derive(Debug)]
struct AppStyles {
    window_title_font_size: f32,
    font_size: f32,
}

impl Default for AppStyles {
    fn default() -> Self {
        AppStyles {
            window_title_font_size: 12.0,
            font_size: 12.0,
        }
    }
}


#[derive(Default)]
struct AppDisplays {
    show_account_form: bool,
    show_auth_pin_entry: bool,
    show_admin_setup_menu: bool,
    show_admin_reset_menu: bool,
    show_preferences_dialog: bool,
    show_credential_popup: bool,
    show_invalid_password: bool,
    show_delete_warning: bool,
    show_delete_confirmation: bool,
    show_edit_dialog: bool,
    popup_start_time: Option<Instant>,
}

#[derive(Default)]
struct AppForms {
    preferences_form: PreferencesForm,
    account_form: AccountForm,
    pub account_setup_form: AccountSetupForm,
}

#[derive(Default)]
struct AppObjects {
    cred: Option<Credential>,
    account_edit: Option<String>,
    account_delete: Option<String>,
    login_user_id: Option<i32>,
    login_pin: String,
    pin: String,
}

#[derive(Default)]
struct AppSelections {
    selected_value: Option<usize>, // Index in Vec
    selected_user: Option<usize>,
}

#[derive(Default)]
struct App {
    conn: Option<Arc<Mutex<Connection>>>,
    current_user: Option<User>,
    objects: AppObjects,
    displays: AppDisplays,
    forms: AppForms,
    selections: AppSelections,
    styles: AppStyles,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "linux")]
        if is_root::is_root() {
            println!("You are running this program as an root");
            self.admin_menu(ctx); // No bool for this as it just always shows for now
            if self.displays.show_admin_reset_menu {
                self.admin_setup_menu(ctx);
            } else if self.displays.show_admin_setup_menu {
                self.admin_setup_menu(ctx);
            } else {
                todo!();
            }
        };
        #[cfg(target_os = "windows")]
        if is_elevated::is_elevated() {
            println!("You are running this program as an admin");
            self.admin_menu(ctx); // No bool for this as it just always shows for now
            if self.displays.show_admin_reset_menu {
                self.admin_setup_menu(ctx);
            } else if self.displays.show_admin_setup_menu {
                self.admin_setup_menu(ctx);
            } else {
                todo!();
            }
        };
        set_styles(ctx, self.current_user.clone());
        self.top_menu_bar(ctx);
        println!("Updating");
        CentralPanel::default().show(ctx, |ui| {
            if let Some(_current_user) = &self.current_user {
                App::account_form(self, ui, ctx, None);
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
                if let Some(cred) = &self.objects.cred.clone() {
                    let pw_str = decrypt(cred);
                    // ui.small(cred.name.clone());
                    // ui.small(cred.description.clone().unwrap_or_default());
                    if self.displays.show_credential_popup {
                        self.credential_popup(ctx, pw_str, cred);
                    }
                    self.popup_timer(ctx, ui);
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
    #[cfg(target_os = "macos")]
    let data_dir = "~/Library/Application Support/aalmp";
    #[cfg(target_os = "linux")]
    // let data_dir = "data";
    let data_dir = "~/.local/share/aalmp"; // Or ~/.config/aalmp
    #[cfg(target_os = "windows")]
    let data_dir = r"%APPDATA%\aalmp"; // or %LOCALAPPDATA%. (e.g., C:\Users\Username\AppData\Roaming\aalmp)

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
            .with_inner_size([500.0, 300.0])
            .with_min_inner_size([320.0, 240.0]),
        #[cfg(target_os = "macos")]
        run_and_return: false, // Stops 'Unexpectedly Quit' Message on Close
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
    match cs {
        ColorScheme::Light => catppuccin_egui::LATTE,
        ColorScheme::Dark => catppuccin_egui::MOCHA,
        _ => catppuccin_egui::FRAPPE,
    }
}

fn set_styles(ctx: &Context, current_user: Option<User>) {
    let (font_family, _font_size, theme) = {
        if let Some(user) = &current_user {
            (user.preferences.font_family.clone(), user.preferences.font_size.clone(), get_theme(user.preferences.color_scheme.clone()))
        } else {
            (FontFamily::Monospace, 12.0, catppuccin_egui::LATTE)
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
    // TODO Move to impl App so we can use mut self
    // // Set AppStyles
    // self.styles = AppStyles {
    //     font_size: font_size,
    //     window_title_font_size: 10.0,
    // }
}

fn create_env_file(input: String) {
    let env_path = Path::new("./.env");
    let res = OpenOptions::new()
        .read(true)
        .write(true)
        .append(true)
        .create(true)
        .open(&env_path);
    match res {
        Ok(mut env_file) => {
            println!(".env created. writing to.");
            let _ = writeln!(env_file, "AES_KEY={:?}", input);
        }
        Err(e) => {
            eprintln!("Error creating .env file: {}", e);
        }
    };
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
    fn window_header(&mut self, str: &str) -> RichText {
        RichText::new(str).size(self.styles.window_title_font_size)
    }

    fn handle_state_dialogs(&mut self, ctx: &Context) {
        if self.displays.show_edit_dialog {
            self.edit_dialog(ctx);
        }
        if self.displays.show_preferences_dialog {
            self.preferences_dialog(ctx);
        }
    }
    fn popup_timer(&mut self, ctx: &Context, _ui: &mut egui::Ui) {
        let time: u64 = if let Some(user) = &self.current_user {user.preferences.popup_time as u64} else {10};
        let str: String = {
            if self.displays.show_credential_popup {
                if let Some(start_time) = self.displays.popup_start_time {
                    if Instant::now().duration_since(start_time) >= Duration::from_secs(time) {
                        "Times up!".to_string()
                    } else {
                        let remaining =
                            Duration::from_secs(time) - Instant::now().duration_since(start_time);
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
    fn top_menu_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add_enabled(
                            self.current_user.is_some(),
                            egui::Button::new("Edit Credential"),
                        )
                        .clicked()
                    {
                        self.displays.show_edit_dialog = true;
                        println!("Set Edit to True");
                        ui.close_kind(UiKind::Menu)
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui
                        .add_enabled(
                            self.current_user.is_some(),
                            egui::Button::new("Preferences"),
                        )
                        .clicked()
                    {
                        self.displays.show_preferences_dialog = true;
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
        // self.current_user.accounts = accounts.clone();

        egui::Window::new("Edit Credential")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                if let Some(acct_name) = &self.objects.account_edit {
                    println!("Wanting to edit {}", acct_name);
                    if let Some(conn) = &self.conn {
                        if let Some(user) = &self.current_user {
                            let cred_res = get_creds(&conn.clone(), &acct_name, user.id);
                            match cred_res {
                                Ok(cred) => {
                                    App::account_form(self, ui, ctx, Some(cred));
                                }
                                Err(_e) => {
                                    println!("Error");
                                    // self.account_edit = None;
                                }
                            }
                        }
                    }
                } else {
                    ui.label(RichText::new("Edit Credential").size(12.0));
                    for acct in accounts {
                        if ui.button(&acct.name).clicked() {
                            self.objects.account_edit = Some(acct.name);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.displays.show_edit_dialog = false; // Close dialog
                        self.objects.account_edit = None;
                    }
                }
            });
    }
    fn preferences_dialog(&mut self, ctx: &Context) {
        egui::Window::new("Preferences")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("Preferences").size(14.0));
                App::settings_form(self, ui);
                if ui.button("Cancel").clicked() {
                    self.forms.preferences_form.in_edit = false;
                    self.displays.show_preferences_dialog = false; // Close dialog
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
                    self.displays.show_invalid_password = false; // Close dialog
                }
            });
    }

    // #[cfg(target_os = "windows")]
    fn admin_setup_menu(&mut self, ctx: &Context) {
        egui::Window::new("Set up account PIN")
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                self.account_setup_form(ui);
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
                    self.displays.show_admin_reset_menu = true; // Close dialog
                }
            });
        } else {
            self.displays.show_admin_setup_menu = true;
        }
    }
    fn credential_popup(&mut self, ctx: &Context, pw_str: String, cred: &Credential) {
        let time: u64 = if let Some(user) = &self.current_user {user.preferences.popup_time as u64} else {10};
        egui::Window::new(format!("Temporary Access: {} seconds", time))
            .collapsible(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.collapsing("Details", |ui| {
                    let dt_local = cred.details.updated_at.with_timezone(&Local);
                    if let Some(desc) = &cred.description {
                        ui.label(RichText::new(format!("Desc: {}", desc)).size(10.0))
                            .on_hover_cursor(egui::CursorIcon::Text);
                    }
                    ui.label(RichText::new(format!("Updated: {}", dt_local.format("%Y/%m/%d %H:%M"))).size(10.0));
                });                
                ui.label(RichText::new(format!("Username: {}", cred.name.clone())).size(14.0))
                    .on_hover_cursor(egui::CursorIcon::Text);
                ui.label(RichText::new(format!("Password: {}", pw_str)).size(14.0))
                    .on_hover_cursor(egui::CursorIcon::Text);
                if ui
                    .button("Close")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.displays.show_credential_popup = false;
                    self.selections.selected_value = None;
                }
            });
        if let Some(start_time) = self.displays.popup_start_time {
            if Instant::now().duration_since(start_time) >= Duration::from_secs(time) {
                println!("Setting popups to false");
                self.displays.show_credential_popup = false;
                self.displays.popup_start_time = None; // Reset timer
                self.selections.selected_value = None;
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
            fill: Color32::from_rgb(26, 0, 26),
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
                if self.displays.show_auth_pin_entry {
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
                    ui.small(RichText::new("Welcome back. Please select user.").color(Color32::WHITE));
                    ComboBox::from_id_salt(1)
                        .selected_text(if let Some(index) = self.selections.selected_user {
                            if let Some(usr) = users.get(index) {
                                &usr.username
                            } else {
                                "Select One"
                            }
                        } else {
                            "Select One"
                        })
                        .show_ui(ui, |ui| {
                            for (i, usr) in users.clone().iter().enumerate() {
                                if ui
                                    .selectable_value(
                                        &mut self.selections.selected_user, // What it is now
                                        Some(i), // What selected value will be when this is clicked
                                        &usr.username,
                                    )
                                    .clicked()
                                {
                                    println!("Clicked");
                                    if let Some(usr) = users.clone().get(i) {
                                        println!("Some User");
                                        self.displays.show_auth_pin_entry = true;
                                        self.objects.login_user_id = Some(usr.id);
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
        if let Some(login_user_id) = self.objects.login_user_id {
            egui::Window::new(self.window_header("Enter PIN"))
                .collapsible(false)
                .movable(false)
                .show(ctx, |ui| {
                    self.auth_pin_entry_form(ui, ctx, login_user_id);
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
        ui.small(RichText::new("Your Accounts"));
        ComboBox::from_id_salt("select_account")
            .selected_text(if let Some(index) = self.selections.selected_value {
                if let Some(acc) = accounts.get(index) {
                    &acc.name
                } else {
                    "Select me"
                }
            } else {
                "Select me"
            })
            .show_ui(ui, |ui| {
                for (i, acc) in accounts.clone().iter().enumerate() {
                    if ui
                        .selectable_value(
                            &mut self.selections.selected_value, // What it is now
                            Some(i), // What selected value will be when this is clicked
                            &acc.name,
                        )
                        .clicked()
                    {
                        if let Some(acc) = accounts.clone().get(i) {
                            self.displays.show_credential_popup = true;
                            self.displays.popup_start_time = Some(Instant::now());
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
                                        Ok(cred) => self.objects.cred = Some(cred),
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
