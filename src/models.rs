use chrono::{DateTime, Utc};

// enum Event {
//     SetPets(Vec),
//     GetPetImage(egui::Context, PetKind),
//     SetPetImage(Option),
//     GetPetFromDB(egui::Context, Arc<Mutex>, i64),
//     SetSelectedPet(Option),
//     InsertPetToDB(egui::Context, Arc<Mutex>, Pet),
//     DeletePetFromDB(egui::Context, Arc<Mutex>, i64),
// }

#[derive(Debug)]
pub struct CredentialInput {
    pub name: String,
    pub password: String,
    pub description: String,
}

#[derive(Clone)]
pub struct Account {
    pub id: i32,
    pub name: String
}

#[derive(Clone, Default, Debug)]
pub struct CredentialDetails {
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Default, Debug)]
pub struct Credential {
    pub name: String,
    pub password_crypto: Vec<u8>,
    pub nonce: Option<Vec<u8>>,
    pub description: Option<String>,
    pub details: CredentialDetails
}

