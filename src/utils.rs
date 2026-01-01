use std::{env, sync::mpsc::Sender};


use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};

use crate::{db_utils::{add_entries, delete_cred, get_all_creds, get_creds}, models::{Credential, Event}};
pub fn decrypt(cred: &Credential) -> String {
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
fn handle_events(event: Event, sender: Sender<Event>) {
    match event {
        // // This was a web fetch
        // Event::GetCred(ctx, name) => {
        //     fetch_cred(ctx, name, sender);
        // }
        Event::GetCredFromDB(ctx, db_con, cred_name, user_id) => {
            if let Ok(cred) = get_creds(&db_con, &cred_name, user_id) {
                let _ = sender.send(Event::SetSelectedCred(Some(cred)));
                ctx.request_repaint();
            }
        }
        Event::DeleteCredFromDB(ctx, db_con, cred_name, user_id) => {
            if delete_cred(&db_con.clone(), &cred_name, user_id).is_ok() {
                if let Ok(creds) = get_all_creds(&db_con, user_id) {
                    let _ = sender.send(Event::SetCreds(creds));
                    ctx.request_repaint();
                }
            }
        }
        Event::InsertCredToDB(ctx, db_con, cred, user_id) => {
            let creds_vec = vec!(cred);
            if let Ok(_new_cred) = add_entries(&db_con.clone(), creds_vec) {
                if let Ok(creds) = get_all_creds(&db_con, user_id) {
                    let _ = sender.send(Event::SetCreds(creds));
                    ctx.request_repaint();
                }
            }
        }
        _ => (),
    }
}


