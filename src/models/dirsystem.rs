use serde::{Deserialize, Serialize};

use super::User;

pub trait FsNode {
    fn id(&self) -> u64;
    fn name(&self) -> &str;
    fn parent_id(&self) -> u64;
    fn owner_id(&self) -> u64;
    /**
     * Returns a list containing the IDs of all Groups, which members are allowed to read the given
     * FsNode.
     */
    fn readable_groups(&self) -> &[u64];
    /**
     * Returns a list containing the IDs of all Groups, which members are allowed to write the
     * given FsNode.
     */
    fn writeable_groups(&self) -> &[u64];

    fn may_read(&self, user: &User) -> bool {
        if self.owner_id() == user.id {
            return true;
        }
        for g_id in self.readable_groups() {
            if user.group_ids.contains(g_id) {
                return true;
            }
        }
        false
    }
    fn may_write(&self, user: &User) -> bool {
        if self.owner_id() == user.id {
            return true;
        }
        for g_id in self.writeable_groups() {
            if user.group_ids.contains(g_id) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct File {
    pub id: u64,
    pub parent_id: u64,
    pub owner_id: u64,
    pub read_group_ids: Vec<u64>,
    pub write_group_ids: Vec<u64>,
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
    fn readable_groups(&self) -> &[u64] {
        self.read_group_ids.as_slice()
    }
    fn writeable_groups(&self) -> &[u64] {
        self.write_group_ids.as_slice()
    }
}

impl Default for File {
    fn default() -> Self {
        File {
            id: 0,
            parent_id: 0,
            owner_id: 0,
            read_group_ids: Vec::new(),
            write_group_ids: Vec::new(),
            name: String::from("[new_file]"),
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Dir {
    pub id: u64,
    pub parent_id: u64,
    pub owner_id: u64,
    pub read_group_ids: Vec<u64>,
    pub write_group_ids: Vec<u64>,
    pub child_ids: Vec<u64>,
    pub name: String,
}

impl Default for Dir {
    fn default() -> Self {
        Dir {
            id: 0,
            parent_id: 0,
            owner_id: 0,
            read_group_ids: Vec::new(),
            write_group_ids: Vec::new(),
            child_ids: Vec::new(),
            name: String::from("[new_dir]"),
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
    fn readable_groups(&self) -> &[u64] {
        self.read_group_ids.as_slice()
    }
    fn writeable_groups(&self) -> &[u64] {
        self.write_group_ids.as_slice()
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
}
