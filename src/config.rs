use std::path::PathBuf;

static DATABASE_LOCATION: &str = "./test-env/server-sled-db";

pub struct Config {
    pub database_location: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        Config {
            database_location: PathBuf::from(DATABASE_LOCATION),
        }
    }
}
