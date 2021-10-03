use std::convert::TryInto;

use chrono::offset::{TimeZone, Utc};
use rand::{thread_rng, RngCore};
use sled::{transaction::ConflictableTransactionError, Db, Transactional, Tree};

use crate::{
    config::Config,
    models::{Dir, File, Group, User, UserSession},
    Error,
};

mod fs_db;
use fs_db::FsDatabase;
mod user_db;
use user_db::UserDatabase;

pub struct Database {
    _sled_db: Db,
    session_tree: Tree,      // K: session_id, V: user_id, creation_date
    user_session_tree: Tree, // K: user_id, session_id

    fs_db: FsDatabase,
    user_db: UserDatabase,
}

impl Database {
    /// Initializes the database.
    pub fn init(config: &Config) -> Result<Database, Error> {
        let sled_db =
            sled::open(config.database_location.as_path()).expect("Could not open database.");

        let session_tree = sled_db
            .open_tree(b"sessions")
            .expect("Could not open sessions tree.");
        let user_session_tree = sled_db
            .open_tree(b"user_sessions")
            .expect("Could not open sessions tree.");

        let fs_db = FsDatabase::init(&sled_db)?;
        let user_db = UserDatabase::init(&sled_db)?;

        Ok(Database {
            _sled_db: sled_db,
            session_tree,
            user_session_tree,
            fs_db,
            user_db,
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
        self.user_db.get_user(user_id)
    }

    pub fn get_userid_by_name(&self, username: &str) -> sled::Result<Option<u64>> {
        self.user_db.get_userid_by_name(username)
    }

    /**
     * Adds the given User with the given fields to the database. If there is already a user with
     * the given ID in the DB, it will be overwritten.
     */
    pub fn insert_user(&self, user: &User) -> Result<(), Error> {
        self.user_db.insert_user(user)
    }

    /**
     * Returns the Group given by the ID `group_id`. If no such group exists, `Ok(None)` is
     * returned.
     */
    pub fn get_group(&self, group_id: u64) -> Result<Option<Group>, Error> {
        self.user_db.get_group(group_id)
    }

    /**
     * If there is a group in the DB with the name given by `groupname`, its ID is returned.
     * Otherwise `Ok(None)` is retuned.
     */
    pub fn get_groupid_by_name(&self, groupname: &str) -> sled::Result<Option<u64>> {
        self.user_db.get_groupid_by_name(groupname)
    }

    /**
     * Adds a new Group with the given fields to the database. The ID of the given Group will be
     * set to a new random and unique value.
     */
    pub fn insert_new_group(&self, group: &mut Group) -> Result<(), Error> {
        self.user_db.insert_new_group(group)
    }

    /**
     * Adds the given Group with the given fields to the database. If there is already a group with
     * the given ID in the DB, it will be overwritten. If there already is a different group with
     * the same name in the DB, `Error::TargetExists` is returned. If the given Groups number of
     * members or admins exceeds the maximum, that can be stored in the DB (`u16::MAX`),
     * `Error::BadCall` is returned.
     */
    pub fn insert_group(&self, group: &Group) -> Result<(), Error> {
        self.user_db.insert_group(group)
    }

    /// Returns the File with the given ID, if it exists in the DB, or None otherwise.
    pub fn get_file(&self, id: u64) -> sled::Result<Option<File>> {
        self.fs_db.get_file(id)
    }

    /// Returns the directory with the given id, it it exists in the DB.
    pub fn get_dir(&self, id: u64) -> Result<Option<Dir>, Error> {
        self.fs_db.get_dir(id)
    }

    /// Returns the IDs of all files, that are childs of the given directory.
    pub fn get_files_by_parent(&self, parent_id: u64) -> Result<Vec<File>, Error> {
        self.fs_db.get_files_by_parent(parent_id)
    }

    /// Returns the IDs of all directories, that are childs of the given directory.
    pub fn get_dirs_by_parent(&self, parent_id: u64) -> Result<Vec<Dir>, Error> {
        self.fs_db.get_dirs_by_parent(parent_id)
    }

    /**
     * Inserts the given File into the DB.
     * The function finds a new id for the File and updates the id field accordingly.
     */
    pub fn insert_new_file(&self, file: &mut File) -> Result<(), Error> {
        // TODO: Refactor fs_db.insert_new_file to take a &mut File as argument too.
        file.id = self
            .fs_db
            .insert_new_file(file.parent_id, file.owner_id, file.name.as_str())?
            .id;
        Ok(())
    }

    /**
     * Changes the properties of the given File in the DB to the values given by the parameter
     * `file`.
     *
     * Changeable properties include `name`, `owner_id` and `parent_id`. The field `id` is used to
     * identify the file to change.
     */
    pub fn update_file(&self, file: &File) -> Result<(), Error> {
        self.fs_db.update_file(file)
    }

    /// Removes the file with the given id from the DB and returns its representation. Returns an
    /// Error with type NoSuchFile, if there is no file with the given id in the DB.
    pub fn remove_file(&self, id: u64) -> Result<File, Error> {
        self.fs_db.remove_file(id)
    }

    /**
     * Inserts the given Dir into the DB.
     * The function finds a new id for the Dir and updates the id field accordingly.
     * Ids in the child_ids Vec will be ignored and not written to the DB.
     */
    pub fn insert_new_dir(&self, dir: &mut Dir) -> Result<(), Error> {
        // TODO: Refactor fs_db.insert_new_dir to take a &mut Dir as argument too.
        dir.id = self
            .fs_db
            .insert_new_dir(dir.parent_id, dir.owner_id, dir.name.as_str())?
            .id;
        Ok(())
    }

    /**
     * Changes the properties of the given Dir in the DB to the values given by the parameter
     * `dir`.
     *
     * Changeable properties include `name`, `owner_id` and `parent_id`. The field `id` is used to
     * identify the directory to change. The field `child_ids` will be ignored.
     */
    pub fn update_dir(&self, dir: &Dir) -> Result<(), Error> {
        self.fs_db.update_dir(dir)
    }

    /// Removes the directory with the given id from the DB and returns its representation.
    /// Returns an Error with type NoSuchDir, if there is no directory with the given id in the DB.
    pub fn remove_dir(&self, id: u64) -> Result<Dir, Error> {
        self.fs_db.remove_dir(id)
    }
}
