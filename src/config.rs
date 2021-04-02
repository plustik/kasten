use std::{
    env,
    path::PathBuf,
};

static DATABASE_LOCATION: &str = "./var/server-sled-db";
static FILE_LOCATION: &str = "./var/files/";

pub struct Config {
    pub database_location: PathBuf,
    pub file_location: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        // Read environment variables:
        let db_location = env::var("KASTEN_DB_LOCATION").unwrap_or(String::from(DATABASE_LOCATION));
        let file_location = env::var("KASTEN_FILE_LOCATION").unwrap_or(String::from(FILE_LOCATION));
        Config {
            database_location: PathBuf::from(db_location),
            file_location: PathBuf::from(file_location),
        }
    }
}
