use rocket::request::{FromRequest, Outcome, Request, State};
use serde::Serialize;

use crate::{
    database::Database,
    models::Id,
};

#[derive(Serialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub pwd_hash: String,
    pub root_dir_id: u64,
}

struct Group {
    id: u64,
    name: String,
    admin_id: u64,
    member_ids: u64,
}

#[derive(Debug)]
pub struct UserSession {
    pub session_id: u64,
    pub user_id: u64,
}

impl UserSession {
    pub fn new(session_id: u64, user_id: u64) -> UserSession {
        UserSession {
            session_id,
            user_id,
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for UserSession {
    type Error = crate::Error;

    fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let cookies = req.cookies();
        let session_cookie = if let Some(c) = cookies.get("session_id") {
            c.value()
        } else {
            return Outcome::Forward(());
        };
        let db = req.guard::<State<Database>>().unwrap();

        let session_id = if let Ok(id) = u64::from_str_radix(session_cookie, 16) {
            id
        } else {
            return Outcome::Forward(());
        };

        if let Ok(Some(res)) = db.get_user_session(session_id) {
            Outcome::Success(res)
        } else {
            Outcome::Forward(())
        }
    }
}
