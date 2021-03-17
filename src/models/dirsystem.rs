pub trait FsNode {
    fn id(&self) -> u64;
    fn name(&self) -> &str;
    fn parent_id(&self) -> u64;
    fn owner_id(&self) -> u64;
}

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

pub struct Dir {
    pub id: u64,
    pub parent_id: u64,
    pub owner_id: u64,
    pub child_ids: Vec<u64>,
    pub name: String,
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
