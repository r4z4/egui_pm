use std::{collections::HashMap, env, io::{Error, ErrorKind}, sync::{Arc, Mutex}};

use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, aead::{Aead, OsRng}};
use chrono::{DateTime, Utc};
use eframe::egui::{Color32, FontFamily};
use rusqlite::{Connection, Row};

use crate::{CredentialDetails, forms::ColorScheme, models::{Account, Credential, CredentialInput, User, UserPreference, UserPreferenceInput}};

// TODO = Create init script
pub fn create_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    println!("Creating DB");
    let res = conn.execute(
        "CREATE TABLE IF NOT EXISTS user (
          id INTEGER PRIMARY KEY,
          username TEXT NOT NULL UNIQUE,
          pin TEXT,
          updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
          created_at TEXT DEFAULT CURRENT_TIMESTAMP
        ) STRICT",
        (), // Empty list of params
    );
    match res {
        Ok(usize) => println!("{}", usize),
        Err(e) => println!("{}", e),
    }
    let res = conn.execute(
        "CREATE TABLE IF NOT EXISTS color_scheme (
          id INTEGER PRIMARY KEY,
          name TEXT NOT NULL
        ) STRICT",
        (), // Empty list of params
    );
    match res {
        Ok(usize) => println!("{}", usize),
        Err(e) => println!("{}", e),
    }
    let insert_sql = "INSERT OR IGNORE INTO color_scheme (name) 
                VALUES ('latte'), ('mocha'), ('macchiato'), ('frappe')";
    let res = conn.execute(insert_sql, ());
    match res {
        Ok(usize) => println!("{}", usize),
        Err(e) => println!("{}", e),
    }
    let res = conn.execute(
        "CREATE TABLE IF NOT EXISTS user_setting (
          id INTEGER PRIMARY KEY,
          user_id INTEGER REFERENCES user(id) UNIQUE,
          font_family TEXT NOT NULL DEFAULT 'monospace',
          color_scheme_id INTEGER REFERENCES color_scheme(id) NOT NULL DEFAULT 1, -- Light
          font_size INTEGER NOT NULL DEFAULT 12,
          updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
          created_at TEXT DEFAULT CURRENT_TIMESTAMP
        ) STRICT",
        (), // Empty list of params
    );
    match res {
        Ok(usize) => println!("{}", usize),
        Err(e) => println!("{}", e),
    }
    let res = conn.execute(
        "CREATE TABLE IF NOT EXISTS credential (
          id INTEGER PRIMARY KEY,
          user_id INTEGER REFERENCES user(id),
          name TEXT NOT NULL,
          password_crypto BLOB,
          nonce BLOB,
          description TEXT,
          updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
          created_at TEXT DEFAULT CURRENT_TIMESTAMP
        ) STRICT",
        (), // Empty list of params
    );
    match res {
        Ok(usize) => println!("{}", usize),
        Err(e) => println!("{}", e),
    }

    let insert_sql = "INSERT OR IGNORE INTO user (username, pin) 
                VALUES ('aaron', '1234')";
    let res = conn.execute(insert_sql, ());
    match res {
        Ok(usize) => println!("{}", usize),
        Err(e) => println!("{}", e),
    }
    let insert_sql = "INSERT OR IGNORE INTO user_setting (user_id, font_family) 
                VALUES (1, 'proportional')";
    let res = conn.execute(insert_sql, ());
    match res {
        Ok(usize) => println!("{}", usize),
        Err(e) => println!("{}", e),
    }

    Ok(())
}

fn build_db_credential(input: &CredentialInput) -> Credential {
    // Transformed from a byte array:
    // println!("Building DB Cred");
    let aes_key: &str = &env::var("AES_KEY").expect("AES_KEY must be set in .env file");
    let key = Key::<Aes256Gcm>::from_slice(aes_key.as_bytes());
    let cipher = Aes256Gcm::new(&key);
    // println!("After cipher");
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let nonce_slice = nonce.as_slice();
    let now: DateTime<Utc> = Utc::now();
    let details = CredentialDetails {
        updated_at: now,
        created_at: now,
    };
    dbg!(&details);
    let ciphertext = cipher.encrypt(&nonce, input.password.as_ref()).unwrap();
    // let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();
    // assert_eq!(&plaintext, b"plaintext message");
    // let stored = String::from_utf8(ciphertext).expect("Invalid UTF-8");
    Credential {
        id: input.id,
        user_id: input.user_id,
        name: input.name.clone(),
        password_crypto: ciphertext,
        nonce: Some(nonce_slice.to_vec()),
        description: Some(input.description.clone()),
        details: details
    }
}

pub fn save_pin_to_db(conn: &Arc<Mutex<Connection>>, pin: String) -> Result<(), rusqlite::Error> {
    let now: DateTime<Utc> = Utc::now();
    let conn = conn.lock().unwrap();
    let user_id = 1;
    // let org_id = if idx % 2 == 0 { Some(org_id) } else { None };
    let res = conn.execute(
        "UPDATE user SET pin = ?1 WHERE id = ?2",
        (pin, user_id),
    );
    match res {
        Ok (res) => println!("{}", res),
        Err(e) => println!("{}", e),
    }
    
    Ok(())
}

pub fn add_entries(conn: &Arc<Mutex<Connection>>, input_vec: Vec<CredentialInput>) -> Result<(), rusqlite::Error> {
    let now: DateTime<Utc> = Utc::now();
    let conn = conn.lock().unwrap();
    for cred in input_vec.iter() {
        let db_cred = build_db_credential(cred);
        dbg!(&db_cred);
        // let org_id = if idx % 2 == 0 { Some(org_id) } else { None };
        let res = conn.execute(
            "INSERT INTO credential (user_id, name, password_crypto, nonce, description, updated_at, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&db_cred.user_id, &db_cred.name, &db_cred.password_crypto, &db_cred.nonce, &db_cred.description, now, now),
        );
        match res {
            Ok (res) => println!("{}", res),
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}

// These are only really ever used for singular records, but making them take vecs just because
pub fn update_entries(conn: &Arc<Mutex<Connection>>, input_vec: Vec<CredentialInput>) -> Result<(), rusqlite::Error> {
    let now: DateTime<Utc> = Utc::now();
    let conn = conn.lock().unwrap();
    for cred in input_vec.iter() {
        let db_cred = build_db_credential(cred);
        dbg!(&db_cred);
        // let org_id = if idx % 2 == 0 { Some(org_id) } else { None };
        let res = conn.execute(
            "UPDATE credential SET name = ?1, password_crypto = ?2, nonce = ?3, description = ?4, updated_at = ?5
            WHERE id = ?6",
            (&db_cred.name, &db_cred.password_crypto, &db_cred.nonce, &db_cred.description, now, &db_cred.id),
        );
        match res {
            Ok (res) => println!("{}", res),
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}

fn map_font_family(ff: &FontFamily) -> &'static str {
    match ff {
        FontFamily::Monospace => "monospace",
        FontFamily::Proportional => "proportional",
        _ => "monospace",
    }
}

fn map_color_scheme(cs: &ColorScheme) -> i32 {
    match cs {
        ColorScheme::Light => 1,
        ColorScheme::Dark => 2,
        _ => 1,
    }
}

fn find_key_for_value<T: PartialEq>(map: &HashMap<i32, T>, value: T) -> Option<&i32> {
    map.iter()
        .find_map(|(key, val)| if *val == value { Some(key) } else { None })
}

fn find_key_for_value_str<T: PartialEq>(map: &HashMap<String, T>, value: T) -> Option<&String> {
    map.iter()
        .find_map(|(key, val)| if *val == value { Some(key) } else { None })
}

pub fn update_user_preferences(conn: &Arc<Mutex<Connection>>, input_vec: Vec<UserPreferenceInput>) -> Result<(), rusqlite::Error> {
    let now: DateTime<Utc> = Utc::now();
    let conn = conn.lock().unwrap();
    for input in input_vec.iter() {
        // let input = build_db_credential(cred);
        dbg!(&input);
        let font_family_input = map_font_family(&input.font_family);
        let color_scheme_id = map_color_scheme(&input.color_scheme);
        println!("Updating Color Scheme to {}", &color_scheme_id);
        // let org_id = if idx % 2 == 0 { Some(org_id) } else { None };
        let res = conn.execute(
            "UPDATE user_setting 
             SET font_family = ?1, color_scheme_id = ?2, updated_at = ?3 
             WHERE user_id = ?4",
            (font_family_input, color_scheme_id, now, &input.user_id),
        );
        match res {
            Ok (res) => println!("{}", res),
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}

pub fn user_from_id(conn: &Arc<Mutex<Connection>>, user_id: i32) -> Result<User, Box<dyn std::error::Error>> {
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT user.id, user.username, user.pin, user_setting.font_family, user_setting.color_scheme_id 
                    FROM user
                    INNER JOIN user_setting ON user_setting.user_id = user.id
                    WHERE user.id = :user_id LIMIT 1";
    let mut stmt = conn.prepare(select_sql)?;
    let mut rows = stmt.query(&[(":user_id", user_id.to_string().as_str())])?;
    // let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let user = User {
            id: row.get(0)?,
            username: row.get(1)?,
            pin: row.get(2)?,
            preferences: build_user_preference(row.get(3)?, row.get(4)?),
        };
        Ok(user)
    } else {
        Err(Box::new(Error::new(ErrorKind::Other, "No Row")))
    }
}

pub fn get_all_creds(conn: &Arc<Mutex<Connection>>, user_id: i32) -> Result<Vec<Credential>, Box<dyn std::error::Error>> {
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT id, user_id, name, password_crypto, nonce, description, updated_at, created_at
                    FROM credential
                    WHERE user_id = :user_id
                    ORDER BY name ASC";
    let mut stmt = conn.prepare(select_sql)?;
    let mut rows = stmt.query(&[(":user_id", user_id.to_string().as_str())])?;

    let mut cred_vec: Vec<Credential> = vec!();
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let cred = cred_from_row(row);
        cred_vec.push(cred);
    }
    Ok(cred_vec)
}

pub fn get_creds(conn: &Arc<Mutex<Connection>>, acc: &str, user_id: i32) -> Result<Credential, Box<dyn std::error::Error>> {
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT id, user_id, name, password_crypto, nonce, description, updated_at, created_at
                    FROM credential
                    WHERE user_id = :user_id
                    AND name = :name";
    let mut stmt = conn.prepare(select_sql)?;
    let mut rows = stmt.query(&[(":name", acc), (":user_id", user_id.to_string().as_str())])?;

    let mut final_cred: Credential = Credential::default();
    while let Some(row) = rows.next()? {
        let name: String = row.get(2)?;
        let cred = cred_from_row(row);
        final_cred = cred;
        println!("->> name: {name}");
        println!("->>  row: {row:?}");
    }
    Ok(final_cred)
}

pub fn delete_cred(conn: &Arc<Mutex<Connection>>, acc: &str, user_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    dbg!(&acc);
    dbg!(&user_id);
    let conn = conn.lock().unwrap();
    let select_sql = "DELETE FROM credential
                    WHERE name = :name
                    AND user_id = :user_id";
    let mut stmt = conn.prepare(select_sql)?;
    let rows = stmt.execute(&[(":name", acc), (":user_id", user_id.to_string().as_str())])?;
    println!("Deleted {} row(s)", rows);
    // while let Some(row) = rows.next()? {
    // }
    Ok(())
}

fn cred_from_row(row: &Row) -> Credential {
    let details = CredentialDetails { 
        updated_at: row.get(6).expect("Error"), 
        created_at: row.get(7).expect("Error") 
    };
    Credential {
        id: row.get(0).expect("Error"),
        user_id: row.get(1).expect("Error"),
        name: row.get(2).expect("Error"),
        password_crypto: row.get(3).expect("Error"),
        nonce: row.get(4).expect("Error"),
        description: row.get(5).expect("Error"),
        details: details
    }
}

pub fn get_current_accounts(conn: Arc<Mutex<Connection>>) -> Result<Vec<Account>, rusqlite::Error> { 
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT id, name
                    FROM credential
                    ORDER BY name ASC";
    let mut stmt = conn.prepare(select_sql)?;
    let rows = stmt.query([]);

    let mut accounts: Vec<Account> = vec!();
    match rows {
        Ok(mut rows) => {
            while let Some(row) = rows.next()? {
                let account =  Account {id: row.get(0)?, name: row.get(1)?};
                accounts.push(account);
            }
            Ok(accounts)
        },
        Err(e) => {
            println!("Err");
            Err(e)
        }
    }
}

pub fn get_current_users(conn: Arc<Mutex<Connection>>) -> Result<Vec<User>, rusqlite::Error> { 
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT user.id, user.username, user.pin, user_setting.font_family, user_setting.color_scheme_id
                    FROM user
                    INNER JOIN user_setting ON user_setting.user_id = user.id
                    ORDER BY user.id ASC";
    let mut stmt = conn.prepare(select_sql)?;
    let rows = stmt.query([]);

    let mut users: Vec<User> = vec!();
    match rows {
        Ok(mut rows) => {
            while let Some(row) = rows.next()? {
                let user =  User {id: row.get(0)?, username: row.get(1)?, pin: row.get(2)?, preferences: build_user_preference(row.get(3)?, row.get(4)?)};
                dbg!(&user);
                users.push(user);
            }
            Ok(users)
        },
        Err(e) => {
            println!("Err");
            Err(e)
        }
    }
}

fn get_color_scheme(id: i32) -> ColorScheme {
    match id {
        1 => ColorScheme::Light,
        2 => ColorScheme::Dark,
        _ => ColorScheme::Light,
    }
}

fn build_user_preference(db_name: String, color_scheme_id: i32) -> UserPreference {
    dbg!(&color_scheme_id);
    let up = UserPreference {font_family: get_font_family(&db_name), color_scheme: get_color_scheme(color_scheme_id), font_size: 12.0};
    up
}

// pub fn insert_test_user(conn: Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> { 
//     let conn = conn.lock().unwrap();
//     let insert_sql = "INSERT OR IGNORE INTO user (username, pin) 
//                     VALUES ('aaron', '1234')";
//     let mut stmt = conn.prepare(insert_sql)?;
//     let rows = stmt.query([]);
//
//     match rows {
//         Ok(mut _rows) => {
//             println!("DB Success adding test user");
//             Ok(())
//         },
//         Err(e) => {
//             println!("Err");
//             Err(e)
//         }
//     }
// }

fn get_font_family(db_name: &str) -> FontFamily {
    match db_name {
        "monospace" => FontFamily::Monospace,
        "proportional" => FontFamily::Proportional,
        _ => FontFamily::Monospace,
    }
}

pub fn create_and_store_backup(conn: Arc<Mutex<Connection>>) {
    use std::process::Command;
    let output = Command::new("sqlite3")
        .arg("_pmdb.db")
        .arg(".backup 'backup_file.sq3'")
        .output()
        .expect("failed to execute process");
    dbg!(output);
    // Save to a hidden dir
    let second = Command::new("mv")
        .arg("backup_file.sq3")
        .arg("/.pmdb/backup_file.sq3")
        .arg("-y")
        .output()
        .expect("unable to run command");
    dbg!(second);
}
