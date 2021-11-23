use std::convert::{TryFrom, TryInto};

use rand::{thread_rng, RngCore};
use sled::{
    transaction::ConflictableTransactionError, transaction::ConflictableTransactionResult, Db,
    IVec, Transactional, Tree,
};

use crate::{
    models::{Group, User},
    Error,
};

pub struct UserDatabase {
    username_id_tree: Tree,    // K: username, V: user_id
    userid_name_tree: Tree,    // K: user_id, V: username
    userid_pwd_tree: Tree,     // K: user_id, V: pwd_hash
    userid_rootdir_tree: Tree, // K: user_id, V: dir_id
    user_groups_tree: Tree,    // K: user_id, V: group_ids

    group_tree: Tree, // K: group_id, V: len(member_ids), member_ids, len(admin_ids), admin_ids, name
    groupname_id_tree: Tree, // K: groupname, V: group_id
}

impl UserDatabase {
    /// Initializes the database.
    pub fn init(sled_db: &Db) -> Result<UserDatabase, Error> {
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
        let user_groups_tree = sled_db
            .open_tree("user_groups")
            .expect("Could not open user_groups tree.");

        let group_tree = sled_db
            .open_tree("group")
            .expect("Could not open group tree.");
        let groupname_id_tree = sled_db
            .open_tree(b"groupnames_ids")
            .expect("Could not open groupids tree.");

        Ok(UserDatabase {
            username_id_tree,
            userid_name_tree,
            userid_pwd_tree,
            userid_rootdir_tree,
            user_groups_tree,
            group_tree,
            groupname_id_tree,
        })
    }

    pub fn get_user(&self, user_id: u64) -> Result<Option<User>, Error> {
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
        let mut group_ids = Vec::new();
        if let Some(bytes) = self.user_groups_tree.get(user_id_bytes)? {
            for index in (0..bytes.len()).step_by(8) {
                let id = u64::from_be_bytes(bytes[index..index + 8].try_into().unwrap());
                if self
                    .get_group(id)?
                    .expect("Expected a nonexisting DB entry.")
                    .member_ids
                    .contains(&id)
                {
                    group_ids.push(id);
                }
            }
        }

        Ok(Some(User {
            id: user_id,
            name: username,
            pwd_hash,
            root_dir_id,
            group_ids,
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

    /**
     * Adds the given User with the given fields to the database. If there is already a user with
     * the given ID in the DB, it will be overwritten.
     */
    pub fn insert_user(&self, user: &User) -> Result<(), Error> {
        // Insert data:
        (
            &self.username_id_tree,
            &self.userid_name_tree,
            &self.userid_pwd_tree,
            &self.userid_rootdir_tree,
        )
            .transaction(|(name_id_tt, id_name_tt, pwd_tt, dir_tt)| {
                name_id_tt.insert(user.name.as_bytes(), &user.id.to_be_bytes())?;
                id_name_tt.insert(&user.id.to_be_bytes(), user.name.as_bytes())?;
                pwd_tt.insert(&user.id.to_be_bytes(), user.pwd_hash.as_bytes())?;
                dir_tt.insert(&user.id.to_be_bytes(), &user.root_dir_id.to_be_bytes())?;

                let res: Result<(), ConflictableTransactionError> = Ok(());
                res
            })?;

        Ok(())
    }

    /**
     * Returns the Group given by the ID `group_id`. If no such group exists, `Ok(None)` is
     * returned.
     */
    pub fn get_group(&self, group_id: u64) -> Result<Option<Group>, Error> {
        let group_id_bytes = group_id.to_be_bytes();

        if let Some(bytes) = self.group_tree.get(group_id_bytes)? {
            let mut index = 2;

            let member_count = u16::from_be_bytes(bytes[..2].try_into().unwrap());
            let mut member_ids = Vec::new();
            for _ in 0..member_count {
                member_ids.push(u64::from_be_bytes(
                    bytes[index..(index + 8)].try_into().unwrap(),
                ));
                index += 8;
            }
            let admin_count = u16::from_be_bytes(bytes[index..(index + 2)].try_into().unwrap());
            let mut admin_ids = Vec::new();
            for _ in 0..admin_count {
                admin_ids.push(u64::from_be_bytes(
                    bytes[index..(index + 8)].try_into().unwrap(),
                ));
                index += 8;
            }
            let name = String::from_utf8(Vec::from(&bytes[index..])).unwrap();

            Ok(Some(Group {
                id: group_id,
                name,
                member_ids,
                admin_ids,
            }))
        } else {
            Ok(None)
        }
    }

    /**
     * Adds a new Group with the given fields to the database. The ID of the given Group will be
     * set to a new random and unique value. If the given Groups number of members or admins
     * exceeds the maximum, that can be stored in the DB (`u16::MAX`), `Error::BadCall` is returned.
     * If there already exists a group with the given name in the DB, `Error::TargetExists` is
     * returned. If `group.member_ids` or `group.admin_ids` contains an ID, that does not
     * correspond to an existing user in the DB, `Error::NoSuchTarget` is returned.
     */
    pub fn insert_new_group(&self, group: &mut Group) -> Result<(), Error> {
        group.id = (
            &self.group_tree,
            &self.groupname_id_tree,
            &self.user_groups_tree,
        )
            .transaction(|(group_t, gname_t, user_g_t)| {
                // Make sure the name is unique:
                if gname_t.get(group.name.as_bytes())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(Error::TargetExists));
                }
                // Get new random group id:
                let mut rng = thread_rng();
                let mut group_id = [0u8; 8];
                rng.fill_bytes(&mut group_id);
                while group_t.get(group_id)?.is_some() || user_g_t.get(group_id)?.is_some() {
                    rng.fill_bytes(&mut group_id);
                }

                // Prepare data to insert:
                if group.member_ids.len() > u16::MAX as usize
                    || group.admin_ids.len() > u16::MAX as usize
                {
                    return Err(ConflictableTransactionError::Abort(Error::BadCall));
                }
                let mut data =
                    Vec::from(u16::try_from(group.member_ids.len()).unwrap().to_be_bytes());
                for id in group.member_ids.iter() {
                    data.extend_from_slice(&id.to_be_bytes());
                }
                data.extend_from_slice(
                    &u16::try_from(group.admin_ids.len()).unwrap().to_be_bytes(),
                );
                for id in group.admin_ids.iter() {
                    data.extend_from_slice(&id.to_be_bytes());
                }
                data.extend_from_slice(group.name.as_bytes());

                // Insert data:
                group_t.insert(&group_id, data)?;
                gname_t.insert(group.name.as_bytes(), &group_id)?;

                // Insert reference into users groups:
                'users: for user_id in group.member_ids.iter().chain(group.admin_ids.iter()) {
                    // Make sure the user exists:
                    user_g_t
                        .get(user_id.to_be_bytes())?
                        .ok_or(ConflictableTransactionError::Abort(Error::NoSuchTarget))?;
                    let mut group_list_bytes: Vec<u8> = Vec::from(
                        user_g_t
                            .get(&user_id.to_be_bytes())?
                            .unwrap_or_else(|| IVec::from(&[]))
                            .as_ref(),
                    );
                    // Make sure the user is not already in the list:
                    for i in (0..(group_list_bytes.len())).step_by(8) {
                        if group_list_bytes[i..(i + 8)] == group_id {
                            continue 'users;
                        }
                    }
                    group_list_bytes.extend_from_slice(&user_id.to_be_bytes());
                    user_g_t.insert(&user_id.to_be_bytes(), group_list_bytes.as_slice())?;
                }

                let res: ConflictableTransactionResult<u64, Error> =
                    Ok(u64::from_be_bytes(group_id));
                res
            })?;

        Ok(())
    }

    /**
     * Adds the given Group with the given fields to the database. If there is already a group with
     * the given ID in the DB, it will be overwritten. If there already is a different group with
     * the same name in the DB, `Error::TargetExists` is returned. If the given Groups number of
     * members or admins exceeds the maximum, that can be stored in the DB (`u16::MAX`),
     * `Error::BadCall` is returned.
     */
    pub fn insert_group(&self, group: &Group) -> Result<(), Error> {
        (
            &self.group_tree,
            &self.groupname_id_tree,
            &self.user_groups_tree,
        )
            .transaction(|(group_t, gname_t, user_g_t)| {
                // Make sure the name is unique:
                let id_by_name_result = gname_t.get(group.name.as_bytes())?;
                if id_by_name_result.is_some()
                    && id_by_name_result.unwrap() != group.id.to_be_bytes()
                {
                    return Err(ConflictableTransactionError::Abort(Error::TargetExists));
                }

                // Prepare data to insert:
                if group.member_ids.len() > u16::MAX as usize
                    || group.admin_ids.len() > u16::MAX as usize
                {
                    return Err(ConflictableTransactionError::Abort(Error::BadCall));
                }
                let mut data =
                    Vec::from(u16::try_from(group.member_ids.len()).unwrap().to_be_bytes());
                for id in group.member_ids.iter() {
                    data.extend_from_slice(&id.to_be_bytes());
                }
                data.extend_from_slice(
                    &u16::try_from(group.admin_ids.len()).unwrap().to_be_bytes(),
                );
                for id in group.admin_ids.iter() {
                    data.extend_from_slice(&id.to_be_bytes());
                }
                data.extend_from_slice(group.name.as_bytes());

                // Insert data:
                group_t.insert(&group.id.to_be_bytes(), data)?;
                gname_t.insert(group.name.as_bytes(), &group.id.to_be_bytes())?;

                // Insert reference into users groups:
                'users: for user_id in group.member_ids.iter().chain(group.admin_ids.iter()) {
                    // Make sure the user exists:
                    user_g_t
                        .get(user_id.to_be_bytes())?
                        .ok_or(ConflictableTransactionError::Abort(Error::NoSuchTarget))?;
                    let mut group_list_bytes: Vec<u8> = Vec::from(
                        user_g_t
                            .get(&user_id.to_be_bytes())?
                            .unwrap_or_else(|| IVec::from(&[]))
                            .as_ref(),
                    );
                    // Make sure the user is not already in the list:
                    for i in (0..(group_list_bytes.len())).step_by(8) {
                        if group_list_bytes[i..(i + 8)] == group.id.to_be_bytes() {
                            continue 'users;
                        }
                    }
                    group_list_bytes.extend_from_slice(&user_id.to_be_bytes());
                    user_g_t.insert(&user_id.to_be_bytes(), group_list_bytes.as_slice())?;
                }

                let res: ConflictableTransactionResult<(), Error> = Ok(());
                res
            })?;

        Ok(())
    }
}
