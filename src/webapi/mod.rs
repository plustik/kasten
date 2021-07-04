use rocket::{
    fs::{self, FileServer},
    Rocket,
};
use rocket_dyn_templates::{
    Engines,
    Template,
    tera::{
        Filter,
        Value,
    },
};

use std::collections::{
    HashMap,
    hash_map::RandomState,
};

use crate::{config::Config, database::Database};

mod content_routes;
mod errors;
use errors::error_catchers;


pub async fn init(db: Database, config: Config) -> Result<(), rocket::Error> {
    Rocket::build()
        .attach(Template::custom(init_template_engine))
        .mount("/", content_routes::get_routes())
        .mount(
            "/static",
            FileServer::new(config.static_files.as_path(), fs::Options::None),
        )
        .manage(config)
        .manage(db)
        .register("/", error_catchers())
        .ignite().await?
        .launch().await
}

fn init_template_engine(engines: &mut Engines) {
    // Add filter to show integers as hex values:
    struct HexFilter;
    impl Filter for HexFilter {
        fn filter(&self, in_value: &Value, _: &HashMap<String, Value, RandomState>) -> tera::Result<Value> {
            if let Value::Number(num) = in_value {
                if num.is_u64() {
                    Ok(Value::String(format!("{:x}", num.as_u64().unwrap())))
                } else {
                    Err(tera::Error::msg(format!(
                        "Number out of bounds: Not a u64"
                    )))
                }
            } else {
                Err(tera::Error::msg(format!(
                    "Not a Number"
                )))
            }
        }
    }
    engines.tera.register_filter("tohex", HexFilter);

    struct SecondFilter;
    impl Filter for SecondFilter {
        fn filter(&self, in_value: &Value, _: &HashMap<String, Value, RandomState>) -> tera::Result<Value> {
            if let Value::Array(vec) = in_value {
                if let Some(res) = vec.get(1) {
                    Ok(res.clone())
                } else {
                    Err(tera::Error::msg(format!(
                        "Index out of bounds: 1"
                    )))
                }
            } else {
                Err(tera::Error::msg(format!(
                    "Wrong type: Expected Array"
                )))
            }
        }
    }
    engines.tera.register_filter("second", SecondFilter);
}
