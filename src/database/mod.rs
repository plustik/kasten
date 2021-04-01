use std::convert::TryInto;

use chrono::offset::{TimeZone, Utc};
use rand::{thread_rng, RngCore};
use sled::{transaction::ConflictableTransactionError, Db, Transactional, Tree};

use crate::{
    config::Config,
    models::{Dir, File, User, UserSession},
    Error,
};

pub struct Database {
    _sled_db: Db,
    session_tree: Tree,        // K: session_id, V: user_id, createn_date
    user_session_tree: Tree,   // K: user_id, session_id
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
        let user_session_tree = sled_db
            .open_tree(b"user_sessions")
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
            user_session_tree,
            username_id_tree,
            userid_name_tree,
            userid_pwd_tree,
            userid_rootdir_tree,
            dir_tree,
            file_tree,
        })
    }

    /// Creates a new session for the given user, inserts the session into the DB and returns it.
    pub fn create_user_session(&self, user_id: u64) -> Result<UserSession, Error> {
        // Generate random session_id:
        let mut rng = thread_rng();
        let mut session_id = rng.next_u64();
        while self.session_tree.contains_key(session_id.to_be_bytes())? {
            session_id = rng.next_u64();
        }

        // Create entry for session-tree:
        let mut session_content = Vec::from(user_id.to_be_bytes());
        let creation_date = Utc::now();
        session_content.extend_from_slice(&creation_date.timestamp().to_be_bytes());
        // Create key for user-session-tree:
        let mut user_session_key = Vec::from(user_id.to_be_bytes());
        user_session_key.extend_from_slice(&session_id.to_be_bytes());

        // Insert data:
        (&self.session_tree, &self.user_session_tree).transaction(|(session_tt, user_tt)| {
            let res: Result<(), ConflictableTransactionError> = Ok(());
            session_tt.insert(&session_id.to_be_bytes(), session_content.as_slice())?;
            user_tt.insert(user_session_key.as_slice(), &[])?;

            res
        })?;

        Ok(UserSession::new(session_id, user_id, creation_date))
    }

    /// Removes the user session with the given id from the DB. If no such session exists in the
    /// DB, it will still return Ok(()).
    pub fn remove_user_session(&self, session_id: u64) -> Result<(), Error> {
        (&self.session_tree, &self.user_session_tree).transaction(|(session_tt, user_tt)| {
            let res: Result<(), ConflictableTransactionError> = Ok(());

            if let Some(v) = session_tt.remove(&session_id.to_be_bytes())? {
                // Create key for user-session-tree:
                let mut user_session_key = Vec::from(&v[0..8]);
                user_session_key.extend_from_slice(&session_id.to_be_bytes());
                user_tt.remove(user_session_key)?;
            }

            res
        })?;

        Ok(())
    }

    /// Iterates over all sessions of the given user and removes all sessions, for which the given
    /// filter function returns false.
    pub fn filter_user_sessions<P>(&self, user_id: u64, mut filter_fn: P) -> Result<(), Error>
    where
        P: FnMut(UserSession) -> bool,
    {
        for res in self.user_session_tree.scan_prefix(user_id.to_be_bytes()) {
            if let Ok((key, _)) = res {
                // Get session from key:
                let session_id = u64::from_be_bytes(key[8..16].try_into().unwrap());
                let session = self.get_user_session(session_id)?.unwrap();

                // Test, whether to remove the session:
                if !filter_fn(session) {
                    self.remove_user_session(session_id)?;
                }
            } else {
                return Err(Error::from(res.unwrap_err()));
            }
        }

        Ok(())
    }

    /// Returns the user session with the given id, if it exists in the DB.
    pub fn get_user_session(&self, session_id: u64) -> Result<Option<UserSession>, Error> {
        let session_id_bytes = session_id.to_be_bytes();

        let bytes = if let Some(b) = self.session_tree.get(session_id_bytes)? {
            b
        } else {
            return Ok(None);
        };

        let user_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let creation_date = Utc.timestamp(i64::from_be_bytes(bytes[8..16].try_into().unwrap()), 0);

        Ok(Some(UserSession::new(session_id, user_id, creation_date)))
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

    pub fn get_file(&self, id: u64) -> sled::Result<Option<File>> {
        self.file_tree.get(id.to_be_bytes()).map(|opt| {
            opt.map(|bytes| File {
                id,
                parent_id: u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
                owner_id: u64::from_be_bytes(bytes[8..16].try_into().unwrap()),
                name: String::from_utf8(Vec::from(&bytes[16..])).unwrap(),
            })
        })
    }


    /// Returns the directory with the given id, it it exists in the DB.
    pub fn get_dir(&self, id: u64) -> sled::Result<Option<Dir>> {
        self.dir_tree.get(&id.to_be_bytes())
            .map(|opt| { opt.map(|bytes| {
                let parent_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
                let owner_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
                let child_number: usize = u16::from_be_bytes(bytes[16..18].try_into().unwrap()) as usize;
                let mut child_ids = Vec::with_capacity(child_number);
                for i in 0..child_number {
                    child_ids.push(u64::from_be_bytes(bytes[(18 + i * 8)..(26 + i * 8)].try_into().unwrap()));
                }
                let name = String::from_utf8(Vec::from(&bytes[(26 + child_number * 8)..])).unwrap();

                Dir {
                    id,
                    parent_id,
                    owner_id,
                    child_ids,
                    name,
                }
            }) })
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


    // Checks, whether the DB contains a file or directory with the given id.
    fn contains_node_id(&self, id: u64) -> sled::Result<bool> {
        Ok(self.file_tree.contains_key(id.to_be_bytes())? || self.dir_tree.contains_key(id.to_be_bytes())?)
    }

    /// Inserts a new file with the given attributes in the DB.
    /// If no errors occour, a representaion of the new file is returned.
    pub fn insert_new_file(
        &self,
        parent_id: u64,
        owner_id: u64,
        name: &str,
    ) -> Result<File, Error> {
        // Generate new file-id:
        let mut rng = thread_rng();
        let mut file_id = rng.next_u64();
        while self.contains_node_id(file_id)? {
            file_id = rng.next_u64();
        }

        // Byte representation of new file:
        let mut data = Vec::from(&parent_id.to_be_bytes()[..]);
        data.extend_from_slice(&owner_id.to_be_bytes());
        data.extend_from_slice(name.as_bytes());

        (&self.file_tree, &self.dir_tree).transaction(|(file_tt, dir_tt)| {
            let parent_bytes = if let Some(b) = dir_tt.get(parent_id.to_be_bytes())? {
                b
            } else {
                return Err(ConflictableTransactionError::Abort(Error::NoSuchDir));
            };
            let mut new_parent_bytes = Vec::from(&parent_bytes[0..16]);

            // Increase child-number:
            let mut child_number = u16::from_be_bytes(parent_bytes[16..18].try_into().unwrap());
            // TODO: Handle overflow:
            child_number += 1;
            new_parent_bytes.push(child_number.to_be_bytes()[0]);
            new_parent_bytes.push(child_number.to_be_bytes()[1]);
            // Add old childs:
            new_parent_bytes
                .extend_from_slice(&parent_bytes[18..(18 + (child_number as usize - 1) * 8)]);
            // Add new child:
            new_parent_bytes.extend_from_slice(&file_id.to_be_bytes());
            // Add name of parent:
            new_parent_bytes
                .extend_from_slice(&parent_bytes[(18 + (child_number as usize - 1) * 8)..]);

            // Insert new parent directory:
            dir_tt.insert(&parent_id.to_be_bytes(), new_parent_bytes)?;

            // Insert file into file-tree:
            file_tt.insert(&file_id.to_be_bytes(), data.as_slice())?;

            Ok(())
        })?;

        Ok(File {
            id: file_id,
            parent_id,
            owner_id,
            name: String::from(name),
        })
    }

    /// Inserts a new dir with the given attributes in the DB.
    /// If no errors occour, a representaion of the new dir is returned.
    pub fn insert_new_dir(
        &self,
        parent_id: u64,
        owner_id: u64,
        name: &str,
    ) -> Result<Dir, Error> {
        // Generate new dir-id:
        let mut rng = thread_rng();
        let mut dir_id = rng.next_u64();
        while self.contains_node_id(dir_id)? {
            dir_id = rng.next_u64();
        }

        // Byte representation of new dir:
        let mut data = Vec::from(&parent_id.to_be_bytes()[..]);
        data.extend_from_slice(&owner_id.to_be_bytes());
        data.push(0);   // Child number
        data.push(0);   // Child number
        data.extend_from_slice(name.as_bytes());

        self.dir_tree.transaction(|dir_tt| {
            let parent_bytes = if let Some(b) = dir_tt.get(parent_id.to_be_bytes())? {
                b
            } else {
                return Err(ConflictableTransactionError::Abort(Error::NoSuchDir));
            };
            let mut new_parent_bytes = Vec::from(&parent_bytes[0..16]);

            // Increase child-number:
            let mut child_number = u16::from_be_bytes(parent_bytes[16..18].try_into().unwrap());
            // TODO: Handle overflow:
            child_number += 1;
            new_parent_bytes.push(child_number.to_be_bytes()[0]);
            new_parent_bytes.push(child_number.to_be_bytes()[1]);
            // Add old childs:
            new_parent_bytes
                .extend_from_slice(&parent_bytes[18..(18 + (child_number as usize - 1) * 8)]);
            // Add new child:
            new_parent_bytes.extend_from_slice(&dir_id.to_be_bytes());
            // Add name of parent:
            new_parent_bytes
                .extend_from_slice(&parent_bytes[(18 + (child_number as usize - 1) * 8)..]);

            // Insert new parent directory:
            dir_tt.insert(&parent_id.to_be_bytes(), new_parent_bytes)?;

            // Insert directory into dir-tree:
            dir_tt.insert(&dir_id.to_be_bytes(), data.as_slice())?;

            Ok(())
        })?;

        Ok(Dir {
            id: dir_id,
            parent_id,
            owner_id,
            child_ids: Vec::new(),
            name: String::from(name),
        })
    }
}
