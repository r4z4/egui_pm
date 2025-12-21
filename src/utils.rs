use std::sync::mpsc::Sender;


use crate::{db_utils::{add_entries, delete_cred, get_all_creds, get_creds}, models::Event};

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
            if delete_cred(&db_con.clone(), &cred_name).is_ok() {
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


