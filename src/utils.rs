fn handle_events(event: Event, sender: Sender) {
    match event {
        Event::GetPetImage(ctx, pet_kind) => {
            fetch_pet_image(ctx, pet_kind, sender);
        }
        Event::GetPetFromDB(ctx, db_con, pet_id) => {
            if let Ok(Some(pet)) = get_pet_from_db(db_con, pet_id) {
                let _ = sender.send(Event::SetSelectedPet(Some(pet)));
                ctx.request_repaint();
            }
        }
        Event::DeletePetFromDB(ctx, db_con, pet_id) => {
            if delete_pet_from_db(db_con.clone(), pet_id).is_ok() {
                if let Ok(pets) = get_pets_from_db(db_con) {
                    let _ = sender.send(Event::SetPets(pets));
                    ctx.request_repaint();
                }
            }
        }
        Event::InsertPetToDB(ctx, db_con, pet) => {
            if let Ok(new_pet) = insert_pet_to_db(db_con.clone(), pet) {
                if let Ok(pets) = get_pets_from_db(db_con) {
                    let _ = sender.send(Event::SetPets(pets));
                    let _ = sender.send(Event::SetSelectedPet(Some(new_pet)));
                    ctx.request_repaint();
                }
            }
        }
        _ => (),
    }
}


