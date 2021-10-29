use rocket::request::FromParam;
use serde::{de::Visitor, Deserialize, Serialize, Serializer};

use std::{convert::TryFrom, fmt, num::ParseIntError};

use crate::Error;

mod dirsystem;
mod users;

pub use dirsystem::{Dir, DirBuilder, File, FileBuilder, FsNode};
pub use users::{Group, User, UserSession};

#[derive(Clone, Copy, Debug)]
pub struct Id(u64);

impl Id {
    pub fn inner(&self) -> u64 {
        self.0
    }
    pub fn as_int(&self) -> u64 {
        self.0
    }
}

impl<'r> FromParam<'r> for Id {
    type Error = Error;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Ok(Id(u64::from_str_radix(param, 16)?))
    }
}
impl From<u64> for Id {
    fn from(value: u64) -> Self {
        Id(value)
    }
}
impl TryFrom<&str> for Id {
    type Error = ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Id(u64::from_str_radix(value, 16)?))
    }
}
impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(IdVisitor)
    }
}
impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(format!("{:x}", self.as_int()).as_str())
    }
}
impl std::cmp::PartialEq for &Id {
    fn eq(&self, other: &Self) -> bool {
        self.as_int() == other.as_int()
    }
}

struct IdVisitor;
impl<'de> Visitor<'de> for IdVisitor {
    type Value = Id;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between 0 and 2^64 as a hex string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Id::try_from(value).map_err(|_: ParseIntError| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(value),
                &"an ID as a hex integer",
            )
        })
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Id(value))
    }
}
