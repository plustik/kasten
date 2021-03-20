use std::convert::TryInto;

use rand::{thread_rng, RngCore};
use sled::{Db, Tree};

use crate::{
    config::Config,
    models::{Dir, File, User, UserSession},
    Error,
};

pub struct Database {
    _sled_db: Db,
    session_tree: Tree,        // K: session_id, V: user_id
    username_id_tree: Tree,    // K: username, V: user_id
    userid_name_tree: Tree,    // K: user_id, V: username
    userid_pwd_tree: Tree,     // K: user_id, V: pwd_hash
    userid_rootdir_tree: Tree, // K: user_id, V: dir_id

    file_tree: Tree, // K: file_id, V: parent_id, owner_id, name
    dir_tree: Tree,  // K: dir_id, V: parent_id, owner_id, child_number(u16), file/dir_ids..., name
}

impl Database {
    // Initializes the database.
    pub fn init(config: &Config) -> Result<Database, ()> {
        let sled_db =
            sled::open(config.database_location.as_path()).expect("Could not open database.");

        let session_tree = sled_db
            .open_tree(b"sessions")
            .expect("Could not open sessions tree.");
        let username_id_tree = sled_db
            .open_tree(b"usernames_ids")
            .expect("Could not open userids tree.");
        let userid_name_tree = sled_db
            .open_tree(b"userids_names")
            .expect("Could not open username tree.");
        let userid_pwd_tree = sled_db
            .open_tree(b"userids_pwds")
            .expect("Could not open password tree.");
        let userid_rootdir_tree = sled_db
            .open_tree("userid_rootdir")
            .expect("Could not open root-dir tree.");
        let file_tree = sled_db
            .open_tree("files")
            .expect("Could not open file tree.");
        let dir_tree = sled_db
            .open_tree("dirs")
            .expect("Could not open fs-childs tree.");

        Ok(Database {
            _sled_db: sled_db,
            session_tree,
            username_id_tree,
            userid_name_tree,
            userid_pwd_tree,
            userid_rootdir_tree,
            dir_tree,
            file_tree,
        })
    }

    pub fn create_user_session(&self, user_id: u64) -> sled::Result<UserSession> {
        // Generate random session_id:
        let mut rng = thread_rng();
        let mut session_id = rng.next_u64();
        while self.session_tree.contains_key(session_id.to_be_bytes())? {
            session_id = rng.next_u64();
        }

        self.session_tree
            .insert(&session_id.to_be_bytes(), &user_id.to_be_bytes())?;

        Ok(UserSession {
            session_id,
            user_id,
        })
    }

    pub fn get_user_session(&self, session_id: u64) -> sled::Result<Option<UserSession>> {
        let session_id_bytes = session_id.to_be_bytes();

        if let Some(user_id_bytes) = self
            .session_tree
            .get(session_id_bytes)?
            .filter(|bytes| bytes.len() == 8)
        {
            let user_id = u64::from_be_bytes(user_id_bytes.as_ref().try_into().unwrap());

            Ok(Some(UserSession::new(session_id, user_id)))
        } else {
            Ok(None)
        }
    }

    pub fn get_user(&self, user_id: u64) -> sled::Result<Option<User>> {
        let user_id_bytes = user_id.to_be_bytes();

        let username = if let Some(username_bytes) = self.userid_name_tree.get(user_id_bytes)? {
            String::from_utf8(Vec::from(username_bytes.as_ref())).unwrap()
        } else {
            return Ok(None);
        };
        let pwd_hash = if let Some(hash_bytes) = self.userid_pwd_tree.get(user_id_bytes)? {
            String::from_utf8(Vec::from(hash_bytes.as_ref())).unwrap()
        } else {
            return Ok(None);
        };
        let root_dir_id = if let Some(dir_id_bytes) = self.userid_rootdir_tree.get(user_id_bytes)? {
            u64::from_be_bytes(
                dir_id_bytes
                    .as_ref()
                    .try_into()
                    .expect("DB contains invalid root dir."),
            )
        } else {
            return Ok(None);
        };

        Ok(Some(User {
            id: user_id,
            name: username,
            pwd_hash,
            root_dir_id,
        }))
    }

    pub fn get_userid_by_name(&self, username: &str) -> sled::Result<Option<u64>> {
        if let Some(id_bytes) = self.username_id_tree.get(username.as_bytes())? {
            Ok(Some(u64::from_be_bytes(
                id_bytes.as_ref().try_into().unwrap(),
            )))
        } else {
            Ok(None)
        }
    }

    fn get_dirs_childs(&self, dir_id: u64) -> Result<Vec<u64>, Error> {
        if let Some(dir) = self.dir_tree.get(dir_id.to_be_bytes())? {
            let child_number = u16::from_be_bytes(dir[16..18].try_into().unwrap()) as usize;
            let mut child_ids = Vec::with_capacity(child_number);
            let mut i = 18;
            for _ in 0..child_number {
                child_ids.push(u64::from_be_bytes(dir[i..(i + 8)].try_into().unwrap()));
                i += 8;
            }

            Ok(child_ids)
        } else {
            Err(Error::EntryNotFound)
        }
    }

    pub fn get_files_by_parent(&self, parent_id: u64) -> Result<Vec<File>, Error> {
        let child_ids = self.get_dirs_childs(parent_id)?;

        let mut res = Vec::with_capacity(child_ids.len());
        for id in child_ids {
            if let Some(bytes) = self.file_tree.get(id.to_be_bytes())? {
                res.push(File {
                    id,
                    parent_id: u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
                    owner_id: u64::from_be_bytes(bytes[8..16].try_into().unwrap()),
                    name: String::from_utf8(Vec::from(&bytes[16..])).unwrap(),
                });
            }
        }

        Ok(res)
    }

    pub fn get_dirs_by_parent(&self, parent_id: u64) -> Result<Vec<Dir>, Error> {
        let child_ids = self.get_dirs_childs(parent_id)?;

        let mut res = Vec::with_capacity(child_ids.len());
        for id in child_ids {
            if let Some(bytes) = self.dir_tree.get(id.to_be_bytes())? {
                // Get vec of child ids:
                let child_number = u16::from_be_bytes(bytes[16..18].try_into().unwrap()) as usize;
                let mut childs = Vec::with_capacity(child_number);
                let mut i = 18;
                for _ in 0..child_number {
                    childs.push(u64::from_be_bytes(bytes[i..(i + 8)].try_into().unwrap()));
                    i += 8;
                }

                res.push(Dir {
                    id,
                    parent_id: u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
                    owner_id: u64::from_be_bytes(bytes[8..16].try_into().unwrap()),
                    child_ids: childs,
                    name: String::from_utf8(Vec::from(&bytes[i..])).unwrap(),
                });
            }
        }

        Ok(res)
    }
}
