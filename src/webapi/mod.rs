use rocket::Rocket;
use rocket_contrib::{
    serve::{self, StaticFiles},
    templates::{
        tera::{self, Value},
        Engines, Template,
    },
};

use std::collections::HashMap;

use crate::{config::Config, database::Database};

mod content_routes;
mod errors;
use errors::error_catchers;

pub fn init(db: Database, config: Config) -> Result<(), ()> {
    Rocket::ignite()
        .attach(Template::custom(init_template_engine))
        .mount("/", content_routes::get_routes())
        .mount(
            "/static",
            StaticFiles::new(config.static_files.as_path(), serve::Options::None),
        )
        .manage(config)
        .manage(db)
        .register(error_catchers())
        .launch();

    Ok(())
}

fn init_template_engine(engines: &mut Engines) {
    // Add filter to show integers as hex values:
    fn hex_filter(in_value: Value, _: HashMap<String, Value>) -> tera::Result<Value> {
        if let Value::Number(num) = in_value {
            if num.is_u64() {
                Ok(Value::String(format!("{:x}", num.as_u64().unwrap())))
            } else {
                Err(tera::Error::from_kind(tera::ErrorKind::Msg(format!(
                    "Number out of bounds: Not a u64"
                ))))
            }
        } else {
            Err(tera::Error::from_kind(tera::ErrorKind::Msg(format!(
                "Not a Number"
            ))))
        }
    }
    engines.tera.register_filter("tohex", hex_filter);

    fn second_filter(in_value: Value, _: HashMap<String, Value>) -> tera::Result<Value> {
        if let Value::Array(vec) = in_value {
            if let Some(res) = vec.get(1) {
                Ok(res.clone())
            } else {
                Err(tera::Error::from_kind(tera::ErrorKind::Msg(format!(
                    "Index out of bounds: 1"
                ))))
            }
        } else {
            Err(tera::Error::from_kind(tera::ErrorKind::Msg(format!(
                "Wrong type: Expected Array"
            ))))
        }
    }
    engines.tera.register_filter("second", second_filter);
}
