use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use rand::thread_rng;
use rocket::{
    fs::{self, FileServer},
    Rocket,
};
use rocket_dyn_templates::{
    tera::{Filter, Value},
    Engines, Template,
};
use serde::{Deserialize, Serialize};

use std::collections::{hash_map::RandomState, HashMap};

use crate::{
    config::Config,
    database::Database,
    models::{Dir, File, Group, Id, User},
};

mod content_routes;
mod errors;
mod rest_api;
use errors::error_catchers;

pub async fn init(db: Database, config: Config) -> Result<(), rocket::Error> {
    Rocket::build()
        .attach(Template::custom(init_template_engine))
        .mount("/", content_routes::get_routes())
        .mount(
            "/static",
            FileServer::new(config.static_files.as_path(), fs::Options::None),
        )
        .mount("/rest_api", rest_api::get_routes())
        .manage(config)
        .manage(db)
        .register("/", error_catchers())
        .ignite()
        .await?
        .launch()
        .await
}

fn init_template_engine(engines: &mut Engines) {
    // Add filter to show integers as hex values:
    struct HexFilter;
    impl Filter for HexFilter {
        fn filter(
            &self,
            in_value: &Value,
            _: &HashMap<String, Value, RandomState>,
        ) -> tera::Result<Value> {
            if let Value::Number(num) = in_value {
                if num.is_u64() {
                    Ok(Value::String(format!("{:x}", num.as_u64().unwrap())))
                } else {
                    Err(tera::Error::msg(String::from(
                        "Number out of bounds: Not a u64",
                    )))
                }
            } else {
                Err(tera::Error::msg(String::from("Not a Number")))
            }
        }
    }
    engines.tera.register_filter("tohex", HexFilter);

    struct SecondFilter;
    impl Filter for SecondFilter {
        fn filter(
            &self,
            in_value: &Value,
            _: &HashMap<String, Value, RandomState>,
        ) -> tera::Result<Value> {
            if let Value::Array(vec) = in_value {
                if let Some(res) = vec.get(1) {
                    Ok(res.clone())
                } else {
                    Err(tera::Error::msg(String::from("Index out of bounds: 1")))
                }
            } else {
                Err(tera::Error::msg(String::from("Wrong type: Expected Array")))
            }
        }
    }
    engines.tera.register_filter("second", SecondFilter);
}

/**
 * Representation of a possibly incomplete Dir that the server got as a requests body.
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct DirMsg {
    pub id: Option<Id>,
    pub parent_id: Option<Id>,
    pub owner_id: Option<Id>,
    pub read_group_ids: Option<Vec<Id>>,
    pub write_group_ids: Option<Vec<Id>>,
    pub child_ids: Option<Vec<Id>>,
    pub name: Option<String>,
}

impl DirMsg {
    pub fn apply_changes(self, dir: &mut Dir) {
        assert!(self.id.is_none() || self.id.unwrap().as_int() == dir.id);

        if let Some(parent) = self.parent_id {
            dir.parent_id = parent.as_int();
        }
        if let Some(owner) = self.owner_id {
            dir.owner_id = owner.as_int();
        }
        if let Some(name) = self.name {
            dir.name = name;
        }
    }
}
impl From<Dir> for DirMsg {
    fn from(dir: Dir) -> Self {
        DirMsg {
            id: Some(Id::from(dir.id)),
            parent_id: Some(Id::from(dir.parent_id)),
            owner_id: Some(Id::from(dir.owner_id)),
            read_group_ids: Some(dir.read_group_ids.into_iter().map(Id::from).collect()),
            write_group_ids: Some(dir.write_group_ids.into_iter().map(Id::from).collect()),
            child_ids: Some(dir.child_ids.into_iter().map(Id::from).collect()),
            name: Some(dir.name),
        }
    }
}

/**
 * Representation of a possibly incomplete File that the server got as a requests body.
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct FileMsg {
    pub id: Option<Id>,
    pub parent_id: Option<Id>,
    pub owner_id: Option<Id>,
    pub read_group_ids: Option<Vec<Id>>,
    pub write_group_ids: Option<Vec<Id>>,
    pub name: Option<String>,
}

impl FileMsg {
    pub fn apply_changes(self, file: &mut File) {
        assert!(self.id.is_none() || self.id.unwrap().as_int() == file.id);

        if let Some(parent) = self.parent_id {
            file.parent_id = parent.as_int();
        }
        if let Some(owner) = self.owner_id {
            file.owner_id = owner.as_int();
        }
        if let Some(name) = self.name {
            file.name = name;
        }
    }
}
impl From<File> for FileMsg {
    fn from(file: File) -> Self {
        FileMsg {
            id: Some(Id::from(file.id)),
            parent_id: Some(Id::from(file.parent_id)),
            owner_id: Some(Id::from(file.owner_id)),
            read_group_ids: Some(file.read_group_ids.into_iter().map(Id::from).collect()),
            write_group_ids: Some(file.write_group_ids.into_iter().map(Id::from).collect()),
            name: Some(file.name),
        }
    }
}

/**
 * Representation of a possibly incomplete User that the server got as a requests body.
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct UserMsg {
    pub id: Option<Id>,
    pub name: Option<String>,
    pub password: Option<String>,
}

impl UserMsg {
    pub fn apply_changes(self, user: &mut User) {
        assert!(self.id.is_none() || self.id.unwrap().as_int() == user.id || user.id == 0);

        if let Some(name) = self.name {
            user.name = name;
        }
        if let Some(password) = self.password {
            // Hash password:
            let pwd_bytes = password.as_bytes();
            let mut rng = thread_rng();
            let salt = SaltString::generate(&mut rng);
            let argon2 = Argon2::default();
            user.pwd_hash = argon2
                .hash_password(pwd_bytes, salt.as_ref())
                .expect("Could not hash password.")
                .to_string();
        }
    }
}
impl From<User> for UserMsg {
    fn from(user: User) -> Self {
        UserMsg {
            id: Some(Id::from(user.id)),
            name: Some(user.name),
            password: None,
        }
    }
}

/**
 * Representation of a possibly incomplete Group that the server got as a requests body.
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMsg {
    pub id: Option<Id>,
    pub name: Option<String>,
}

impl GroupMsg {
    pub fn apply_changes(self, group: &mut Group) {
        assert!(self.id.is_none() || self.id.unwrap().as_int() == group.id || group.id == 0);

        if let Some(name) = self.name {
            group.name = name;
        }
    }
}
impl From<Group> for GroupMsg {
    fn from(group: Group) -> Self {
        GroupMsg {
            id: Some(Id::from(group.id)),
            name: Some(group.name),
        }
    }
}
