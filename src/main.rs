#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use std::convert::From;

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
    DBError(sled::Error),
    EntryNotFound,
}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Self {
        Error::DBError(err)
    }
}
