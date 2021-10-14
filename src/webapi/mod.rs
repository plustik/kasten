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
use serde::Deserialize;

use std::collections::{hash_map::RandomState, HashMap};

use crate::{
    config::Config,
    database::Database,
    models::{Dir, File, Group, User},
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
#[derive(Deserialize)]
pub struct DirMsg {
    pub id: Option<u64>,
    pub parent_id: Option<u64>,
    pub owner_id: Option<u64>,
    pub read_group_ids: Option<Vec<u64>>,
    pub write_group_ids: Option<Vec<u64>>,
    pub child_ids: Option<Vec<u64>>,
    pub name: Option<String>,
}

impl DirMsg {
    pub fn apply_changes(self, dir: &mut Dir) {
        assert!(self.id.is_none() || self.id.unwrap() == dir.id);

        if let Some(parent) = self.parent_id {
            dir.parent_id = parent;
        }
        if let Some(owner) = self.owner_id {
            dir.owner_id = owner;
        }
        if let Some(name) = self.name {
            dir.name = name;
        }
    }
}

/**
 * Representation of a possibly incomplete File that the server got as a requests body.
 */
#[derive(Deserialize)]
pub struct FileMsg {
    pub id: Option<u64>,
    pub parent_id: Option<u64>,
    pub owner_id: Option<u64>,
    pub read_group_ids: Option<Vec<u64>>,
    pub write_group_ids: Option<Vec<u64>>,
    pub name: Option<String>,
}

impl FileMsg {
    pub fn apply_changes(self, file: &mut File) {
        assert!(self.id.is_none() || self.id.unwrap() == file.id);

        if let Some(parent) = self.parent_id {
            file.parent_id = parent;
        }
        if let Some(owner) = self.owner_id {
            file.owner_id = owner;
        }
        if let Some(name) = self.name {
            file.name = name;
        }
    }
}

/**
 * Representation of a possibly incomplete User that the server got as a requests body.
 */
#[derive(Deserialize)]
pub struct UserMsg {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub password: Option<String>,
}

impl UserMsg {
    pub fn apply_changes(self, user: &mut User) {
        assert!(self.id.is_none() || self.id.unwrap() == user.id || user.id == 0);

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
                .hash_password_simple(pwd_bytes, salt.as_ref())
                .expect("Could not hash password.")
                .to_string();
        }
    }
}

/**
 * Representation of a possibly incomplete Group that the server got as a requests body.
 */
#[derive(Deserialize)]
pub struct GroupMsg {
    pub id: Option<u64>,
    pub name: Option<String>,
}

impl GroupMsg {
    pub fn apply_changes(self, group: &mut Group) {
        assert!(self.id.is_none() || self.id.unwrap() == group.id || group.id == 0);

        if let Some(name) = self.name {
            group.name = name;
        }
    }
}
