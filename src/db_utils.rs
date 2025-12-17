use std::sync::{Arc, Mutex};

use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, aead::{Aead, OsRng}};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row};

use crate::{AES_KEY, CredentialDetails, models::{Account, Credential, CredentialInput}};

pub fn create_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    println!("Creating DB");
    let res = conn.execute(
        "CREATE TABLE IF NOT EXISTS credential (
          id INTEGER PRIMARY KEY,
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
    Ok(())
}

fn build_db_credential(input: &CredentialInput) -> Credential {
    // Transformed from a byte array:
    // println!("Building DB Cred");
    let key = Key::<Aes256Gcm>::from_slice(AES_KEY.as_bytes());
    let cipher = Aes256Gcm::new(&key);
    // println!("After cipher");
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let nonce_slice = nonce.as_slice();
    let now: DateTime<Utc> = Utc::now();
    dbg!(&now);
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
        name: input.name.clone(),
        password_crypto: ciphertext,
        nonce: Some(nonce_slice.to_vec()),
        description: Some(input.description.clone()),
        details: details
    }
}

pub fn add_entries(conn: &Arc<Mutex<Connection>>, input_vec: Vec<CredentialInput>) -> Result<(), rusqlite::Error> {
    let now: DateTime<Utc> = Utc::now();
    let conn = conn.lock().unwrap();
    for cred in input_vec.iter() {
        let db_cred = build_db_credential(cred);
        dbg!(&db_cred);
        // let org_id = if idx % 2 == 0 { Some(org_id) } else { None };
        let res = conn.execute(
            "INSERT INTO credential (name, password_crypto, nonce, description, updated_at, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (&db_cred.name, &db_cred.password_crypto, &db_cred.nonce, &db_cred.description, now, now),
        );
        match res {
            Ok (res) => println!("{}", res),
            Err(e) => println!("{}", e),
        }
    }
    Ok(())
}

pub fn get_all_creds(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Credential>, Box<dyn std::error::Error>> {
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT id, name, password_crypto, nonce, description, updated_at, created_at
                    FROM credential
                    ORDER BY name ASC";
    let mut stmt = conn.prepare(select_sql)?;
    let mut rows = stmt.query([])?;

    let mut cred_vec: Vec<Credential> = vec!();
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let cred = cred_from_row(row);
        cred_vec.push(cred);
    }
    Ok(cred_vec)
}

pub fn get_creds(conn: &Arc<Mutex<Connection>>, acc: &str) -> Result<Credential, Box<dyn std::error::Error>> {
    let conn = conn.lock().unwrap();
    let select_sql = "SELECT id, name, password_crypto, nonce, description, updated_at, created_at
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
    Ok(final_cred)
}

pub fn delete_cred(conn: &Arc<Mutex<Connection>>, acc: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = conn.lock().unwrap();
    let select_sql = "DELETE FROM credential
                    WHERE name = :name";
    let mut stmt = conn.prepare(select_sql)?;
    let mut rows = stmt.query([])?;

    // while let Some(row) = rows.next()? {
    // }
    Ok(())
}

fn cred_from_row(row: &Row) -> Credential {
    let details = CredentialDetails { 
        updated_at: row.get(5).expect("Error"), 
        created_at: row.get(6).expect("Error") 
    };
    Credential {
        name: row.get(1).expect("Error"),
        password_crypto: row.get(2).expect("Error"),
        nonce: row.get(3).expect("Error"),
        description: row.get(4).expect("Error"),
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
