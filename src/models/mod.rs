use rocket::request::FromParam;

use crate::Error;

mod dirsystem;
mod users;

pub use dirsystem::{Dir, DirBuilder, File, FileBuilder, FsNode};
pub use users::{Group, User, UserSession};

pub struct Id(u64);

impl Id {
    pub fn inner(&self) -> u64 {
        self.0
    }
}

impl<'r> FromParam<'r> for Id {
    type Error = Error;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Ok(Id(u64::from_str_radix(param, 16)?))
    }
}
