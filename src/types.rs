use serde::{Deserialize, Serialize};


#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: u64,
    pub name: String,
}

#[derive(Deserialize, Serialize)]
pub struct FileAsset {
    pub id: u64,
    pub name: String,
    pub owner_id: u64,
}

#[derive(Deserialize, Serialize)]
pub enum Transaction {
    Upload(User, FileAsset),
    Download(User, FileAsset),
}

#[derive(Deserialize, Serialize)]
pub struct Block {
    pub index: usize,
    pub parent: Option<String>,
    pub hash: String,
    pub data: Option<Transaction>,
}

#[derive(PartialEq, Eq)]
pub enum SenmonError {
    InvalidIndex,
    InvalidParent,
}
