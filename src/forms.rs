use eframe::egui::{self, FontFamily, Label};

use crate::{
    App, db_utils::{add_entries, update_entries, update_user_preferences, user_from_id}, decrypt, models::{Account, Credential, CredentialInput, UserPreferenceInput}
};

#[derive(Default)]
pub struct PreferencesForm {
    font_family: FontFamily,
    color_scheme: ColorScheme,
    in_edit: bool,
    // font_family: String
}

#[derive(Default)]
pub struct AccountForm {
    name: String,
    password: String,
    description: String,
    updating: bool,
    // font_family: String
}

#[derive(PartialEq, Clone, Debug, Default, Hash, Eq)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

impl App {
    pub fn account_form(&mut self, ui: &mut egui::Ui, credential: Option<Credential>) {
        let mut heading = "New Account".to_string();
        if let Some(cred) = credential {
            heading = cred.name.clone();
            self.forms.account_form.updating = true;
            self.forms.account_form.name = cred.name.clone();
            self.forms.account_form.password = decrypt(&cred);
            self.forms.account_form.description = cred.description.unwrap_or("".to_string());
        }
        ui.collapsing(heading, |ui| {
            ui.vertical_centered_justified(|ui| {
                egui::Grid::new("form_grid").num_columns(2).spacing([8.0, 12.0]).show(ui, |ui| {
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
                            id: None,
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
                                self.account_edit = None;
                            }
                            None => println!("No Conn"),
                        }
                    }
                    if ui.button("Clear").clicked() {
                        self.forms.account_form.name.clear();
                        self.forms.account_form.password.clear();
                        self.forms.account_form.description.clear();
                    }
                });
            });
        });
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
