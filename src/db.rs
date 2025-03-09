use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

use crate::session::*;

#[derive(Clone)]
pub struct DatabaseConnection {
    pub ctx: Arc<Mutex<rusqlite::Connection>>,
}

impl DatabaseConnection {
    pub fn new(ctx: rusqlite::Connection) -> Self {
        Self {
            ctx: Arc::new(Mutex::new(ctx)),
        }
    }
}

pub async fn session_serialize(db: &DatabaseConnection, ssn: &Session) -> Option<rusqlite::Error> {
    let cnx = db.ctx.deref().lock().unwrap();
    let mut stmt =
        cnx.prepare("INSERT INTO sessions(session_id, user_id, expires) VALUES(?1, ?2, ?3);").unwrap();
    let result: Result<usize, _> = stmt.execute(
        (
            ssn.session_id,
            ssn.user_id,
            ssn.expires_at.to_rfc2822()
        ),
    );
    match result {
        Ok(1) => None,
        Err(e) => Some(e),
        _ => Some(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub async fn session_valid(db: &DatabaseConnection, ssn: &Session) -> bool {
    let cnx = db.ctx.deref().lock().unwrap();
    let result: Result<u32, _> = cnx.query_row_and_then(
        "SELECT 1 FROM sessions as s JOIN user_reg as u USING s.user_id = u.user_id WHERE u.user_id=?1 AND s.session_id=?2;",
        [ssn.user_id, ssn.session_id],
        |row| row.get(0)
    );
    return match result {
        Ok(1) => true,
        _ => false,
    };
}

pub async fn is_present(db: &DatabaseConnection, user_name: &str) -> bool {
    let cnx = db.ctx.deref().lock().unwrap();
    let result: Result<u32, _> = cnx.query_row_and_then(
        "SELECT * FROM user_reg WHERE username=?1",
        [user_name],
        |row| row.get(0),
    );
    result.is_ok()
}

pub async fn validate_user(db: &DatabaseConnection, user_name: &str, password: &str) -> bool {
    let cnx = db.ctx.deref().lock().unwrap();
    let result: Result<u32, _> = cnx.query_row(
        "SELECT 1 FROM user_reg WHERE username=?1 AND password=?2;",
        [user_name, password],
        |r| r.get(0),
    );
    if let Ok(_) = result {
        return true;
    } else {
        return false;
    }
}

pub async fn get_user_id(db: &DatabaseConnection, user_name: &str) -> Result<u32, rusqlite::Error> {
    let cnx = db.ctx.deref().lock().unwrap();
    let result: Result<u32, _> = cnx.query_row_and_then(
        "SELECT * FROM user_reg WHERE username=?1",
        [user_name],
        |row| row.get(0),
    );
    result
}

pub async fn add_user(
    db: &DatabaseConnection,
    user_name: &str,
    password: &str,
) -> Option<rusqlite::Error> {
    let cnx = db.ctx.deref().lock().unwrap();
    let result = cnx.execute(
        "INSERT INTO user_reg(username, password) VALUES(?1, ?2);",
        [user_name, password],
    );
    return match result {
        Ok(_) => None,
        Err(e) => Some(e),
    };
}

pub async fn delete_user(db: &DatabaseConnection, user_name: &str) -> Option<rusqlite::Error> {
    let cnx = db.ctx.deref().lock().unwrap();
    let result = cnx.execute("DELETE FROM user_reg WHERE username=?1;", [user_name]);
    return match result {
        Ok(_) => None,
        Err(e) => Some(e),
    };
}
