use chrono::{DateTime, Duration, Utc};
use rand::Rng;

pub struct Session {
    pub user_id: u32,
    pub session_id: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

const SESSION_LIFETIME: chrono::TimeDelta = Duration::hours(1);

impl Session {
    pub fn new(user_id: u32) -> Self {
        let mut random_number = rand::thread_rng();
        let session_id: u32 = random_number.gen();
        let now = Utc::now();
        Session {
            user_id,
            session_id, 
            created_at: now,
            expires_at: now + SESSION_LIFETIME
        }
    }
}
