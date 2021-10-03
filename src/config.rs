use std::{env, path::PathBuf};

static DATABASE_LOCATION: &str = "./var/server-sled-db";
static FILE_LOCATION: &str = "./var/files/";
static STATIC_FILES: &str = "./static/";

pub struct Config {
    pub database_location: PathBuf,
    pub file_location: PathBuf,
    pub static_files: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        // Read environment variables:
        let db_location =
            env::var("KASTEN_DB_LOCATION").unwrap_or_else(|_| String::from(DATABASE_LOCATION));
        let file_location =
            env::var("KASTEN_FILE_LOCATION").unwrap_or_else(|_| String::from(FILE_LOCATION));
        let static_files =
            env::var("KASTEN_STATIC_FILES").unwrap_or_else(|_| String::from(STATIC_FILES));
        Config {
            database_location: PathBuf::from(db_location),
            file_location: PathBuf::from(file_location),
            static_files: PathBuf::from(static_files),
        }
    }
}
