use std::path::Path;

use eframe::egui::{self, Color32, Context, FontFamily, Label, RichText};
use rand::{Rng, distr::Alphanumeric};

use crate::{
    App, create_env_file,
    db_utils::{
        add_entries, delete_cred, save_pin_to_db, update_entries, update_user_preferences,
        user_from_id,
    },
    models::{Credential, CredentialInput, UserPreferenceInput},
    utils::decrypt,
};

#[derive(Default)]
pub struct PreferencesForm {
    font_family: FontFamily,
    color_scheme: ColorScheme,
    font_size: f32,
    in_edit: bool,
    // font_family: String
}

#[derive(Default)]
pub struct AccountForm {
    id: Option<i32>,
    name: String,
    password: String,
    description: String,
    updating: bool,
    // font_family: String
}

#[derive(Default)]
pub struct AccountSetupForm {
    pub pin: String,
    pub aes_input: String,
}

#[derive(PartialEq, Clone, Debug, Default, Hash, Eq)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}


impl App {
    pub fn account_setup_form(&mut self, ui: &mut egui::Ui) {
        let env_path = Path::new("./.env");
        ui.label(RichText::new("Please set a 4 digit PIN.").size(12.0));
        ui.horizontal(|ui| {
            // Limit to 4 characters, only allow digits, single line
            ui.add(
                egui::TextEdit::singleline(&mut self.forms.account_setup_form.pin)
                    .char_limit(4)
                    .hint_text("1234"),
            );
        });
        if !env_path.exists() {
            ui.label(
                RichText::new("Please set a 32 character AES key (or choose auto-generate)")
                    .size(12.0),
            );
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.forms.account_setup_form.aes_input)
                        .char_limit(32)
                        .hint_text("12ab"),
                );
            });
            if ui.button("Auto-generate").clicked() {
                // let key = Aes256Gcm::generate_key(&mut OsRng);
                // let key_bytes: [u8; 32] = key.into();
                // let res = std::str::from_utf8(&key_bytes);
                // match res {
                //     Ok(str) => {
                //         let input = str.trim_end_matches(char::from(0)).to_string();
                //         self.forms.account_setup_form.aes_input = input;
                //     },
                //     Err(e) => println!("Failed to create ENV file. {}", e)
                // }
                let rng = rand::rng();
                let s: String = rng
                    .sample_iter(&Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect();
                self.forms.account_setup_form.aes_input = s;
            }
        }
        if ui.button("Submit").clicked() {
            if self.forms.account_setup_form.pin.is_empty() {
                ui.label(egui::RichText::new("PIN Cannot be empty").color(egui::Color32::RED));
            }
            if !env_path.exists() && self.forms.account_setup_form.aes_input.is_empty() {
                // TODO: Popup w/ warning about user needing to create their own .env file
                ui.label(egui::RichText::new("AES Cannot be empty").color(egui::Color32::RED));
            }
            // Save PIN to DB & Show Message
            match &self.conn {
                Some(conn) => {
                    println!("We have a conn!");
                    create_env_file(self.forms.account_setup_form.aes_input.clone());
                    let _ = save_pin_to_db(&conn, self.objects.pin.clone());
                }
                None => println!("No Conn"),
            }
            ui.small("You can now use this PIN to access and use Password Manager");
            self.displays.show_preferences_dialog = false; // Close dialog
        }
        if ui.button("Cancel").clicked() {
            self.displays.show_preferences_dialog = false; // Close dialog
        }
        // Validate: Only allow numeric input
        // if !self.pin.chars().all(|c| c.is_digit(10)) {
        //     self.pin.retain(|c| c.is_digit(10));
        // }
    }
    pub fn auth_pin_entry_form(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        login_user_id: i32,
    ) {
        if self.displays.show_invalid_password {
            self.invalid_password(ctx);
        } else {
            ui.label(RichText::new("Enter PIN").size(12.0));
            ui.horizontal(|ui| {
                // Limit to 4 characters, only allow digits, single line
                ui.add(
                    egui::TextEdit::singleline(&mut self.objects.login_pin)
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
                                dbg!(&pin_user);
                                if self.objects.login_pin == pin_user.pin {
                                    self.current_user = Some(pin_user);
                                } else {
                                    dbg!(&pin_user.pin);
                                    dbg!(&self.objects.login_pin);
                                    println!("Invalid Password");
                                    self.displays.show_invalid_password = true;
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
    }
    pub fn account_form(&mut self, ui: &mut egui::Ui, ctx: &Context, credential: Option<Credential>) {
        if self.displays.show_delete_confirmation {
            ui.small("Record has been removed. Hit Ok to return Home.");
            if ui.button("Ok").clicked() {
                self.displays.show_delete_warning = false;
                self.displays.show_delete_confirmation = false;
                self.displays.show_edit_dialog = false;
                self.displays.show_account_form = false;
            }
        } else {
            if self.displays.show_delete_warning {
                egui::Grid::new("my_grid").striped(false).show(ui, |ui| {
                    // self.dialogs.delete_warning(ui);
                    if let Some(user) = &self.current_user {
                        if let Some(acct_del) = &self.objects.account_delete {
                            ui.small(format!("Corfirm deletion of {}", &acct_del));
                            ui.end_row();
                            if ui.button("Confirm Delete").clicked() {
                                match &self.conn {
                                    Some(conn) => {
                                        println!("DELETING {} FROM DATABASE", &acct_del);
                                        let res = delete_cred(conn, acct_del, user.id);
                                        match res {
                                            Ok(_del) => println!("Success"),
                                            Err(e) => println!("Err: {}", e),
                                        }
                                    }
                                    None => println!("No Conn"),
                                }
                                self.displays.show_delete_confirmation = true;
                                self.displays.show_delete_warning = false;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.displays.show_delete_warning = false;
                    }
                    ui.end_row();
                });
            } else {
                let heading = if let Some(cred) = &credential {
                    cred.name.clone()
                } else {
                    "New Account".to_string()
                };
                if !self.forms.account_form.updating
                    && let Some(cred) = &credential
                {
                    self.forms.account_form.updating = true;
                    self.forms.account_form.id = cred.id.clone();
                    self.forms.account_form.name = cred.name.clone();
                    self.forms.account_form.password = decrypt(&cred);
                    self.forms.account_form.description =
                        cred.description.clone().unwrap_or("".to_string());
                }
                if self.displays.show_account_form {
                    egui::Window::new(self.window_header("New/Edit Credential"))
                        .collapsible(false)
                        .movable(false)
                        .show(ctx, |ui| {
                        ui.vertical_centered_justified(|ui| {
                            egui::Grid::new("form_grid")
                                .num_columns(2)
                                .spacing([8.0, 12.0])
                                .show(ui, |ui| {
                                    ui.label("Name");
                                    ui.text_edit_singleline(&mut self.forms.account_form.name);
                                    ui.end_row();
                                    ui.label("Password");
                                    ui.text_edit_singleline(&mut self.forms.account_form.password);
                                    ui.end_row();
                                    ui.label("Description");
                                    ui.text_edit_singleline(&mut self.forms.account_form.description);
                                    ui.end_row();
                                });
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.button("Submit").clicked() {
                                    let user_id = {
                                        if let Some(current_user) = &self.current_user {
                                            current_user.id
                                        } else {
                                            0
                                        }
                                    };
                                    let input_vec = vec![CredentialInput {
                                        id: self.forms.account_form.id,
                                        user_id: user_id,
                                        name: self.forms.account_form.name.clone(),
                                        password: self.forms.account_form.password.clone(),
                                        description: self.forms.account_form.description.clone(),
                                    }];
                                    dbg!(&input_vec);
                                    match &self.conn {
                                        Some(conn) => {
                                            println!("We have a conn!");
                                            // Just use presence of id on the form rather than updating bool
                                            if self.forms.account_form.updating {
                                                let _ = update_entries(&conn, input_vec);
                                            } else {
                                                let _ = add_entries(&conn, input_vec);
                                            }
                                            // self.accounts.push((self.name_input.clone(), self.password_input.clone()));
                                            self.forms.account_form.name.clear();
                                            self.forms.account_form.password.clear();
                                            self.forms.account_form.description.clear();
                                            self.objects.account_edit = None;
                                            self.forms.account_form.updating = false;
                                            self.displays.show_account_form = false;
                                        }
                                        None => println!("No Conn"),
                                    }
                                }
                                if ui.button("Reset").clicked() {
                                    self.forms.account_form.updating = false;
                                }
                                if ui.button("Clear").clicked() {
                                    self.forms.account_form.name.clear();
                                    self.forms.account_form.password.clear();
                                    self.forms.account_form.description.clear();
                                }
                                if let Some(cred) = &credential {
                                    if ui.button("Delete").clicked() {
                                        self.displays.show_delete_warning = true;
                                        self.forms.account_form.updating = false;
                                        self.objects.account_delete = Some(cred.name.clone());
                                    }
                                }
                                if ui.button("Cancel").clicked() {
                                    self.forms.account_form.name.clear();
                                    self.forms.account_form.password.clear();
                                    self.forms.account_form.description.clear();
                                    self.displays.show_edit_dialog = false;
                                    self.forms.account_form.updating = false;
                                    self.objects.account_edit = None;
                                    self.displays.show_account_form = false;
                                }
                            });
                        });
                    });
                } else {
                    if ui.button(&heading).clicked() {
                        self.displays.show_account_form = true;
                    }
                }
            }
        }
    }
    pub fn settings_form_two(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered_justified(|ui| {
            let mut radio = {
                if let Some(user) = &self.current_user {
                    user.preferences.font_family.clone()
                } else {
                    FontFamily::Monospace
                }
            };

            // ui.radio_value(&mut my_enum, Enum::First, "First");
            // ui.checkbox(&mut self.is_checked, "Option Enabled");
            if !self.forms.preferences_form.in_edit
                    && let Some(user) = &self.current_user
                {
                    self.forms.preferences_form.font_family = user.preferences.font_family.clone();
                    self.forms.preferences_form.color_scheme = user.preferences.color_scheme.clone();
                    self.forms.preferences_form.font_size = user.preferences.font_size;
                }
            // Font Family
            ui.horizontal(|ui| {
                ui.add(Label::new("Font Family"));
                ui.horizontal(|ui| {
                    // ui.radio_value(&mut radio, FontFamily::Monospace, "Monospace");
                    // ui.radio_value(&mut radio, FontFamily::Proportional, "Proportional");
                    if ui
                        .add(egui::RadioButton::new(
                            self.forms.preferences_form.font_family == FontFamily::Monospace,
                            "Monospace",
                        ))
                        .clicked()
                    {
                        println!("Mono");
                        self.forms.preferences_form.in_edit = true;
                        self.forms.preferences_form.font_family = FontFamily::Monospace
                    }
                    if ui
                        .add(egui::RadioButton::new(
                            self.forms.preferences_form.font_family == FontFamily::Proportional,
                            "Proportional",
                        ))
                        .clicked()
                    {
                        println!("Proportional");
                        self.forms.preferences_form.in_edit = true;
                        self.forms.preferences_form.font_family = FontFamily::Proportional
                    }
                    // ui.radio_value(radio, FontFamily::Name("serif"), "Custom");
                });
            });
            ui.end_row();
            // Color
            ui.horizontal(|ui| {
                ui.add(Label::new("Color Scheme"));
                ui.horizontal(|ui| {
                    // ui.radio_value(&mut radio, FontFamily::Monospace, "Monospace");
                    // ui.radio_value(&mut radio, FontFamily::Proportional, "Proportional");
                    if ui
                        .add(egui::RadioButton::new(
                            self.forms.preferences_form.color_scheme == ColorScheme::Light,
                            "Light",
                        ))
                        .clicked()
                    {
                        println!("Light");
                        self.forms.preferences_form.in_edit = true;
                        self.forms.preferences_form.color_scheme = ColorScheme::Light
                    }
                    if ui
                        .add(egui::RadioButton::new(
                            self.forms.preferences_form.color_scheme == ColorScheme::Dark,
                            "Dark",
                        ))
                        .clicked()
                    {
                        println!("Dark");
                        self.forms.preferences_form.in_edit = true;
                        self.forms.preferences_form.color_scheme = ColorScheme::Dark
                    }
                    // ui.radio_value(radio, FontFamily::Name("serif"), "Custom");
                });
            });
            ui.end_row();

            // Admin Color
            ui.horizontal(|ui| {
                ui.add(Label::new("Font Size"));
                ui.horizontal(|ui| {
                    // ui.radio_value(&mut radio, FontFamily::Monospace, "Monospace");
                    // ui.radio_value(&mut radio, FontFamily::Proportional, "Proportional");
                    if ui
                        .add(egui::RadioButton::new(
                            self.forms.preferences_form.font_size == 12.0,
                            "12",
                        ))
                        .clicked()
                    {
                        println!("Font 12");
                        self.forms.preferences_form.in_edit = true;
                        self.forms.preferences_form.font_size = 12.0;
                    }
                    if ui
                        .add(egui::RadioButton::new(
                            self.forms.preferences_form.font_size == 16.0,
                            "16",
                        ))
                        .clicked()
                    {
                        println!("Font 16");
                        self.forms.preferences_form.in_edit = true;
                        self.forms.preferences_form.font_size = 16.0;
                    }
                    // ui.radio_value(radio, FontFamily::Name("serif"), "Custom");
                });
            });
            ui.end_row();

            ui.horizontal(|ui| {
                if ui.button("Submit").clicked() {
                    // Save to DB
                    println!("Updating User Preferences");
                    let user_id = {
                        if let Some(current_user) = &self.current_user {
                            current_user.id
                        } else {
                            0
                        }
                    };
                    let input_vec = vec![UserPreferenceInput {
                        user_id: user_id,
                        font_family: self.forms.preferences_form.font_family.clone(),
                        color_scheme: self.forms.preferences_form.color_scheme.clone(),
                        font_size: self.forms.preferences_form.font_size.clone(),
                    }];
                    dbg!(&input_vec);
                    match &self.conn {
                        Some(conn) => {
                            println!("We have a conn!");
                            let _ = update_user_preferences(&conn, input_vec.clone());
                            // Refetch and resave current user with new, updated info
                            match user_from_id(&conn, input_vec[0].user_id) {
                                Ok(user) => self.current_user = Some(user),
                                Err(e) => {
                                    println!("Error getting User: {}", e);
                                    self.current_user = None;
                                }
                            }
                            // self.accounts.push((self.name_input.clone(), self.password_input.clone()));
                            // self.name_input.clear();
                        }
                        None => println!("No Conn"),
                    }
                }
                if ui.button("Clear").clicked() {
                    self.forms.preferences_form.font_family = FontFamily::Monospace;
                    self.forms.preferences_form.color_scheme = ColorScheme::Dark;
                    self.forms.preferences_form.font_size = 12.0;
                }
            });
        });
    }
    pub fn edit_account_form(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered_justified(|ui| {
            let mut radio = {
                if let Some(user) = &self.current_user {
                    user.preferences.font_family.clone()
                } else {
                    FontFamily::Monospace
                }
            };

            // ui.radio_value(&mut my_enum, Enum::First, "First");
            // ui.checkbox(&mut self.is_checked, "Option Enabled");
            // Font Family
            ui.add(Label::new("Font Family"));
            ui.horizontal(|ui| {
                // ui.radio_value(&mut radio, FontFamily::Monospace, "Monospace");
                // ui.radio_value(&mut radio, FontFamily::Proportional, "Proportional");
                if ui
                    .add(egui::RadioButton::new(
                        self.forms.preferences_form.font_family == FontFamily::Monospace,
                        "Monospace",
                    ))
                    .clicked()
                {
                    println!("Mono");
                    self.forms.preferences_form.in_edit = true;
                    self.forms.preferences_form.font_family = FontFamily::Monospace
                }
                if ui
                    .add(egui::RadioButton::new(
                        self.forms.preferences_form.font_family == FontFamily::Proportional,
                        "Proportional",
                    ))
                    .clicked()
                {
                    println!("Proportional");
                    self.forms.preferences_form.in_edit = true;
                    self.forms.preferences_form.font_family = FontFamily::Proportional
                }
                // ui.radio_value(radio, FontFamily::Name("serif"), "Custom");
            });
            ui.end_row();
            // Color
            ui.add(Label::new("Color Scheme"));
            ui.horizontal(|ui| {
                // ui.radio_value(&mut radio, FontFamily::Monospace, "Monospace");
                // ui.radio_value(&mut radio, FontFamily::Proportional, "Proportional");
                if ui
                    .add(egui::RadioButton::new(
                        self.forms.preferences_form.color_scheme == ColorScheme::Light,
                        "Light",
                    ))
                    .clicked()
                {
                    println!("Mono");
                    self.forms.preferences_form.in_edit = true;
                    self.forms.preferences_form.color_scheme = ColorScheme::Light
                }
                if ui
                    .add(egui::RadioButton::new(
                        self.forms.preferences_form.color_scheme == ColorScheme::Dark,
                        "Dark",
                    ))
                    .clicked()
                {
                    println!("Proportional");
                    self.forms.preferences_form.in_edit = true;
                    self.forms.preferences_form.color_scheme = ColorScheme::Dark
                }
                // ui.radio_value(radio, FontFamily::Name("serif"), "Custom");
            });
            ui.end_row();

            ui.horizontal(|ui| {
                if ui.button("Submit").clicked() {
                    // Save to DB
                    println!("Updating User Preferences");
                    let user_id = {
                        if let Some(current_user) = &self.current_user {
                            current_user.id
                        } else {
                            0
                        }
                    };
                    let input_vec = vec![UserPreferenceInput {
                        user_id: user_id,
                        font_family: self.forms.preferences_form.font_family.clone(),
                        color_scheme: self.forms.preferences_form.color_scheme.clone(),
                        font_size: self.forms.preferences_form.font_size.clone()
                    }];
                    dbg!(&input_vec);
                    match &self.conn {
                        Some(conn) => {
                            println!("We have a conn!");
                            let _ = update_user_preferences(&conn, input_vec);
                            // self.accounts.push((self.name_input.clone(), self.password_input.clone()));
                            // self.name_input.clear();
                        }
                        None => println!("No Conn"),
                    }
                }
                if ui.button("Clear").clicked() {
                    self.forms.preferences_form.font_family = FontFamily::Monospace;
                }
            });
        });
    }
}
