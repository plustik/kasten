use std::path::PathBuf;

static DATABASE_LOCATION: &str = "./test-env/server-sled-db";
static FILE_LOCATION: &str = "./test-env/files/";

pub struct Config {
    pub database_location: PathBuf,
    pub file_location: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        Config {
            database_location: PathBuf::from(DATABASE_LOCATION),
            file_location: PathBuf::from(FILE_LOCATION),
        }
    }
}
