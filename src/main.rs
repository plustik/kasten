#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use std::{
    convert::From,
    fmt::{Display, Formatter},
};

mod config;
mod database;
mod models;
mod webapi;

fn main() {
    let config = config::Config::new();

    let db = database::Database::init(&config).unwrap();

    webapi::init(db).unwrap();
}

#[derive(Debug)]
pub enum Error {
    DbError(sled::Error),
    EntryNotFound,
    NoSuchUser,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        use Error::*;

        match self {
            DbError(e) => write!(f, "DB-Error: {}", e),
            EntryNotFound => write!(f, "The given entry was not found in the DB."),
            NoSuchUser => write!(f, "The given user does not exist in the DB."),
        }
    }
}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Self {
        Error::DbError(err)
    }
}
