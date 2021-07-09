use serde::{Deserialize, Serialize};

use std::convert::TryInto;

use crate::Error;

pub trait FsNode {
    fn id(&self) -> u64;
    fn name(&self) -> &str;
    fn parent_id(&self) -> u64;
    fn owner_id(&self) -> u64;
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct File {
    pub id: u64,
    pub parent_id: u64,
    pub owner_id: u64,
    pub name: String,
}

impl FsNode for File {
    fn id(&self) -> u64 {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parent_id(&self) -> u64 {
        self.parent_id
    }

    fn owner_id(&self) -> u64 {
        self.owner_id
    }
}

impl Default for File {
    fn default() -> Self {
        File {
            id: 0,
            parent_id: 0,
            owner_id: 0,
            name: String::from("[new_file]")
        }
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub struct Dir {
    pub id: u64,
    pub parent_id: u64,
    pub owner_id: u64,
    pub child_ids: Vec<u64>,
    pub name: String,
}

impl Dir {
    pub fn from_db_entry(id: u64, bytes: &[u8]) -> Result<Self, Error> {
        let parent_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let owner_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let child_number = u16::from_be_bytes(bytes[16..18].try_into().unwrap()) as usize;

        let mut child_ids = Vec::with_capacity(child_number);
        for i in 0..child_number {
            child_ids.push(u64::from_be_bytes(
                bytes[(18 + i * 8)..(26 + i * 8)].try_into().unwrap(),
            ));
        }

        let name = String::from_utf8(Vec::from(&bytes[(18 + child_number * 8)..]))?;

        Ok(Dir {
            id,
            parent_id,
            owner_id,
            child_ids,
            name,
        })
    }
}

impl FsNode for Dir {
    fn id(&self) -> u64 {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parent_id(&self) -> u64 {
        self.parent_id
    }

    fn owner_id(&self) -> u64 {
        self.owner_id
    }
}

trait Rule {
    fn node_id(&self) -> u64;
    fn is_visible(&self) -> bool;
    fn may_read(&self) -> bool;
    fn may_write(&self) -> bool;
    fn pwd_hash(&self) -> Option<u64>;
}

struct UserRule {
    user_id: u64,
    node_id: u64,
    is_visible: bool,
    may_read: bool,
    may_write: bool,
    pwd_hash: Option<u64>,
}

impl Rule for UserRule {
    fn node_id(&self) -> u64 {
        self.node_id
    }

    fn is_visible(&self) -> bool {
        self.is_visible
    }

    fn may_read(&self) -> bool {
        self.may_read
    }

    fn may_write(&self) -> bool {
        self.may_write
    }

    fn pwd_hash(&self) -> Option<u64> {
        self.pwd_hash
    }
}

struct GroupRule {
    group_id: u64,
    node_id: u64,
    is_visible: bool,
    may_read: bool,
    may_write: bool,
    pwd_hash: Option<u64>,
}

impl Rule for GroupRule {
    fn node_id(&self) -> u64 {
        self.node_id
    }

    fn is_visible(&self) -> bool {
        self.is_visible
    }

    fn may_read(&self) -> bool {
        self.may_read
    }

    fn may_write(&self) -> bool {
        self.may_write
    }

    fn pwd_hash(&self) -> Option<u64> {
        self.pwd_hash
    }
}
