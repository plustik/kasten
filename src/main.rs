#[macro_use]
extern crate rocket;

use std::{
    convert::From,
    fmt::{Display, Formatter},
};

mod config;
mod controller;
mod database;
mod models;
mod webapi;

#[rocket::main]
async fn main() {
    let config = config::Config::new();

    let db = database::Database::init(&config).unwrap();

    webapi::init(db, config).await.unwrap();
}

#[derive(Debug)]
pub enum Error {
    DbError(sled::Error),
    IoError(std::io::Error),
    ParseIntError(std::num::ParseIntError),
    EncodingError,
    TransactionAbortError,
    TransactionConflictError,
    TransactionStorageError,
    EntryNotFound,
    NoSuchUser,
    NoSuchDir,
    NoSuchFile,
    NoSuchTarget, // Calling a function to perform an action on a nonexisting target.
    InconsistentDbState,
    ForbiddenAction,
    MissingAuthorization, // Given user doesn't have permission
    BadCall,              // Calling a function with wrong arguments.
    TargetExists,         // Tries to create an existing object.
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        use Error::*;

        match self {
            DbError(e) => write!(f, "DB-Error: {}", e),
            IoError(e) => write!(f, "IoError: {}", e),
            ParseIntError(_) => write!(f, "Could not parse the given number."),
            EncodingError => write!(f, "Could not decode data."),
            TransactionAbortError => write!(f, "TransactionError: Abort"),
            TransactionConflictError => write!(f, "TransactionError: Conflict"),
            TransactionStorageError => write!(f, "TransactionError: StorageError"),
            EntryNotFound => write!(f, "The given entry was not found in the DB."),
            NoSuchUser => write!(f, "The given user does not exist in the DB."),
            NoSuchDir => write!(f, "The given directory does not exist in the DB."),
            NoSuchFile => write!(f, "The given file does not exist in the DB."),
            NoSuchTarget => write!(f, "The given target does not exist in the DB."),
            InconsistentDbState => write!(f, "The DB was found in an inconsisten state."),
            ForbiddenAction => write!(
                f,
                "Tried an action, that would result in a forbidden state."
            ),
            MissingAuthorization => write!(
                f,
                "The given user/group does not have the necessary permission for the given action."
            ),
            BadCall => write!(f, "Unexpected arguments."),
            TargetExists => write!(f, "Target exists."),
        }
    }
}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Self {
        Error::DbError(err)
    }
}
impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Error::ParseIntError(err)
    }
}
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}
impl<T> From<sled::transaction::TransactionError<T>> for Error {
    fn from(err: sled::transaction::TransactionError<T>) -> Self {
        match err {
            sled::transaction::TransactionError::Abort(_) => Error::TransactionAbortError,
            sled::transaction::TransactionError::Storage(e) => Error::DbError(e),
        }
    }
}
impl From<sled::transaction::UnabortableTransactionError> for Error {
    fn from(err: sled::transaction::UnabortableTransactionError) -> Self {
        match err {
            sled::transaction::UnabortableTransactionError::Conflict => {
                Error::TransactionConflictError
            }
            sled::transaction::UnabortableTransactionError::Storage(e) => Error::DbError(e),
        }
    }
}
impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Self {
        Error::EncodingError
    }
}
