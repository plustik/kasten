use chrono::{offset::Utc, DateTime, Duration};
use rocket::{
    request::{FromRequest, Outcome, Request},
    State,
};
use serde::Serialize;

use crate::{database::Database, webapi::GroupMsg};

#[derive(Serialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub pwd_hash: String,
    pub root_dir_id: u64,
}

pub struct Group {
    pub id: u64,
    pub name: String,
    pub member_ids: Vec<u64>,
    pub admin_ids: Vec<u64>,
}

impl Group {
    pub fn contains_user(&self, user_id: u64) -> bool {
        self.member_ids.contains(&user_id)
    }

    pub fn contains_admin(&self, user_id: u64) -> bool {
        self.admin_ids.contains(&user_id)
    }
}

impl From<GroupMsg> for Group {
    fn from(item: GroupMsg) -> Self {
        let id = item.id.unwrap_or(0);
        let name = item.name.unwrap_or(String::from("[MISSING_NAME]"));

        Group {
            id,
            name,
            member_ids: Vec::new(),
            admin_ids: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct UserSession {
    pub session_id: u64,
    pub user_id: u64,
    pub creation_date: DateTime<Utc>,
}

impl UserSession {
    pub fn new(session_id: u64, user_id: u64, creation_date: DateTime<Utc>) -> UserSession {
        UserSession {
            session_id,
            user_id,
            creation_date,
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserSession {
    type Error = crate::Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = req.cookies();
        let session_cookie = if let Some(c) = cookies.get("session_id") {
            c.value()
        } else {
            return Outcome::Forward(());
        };
        let db = req.guard::<&State<Database>>().await.unwrap();

        let session_id = if let Ok(id) = u64::from_str_radix(session_cookie, 16) {
            id
        } else {
            return Outcome::Forward(());
        };

        if let Ok(Some(res)) = db.get_user_session(session_id) {
            // Check, if the given session id to old:
            if Utc::now().signed_duration_since(res.creation_date) < Duration::hours(24) {
                Outcome::Success(res)
            } else {
                Outcome::Forward(())
            }
        } else {
            Outcome::Forward(())
        }
    }
}
