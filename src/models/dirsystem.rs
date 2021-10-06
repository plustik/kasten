use serde::{Deserialize, Serialize};

use std::convert::TryInto;

use crate::Error;

pub trait FsNode {
    fn id(&self) -> u64;
    fn name(&self) -> &str;
    fn parent_id(&self) -> u64;
    fn owner_id(&self) -> u64;
    fn user_may_read(&self) -> bool;
    fn user_may_write(&self) -> bool;
    fn group_may_read(&self) -> bool;
    fn group_may_write(&self) -> bool;

    fn may_write(&self, user_id: u64) -> bool {
        user_id == self.owner_id()
    }

    fn may_read(&self, user_id: u64) -> bool {
        user_id == self.owner_id()
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct File {
    pub id: u64,
    pub parent_id: u64,
    pub owner_id: u64,
    pub name: String,
    pub permission_bits: u16, // ..., User: rwx, Group: rwx, All: rwx
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

    fn user_may_read(&self) -> bool {
        self.permission_bits & 0b100000000 == 0b100000000
    }
    fn user_may_write(&self) -> bool {
        self.permission_bits & 0b010000000 == 0b010000000
    }
    fn group_may_read(&self) -> bool {
        self.permission_bits & 0b001000000 == 0b001000000
    }
    fn group_may_write(&self) -> bool {
        self.permission_bits & 0b000100000 == 0b000100000
    }
}

impl Default for File {
    fn default() -> Self {
        File {
            id: 0,
            parent_id: 0,
            owner_id: 0,
            name: String::from("[new_file]"),
            permission_bits: 0b110000000,
        }
    }
}

pub struct FileBuilder {
    file: File,
}

impl FileBuilder {
    pub fn new() -> Self {
        FileBuilder {
            file: File::default(),
        }
    }

    pub fn build(self) -> File {
        self.file
    }

    pub fn with_parent_id(mut self, parent_id: u64) -> Self {
        self.file.parent_id = parent_id;
        self
    }
    pub fn with_owner_id(mut self, owner_id: u64) -> Self {
        self.file.owner_id = owner_id;
        self
    }
    pub fn with_name<T: Into<String>>(mut self, name: T) -> Self {
        self.file.name = name.into();
        self
    }
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.file.name = name.into();
    }
    pub fn set_permissions<T: Into<u16>>(&mut self, bits: T) {
        self.file.permission_bits = bits.into();
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Dir {
    pub id: u64,
    pub parent_id: u64,
    pub owner_id: u64,
    pub child_ids: Vec<u64>,
    pub name: String,
    pub permission_bits: u16,
}

impl Dir {
    pub fn from_db_entry(id: u64, bytes: &[u8]) -> Result<Self, Error> {
        let parent_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let owner_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let permission_bits = u16::from_be_bytes(bytes[16..18].try_into().unwrap());
        let child_number = u16::from_be_bytes(bytes[18..20].try_into().unwrap()) as usize;

        let mut child_ids = Vec::with_capacity(child_number);
        for i in 0..child_number {
            child_ids.push(u64::from_be_bytes(
                bytes[(20 + i * 8)..(28 + i * 8)].try_into().unwrap(),
            ));
        }

        let name = String::from_utf8(Vec::from(&bytes[(20 + child_number * 8)..]))?;

        Ok(Dir {
            id,
            parent_id,
            owner_id,
            child_ids,
            name,
            permission_bits,
        })
    }
}

impl Default for Dir {
    fn default() -> Self {
        Dir {
            id: 0,
            parent_id: 0,
            owner_id: 0,
            child_ids: Vec::new(),
            name: String::from("[new_dir]"),
            permission_bits: 0b110000000,
        }
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

    fn user_may_read(&self) -> bool {
        self.permission_bits & 0b100000000 == 0b100000000
    }
    fn user_may_write(&self) -> bool {
        self.permission_bits & 0b010000000 == 0b010000000
    }
    fn group_may_read(&self) -> bool {
        self.permission_bits & 0b001000000 == 0b001000000
    }
    fn group_may_write(&self) -> bool {
        self.permission_bits & 0b000100000 == 0b000100000
    }
}

pub struct DirBuilder {
    dir: Dir,
}

impl DirBuilder {
    pub fn new() -> Self {
        DirBuilder {
            dir: Dir::default(),
        }
    }

    pub fn build(self) -> Dir {
        self.dir
    }

    pub fn set_id(&mut self, id: u64) {
        self.dir.id = id;
    }

    pub fn set_parent_id(&mut self, parent_id: u64) {
        self.dir.parent_id = parent_id;
    }

    pub fn set_owner_id(&mut self, owner_id: u64) {
        self.dir.owner_id = owner_id;
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.dir.name = name.into();
    }

    pub fn with_id(mut self, id: u64) -> Self {
        self.dir.id = id;
        self
    }

    pub fn with_parent_id(mut self, parent_id: u64) -> Self {
        self.dir.parent_id = parent_id;
        self
    }

    pub fn with_owner_id(mut self, owner_id: u64) -> Self {
        self.dir.owner_id = owner_id;
        self
    }

    pub fn with_name<T: Into<String>>(mut self, name: T) -> Self {
        self.dir.name = name.into();
        self
    }
    pub fn set_permissions<T: Into<u16>>(&mut self, bits: T) {
        self.dir.permission_bits = bits.into();
    }
}
