use std::convert::{TryFrom, TryInto};

use rand::{thread_rng, RngCore};
use sled::{transaction::ConflictableTransactionError, Db, Transactional, Tree};

use crate::{
    models::{Dir, File},
    Error,
};

pub struct FsDatabase {
    file_tree: Tree, // K: file_id, V: parent_id, owner_id, name_len, name, type_len, media_type
    dir_tree: Tree,  // K: dir_id, V: parent_id, owner_id, child_number(u16), file/dir_ids..., name
    permissions_tree: Tree, // K: fs_node_id, V: read_group_number (u16), read_group_ids..., write_group_number (u16), write_group_ids...,
}

impl FsDatabase {
    /// Initializes the database.
    pub fn init(sled_db: &Db) -> Result<FsDatabase, Error> {
        let file_tree = sled_db
            .open_tree("files")
            .expect("Could not open file tree.");
        let dir_tree = sled_db
            .open_tree("dirs")
            .expect("Could not open dirs tree.");
        let permissions_tree = sled_db
            .open_tree("fs_node_permissions")
            .expect("Could not open fs-node-permissions tree.");

        Ok(FsDatabase {
            dir_tree,
            file_tree,
            permissions_tree,
        })
    }

    /// Returns the File with the given ID, if it exists in the DB, or None otherwise.
    pub fn get_file(&self, id: u64) -> sled::Result<Option<File>> {
        Ok(self
            .file_tree
            .get(id.to_be_bytes())?
            .zip(self.permissions_tree.get(id.to_be_bytes())?)
            .map(|(file_entry, perm_entry)| {
                let (name, name_len) = parse_db_string(&file_entry[16..]);
                let (media_type, _) = parse_db_string(&file_entry[(16 + name_len)..]);
                File {
                    id,
                    parent_id: u64::from_be_bytes(file_entry[0..8].try_into().unwrap()),
                    owner_id: u64::from_be_bytes(file_entry[8..16].try_into().unwrap()),
                    read_group_ids: parse_read_group_ids(&perm_entry),
                    write_group_ids: parse_write_group_ids(&perm_entry),
                    name,
                    media_type,
                }
            }))
    }

    /// Returns the directory with the given id, it it exists in the DB.
    pub fn get_dir(&self, id: u64) -> Result<Option<Dir>, Error> {
        Ok(self
            .dir_tree
            .get(id.to_be_bytes())?
            .zip(self.permissions_tree.get(id.to_be_bytes())?)
            .map(|(dir_entry, perm_entry)| {
                let mut res = entry_to_dir_incomplete(id, &dir_entry);
                res.read_group_ids = parse_read_group_ids(&perm_entry);
                res.write_group_ids = parse_write_group_ids(&perm_entry);
                res
            }))
    }

    /// Returns the IDs of the given directory's childs.
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

    /// Returns the IDs of all files, that are childs of the given directory.
    pub fn get_files_by_parent(&self, parent_id: u64) -> Result<Vec<File>, Error> {
        let child_ids = self.get_dirs_childs(parent_id)?;

        let mut res = Vec::with_capacity(child_ids.len());
        for id in child_ids {
            if let Some(file) = self.get_file(id)? {
                res.push(file);
            }
        }

        Ok(res)
    }

    /// Returns the IDs of all directories, that are childs of the given directory.
    pub fn get_dirs_by_parent(&self, parent_id: u64) -> Result<Vec<Dir>, Error> {
        let child_ids = self.get_dirs_childs(parent_id)?;

        let mut res = Vec::with_capacity(child_ids.len());
        for id in child_ids {
            if let Some(dir) = self.get_dir(id)? {
                res.push(dir);
            }
        }

        Ok(res)
    }

    /// Checks, whether the DB contains a file with the given id.
    fn is_file(&self, id: u64) -> sled::Result<bool> {
        self.file_tree.contains_key(id.to_be_bytes())
    }

    /// Checks, whether the DB contains a dir with the given id.
    fn is_dir(&self, id: u64) -> sled::Result<bool> {
        self.dir_tree.contains_key(id.to_be_bytes())
    }

    /// Checks, whether the DB contains a file or directory with the given id.
    fn contains_node_id(&self, id: u64) -> sled::Result<bool> {
        Ok(self.is_file(id)? || self.is_dir(id)?)
    }

    /// Inserts a new file with the given attributes in the DB. The ID of the given file will be
    /// updated to a new unique value.
    pub fn insert_new_file(&self, file: &mut File) -> Result<(), Error> {
        if file.read_group_ids.len() > u16::MAX as usize
            || file.write_group_ids.len() > u16::MAX as usize
        {
            return Err(Error::BadCall);
        }

        // Byte representation of new file:
        let mut data = Vec::from(&file.parent_id.to_be_bytes()[..]);
        data.extend_from_slice(&file.owner_id.to_be_bytes());
        string_to_bytes(&file.name, &mut data);
        string_to_bytes(&file.media_type, &mut data);
        // Byte representation of permissions:
        let mut perm_data =
            Vec::with_capacity(4 + 8 * (file.read_group_ids.len() + file.write_group_ids.len()));
        serialize_id_list(file.read_group_ids.as_slice(), &mut perm_data);
        serialize_id_list(file.write_group_ids.as_slice(), &mut perm_data);

        file.id = (&self.file_tree, &self.dir_tree, &self.permissions_tree).transaction(
            |(file_t, dir_t, perm_t)| {
                // Generate new file-id:
                let mut rng = thread_rng();
                let mut file_id = [0u8; 8];
                rng.fill_bytes(&mut file_id);
                while dir_t.get(file_id)?.is_some()
                    || file_t.get(file_id)?.is_some()
                    || u64::from_be_bytes(file_id) == 0
                {
                    rng.fill_bytes(&mut file_id);
                }

                let parent_bytes = if let Some(b) = dir_t.get(file.parent_id.to_be_bytes())? {
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
                new_parent_bytes.extend_from_slice(&file_id);
                // Add name of parent:
                new_parent_bytes
                    .extend_from_slice(&parent_bytes[(18 + (child_number as usize - 1) * 8)..]);

                // Insert new parent directory:
                dir_t.insert(&file.parent_id.to_be_bytes(), new_parent_bytes)?;
                // Insert file into file-tree:
                file_t.insert(&file_id, data.as_slice())?;
                // Insert permissions into permissions-tree:
                perm_t.insert(&file_id, perm_data.as_slice())?;

                Ok(u64::from_be_bytes(file_id))
            },
        )?;

        Ok(())
    }

    /**
     * Changes the properties of the given File in the DB to the values given by the parameter
     * `file`.
     *
     * Changeable properties include `name`, `owner_id`, read_group_ids and write_group_ids,
     * `parent_id` and 'media_type'. The field `id` is used to identify the file to change.
     */
    pub fn update_file(&self, new_file: &File) -> Result<(), Error> {
        // Byte representation of permissions:
        let mut perm_data = Vec::with_capacity(
            4 + 8 * (new_file.read_group_ids.len() + new_file.write_group_ids.len()),
        );
        serialize_id_list(new_file.read_group_ids.as_slice(), &mut perm_data);
        serialize_id_list(new_file.write_group_ids.as_slice(), &mut perm_data);

        (&self.file_tree, &self.permissions_tree).transaction(|(file_t, perm_t)| {
            // Get current version of the file:
            let old_bytes = match file_t.get(new_file.id.to_be_bytes())? {
                Some(b) => b,
                None => {
                    return Err(ConflictableTransactionError::Abort(Error::NoSuchFile));
                }
            };

            let mut new_bytes = Vec::with_capacity(old_bytes.len());
            // Add parent_id:
            new_bytes.extend_from_slice(&new_file.parent_id.to_be_bytes());
            // Add owner_id:
            new_bytes.extend_from_slice(&new_file.owner_id.to_be_bytes());
            // Add name:
            string_to_bytes(&new_file.name, &mut new_bytes);
            // Add media_type:
            string_to_bytes(&new_file.media_type, &mut new_bytes);

            // Insert new File:
            file_t.insert(&new_file.id.to_be_bytes(), new_bytes)?;
            // Insert permissions into permissions-tree:
            perm_t.insert(&new_file.id.to_be_bytes(), perm_data.as_slice())?;

            Ok(())
        })?;

        Ok(())
    }

    /// Removes the file with the given id from the DB and returns its representation. Returns an
    /// Error with type NoSuchFile, if there is no file with the given id in the DB.
    pub fn remove_file(&self, id: u64) -> Result<File, Error> {
        let mut res = (&self.file_tree, &self.dir_tree)
            .transaction(|(file_tt, dir_tt)| {
                // Remove file from file-tree:
                let bytes = match file_tt.remove(&id.to_be_bytes())? {
                    Some(b) => b,
                    None => {
                        return Err(ConflictableTransactionError::Abort(Error::NoSuchFile));
                    }
                };

                let (name, name_len) = parse_db_string(&bytes[16..]);
                let (media_type, _) = parse_db_string(&bytes[(16 + name_len)..]);
                let res = File {
                    id,
                    parent_id: u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
                    owner_id: u64::from_be_bytes(bytes[8..16].try_into().unwrap()),
                    read_group_ids: Vec::new(),
                    write_group_ids: Vec::new(),
                    name,
                    media_type,
                };

                // Remove file from parent:
                // Get current parent representation:
                let mut parent_bytes = match dir_tt.get(res.parent_id.to_be_bytes()) {
                    Ok(Some(b)) => Vec::from(b.as_ref()),
                    Ok(None) => {
                        return Err(ConflictableTransactionError::Abort(
                            Error::InconsistentDbState,
                        ));
                    }
                    Err(e) => {
                        return Err(ConflictableTransactionError::Abort(Error::from(e)));
                    }
                };

                // Decrease child-number:
                let mut child_number = u16::from_be_bytes(parent_bytes[16..18].try_into().unwrap());
                // TODO: Handle overflow:
                child_number -= 1;
                parent_bytes[16] = child_number.to_be_bytes()[0];
                parent_bytes[17] = child_number.to_be_bytes()[1];
                // Add all childs, except the given one:
                for i in 0..(child_number as usize + 1) {
                    let child_id = u64::from_be_bytes(
                        parent_bytes[(18 + i * 8)..(26 + i * 8)].try_into().unwrap(),
                    );
                    if child_id == id {
                        std::mem::drop(parent_bytes.drain((18 + i * 8)..(26 + i * 8)));
                        break;
                    }
                }

                // Insert new parent directory:
                if let Err(e) = dir_tt.insert(&res.parent_id.to_be_bytes(), parent_bytes) {
                    return Err(ConflictableTransactionError::Abort(Error::from(e)));
                }

                Ok(res)
            })
            .map_err(Error::from)?;

        if let Some(perm_bytes) = self.permissions_tree.remove(&id.to_be_bytes())? {
            res.read_group_ids = parse_read_group_ids(&perm_bytes);
            res.write_group_ids = parse_write_group_ids(&perm_bytes);
        }

        Ok(res)
    }

    /// Inserts a new dir with the given attributes in the DB. The ID if the given Dir will be set
    /// to a new unique value.
    pub fn insert_new_dir(&self, dir: &mut Dir) -> Result<(), Error> {
        // Byte representation of new dir:
        let mut data = Vec::from(&dir.parent_id.to_be_bytes()[..]);
        data.extend_from_slice(&dir.owner_id.to_be_bytes());
        data.push(0); // Child number
        data.push(0); // Child number
        data.extend_from_slice(dir.name.as_bytes());
        // Byte representation of permissions:
        let mut perm_data =
            Vec::with_capacity(4 + 8 * (dir.read_group_ids.len() + dir.write_group_ids.len()));
        serialize_id_list(dir.read_group_ids.as_slice(), &mut perm_data);
        serialize_id_list(dir.write_group_ids.as_slice(), &mut perm_data);

        dir.id = (&self.dir_tree, &self.permissions_tree, &self.file_tree).transaction(
            |(dir_t, perm_t, file_t)| {
                // Generate new dir-id:
                let mut rng = thread_rng();
                let mut dir_id = [0u8; 8];
                rng.fill_bytes(&mut dir_id);
                while dir_t.get(dir_id)?.is_some()
                    || file_t.get(dir_id)?.is_some()
                    || u64::from_be_bytes(dir_id) == 0
                {
                    rng.fill_bytes(&mut dir_id);
                }
                let parent_bytes = if let Some(b) = dir_t.get(dir.parent_id.to_be_bytes())? {
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
                new_parent_bytes.extend_from_slice(&dir_id);
                // Add name of parent:
                new_parent_bytes
                    .extend_from_slice(&parent_bytes[(18 + (child_number as usize - 1) * 8)..]);

                // Insert new parent directory:
                dir_t.insert(&dir.parent_id.to_be_bytes(), new_parent_bytes)?;
                // Insert directory into dir-tree:
                dir_t.insert(&dir_id, data.as_slice())?;
                // Insert permissions into permissions-tree:
                perm_t.insert(&dir_id, perm_data.as_slice())?;

                Ok(u64::from_be_bytes(dir_id))
            },
        )?;

        Ok(())
    }

    /**
     * Changes the properties of the given Dir in the DB to the values given by the parameter
     * `dir`.
     *
     * Changeable properties include `name`, `owner_id` and `parent_id`. The field `id` is used to
     * identify the directory to change.     *
     */
    pub fn update_dir(&self, new_dir: &Dir) -> Result<(), Error> {
        // Byte representation of permissions:
        let mut perm_data = Vec::with_capacity(
            4 + 8 * (new_dir.read_group_ids.len() + new_dir.write_group_ids.len()),
        );
        serialize_id_list(new_dir.read_group_ids.as_slice(), &mut perm_data);
        serialize_id_list(new_dir.write_group_ids.as_slice(), &mut perm_data);

        (&self.dir_tree, &self.permissions_tree).transaction(|(dir_t, perm_t)| {
            // Get current version of the dir:
            let old_bytes = match dir_t.get(new_dir.id.to_be_bytes())? {
                Some(b) => b,
                None => {
                    return Err(ConflictableTransactionError::Abort(Error::NoSuchDir));
                }
            };

            let mut new_bytes = Vec::with_capacity(old_bytes.len());
            // Add parent_id:
            new_bytes.extend_from_slice(&new_dir.parent_id.to_be_bytes());
            // Add owner_id:
            new_bytes.extend_from_slice(&new_dir.owner_id.to_be_bytes());
            // Add childs:
            let end_of_childs: usize = (18
                + 8 * (u16::from_be_bytes(old_bytes[16..18].try_into().unwrap())))
            .try_into()
            .unwrap();
            new_bytes.extend_from_slice(&old_bytes[18..end_of_childs]);
            // Add name:
            new_bytes.extend_from_slice(new_dir.name.as_bytes());

            // Insert new Dir:
            dir_t.insert(&new_dir.id.to_be_bytes(), new_bytes)?;
            // Insert permissions into permissions-tree:
            perm_t.insert(&new_dir.id.to_be_bytes(), perm_data.as_slice())?;

            Ok(())
        })?;

        Ok(())
    }

    /**
     * Removes the directory with the given id from the DB and returns its representation.
     * Returns an Error with type NoSuchDir, if there is no directory with the given id in the DB.
     */
    pub fn remove_dir(&self, id: u64) -> Result<Dir, Error> {
        // Return an Err, if it is the root dir of an user:
        if let Some(b) = self.dir_tree.get(id.to_be_bytes())? {
            let dir = entry_to_dir_incomplete(id, &b);
            if dir.parent_id == 0 {
                return Err(Error::ForbiddenAction);
            }
        } else {
            return Err(Error::NoSuchDir);
        };

        let mut res = (&self.dir_tree, &self.file_tree, &self.permissions_tree)
            .transaction(|(dir_t, file_t, perm_t)| {
                let dir = if let Some(b) = self.dir_tree.get(id.to_be_bytes())? {
                    entry_to_dir_incomplete(id, &b)
                } else {
                    return Err(ConflictableTransactionError::Abort(Error::NoSuchDir));
                };

                // Remove directory from parent:
                // Get current parent representation:
                let mut parent_bytes = match dir_t.get(dir.parent_id.to_be_bytes()) {
                    Ok(Some(b)) => Vec::from(b.as_ref()),
                    Ok(None) => {
                        return Err(ConflictableTransactionError::Abort(Error::NoSuchDir));
                    }
                    Err(e) => {
                        return Err(ConflictableTransactionError::Abort(Error::from(e)));
                    }
                };

                // Decrease child-number:
                let mut child_number = u16::from_be_bytes(parent_bytes[16..18].try_into().unwrap());
                // TODO: Handle overflow:
                child_number -= 1;
                parent_bytes[16] = child_number.to_be_bytes()[0];
                parent_bytes[17] = child_number.to_be_bytes()[1];
                // Add all childs, except the given one:
                for i in 0..(child_number as usize + 1) {
                    let child_id = u64::from_be_bytes(
                        parent_bytes[(18 + i * 8)..(26 + i * 8)].try_into().unwrap(),
                    );
                    if child_id == id {
                        std::mem::drop(parent_bytes.drain((18 + i * 8)..(26 + i * 8)));
                        break;
                    }
                }

                // Insert new parent directory:
                if let Err(e) = dir_t.insert(&dir.parent_id.to_be_bytes(), parent_bytes) {
                    return Err(ConflictableTransactionError::Abort(Error::from(e)));
                }

                // Remove childs from DB:
                let mut todo_stack: Vec<u64> = dir.child_ids.clone();

                while !todo_stack.is_empty() {
                    let next_id = todo_stack.pop().unwrap();
                    if let Some(bytes) = dir_t.remove(&next_id.to_be_bytes())? {
                        let dir = entry_to_dir_incomplete(next_id, &bytes);
                        for ch in dir.child_ids {
                            todo_stack.push(ch);
                        }
                    } else {
                        file_t.remove(&next_id.to_be_bytes())?;
                        // TODO: Remove the file from storage
                    }
                    perm_t.remove(&next_id.to_be_bytes())?;
                }

                Ok(dir)
            })
            .map_err(Error::from)?;

        if let Some(perm_bytes) = self.permissions_tree.remove(&id.to_be_bytes())? {
            res.read_group_ids = parse_read_group_ids(&perm_bytes);
            res.write_group_ids = parse_write_group_ids(&perm_bytes);
        }

        Ok(res)
    }

    /**
     * Adds the group Id `group_id` to the list of readable groups for the file or directory given
     * by `fs_node_id`.
     *
     * If there is no entry for a FsNode with the given ID in the permission table,
     * `Err(Error::NoSuchTarget)` is returned.
     */
    pub fn add_readable_group(&self, fs_node_id: u64, group_id: u64) -> Result<(), Error> {
        self.permissions_tree
            .transaction(|perm_t| {
                let old_bytes = perm_t
                    .get(&fs_node_id.to_be_bytes())?
                    .ok_or(ConflictableTransactionError::Abort(Error::NoSuchTarget))?;
                let mut new_bytes = Vec::with_capacity(old_bytes.len());
                // Increase number of IDs:
                let new_len = 1 + u16::from_be_bytes(old_bytes[..2].try_into().unwrap()); // TODO: Handle overflow
                new_bytes.extend_from_slice(&new_len.to_be_bytes());
                let second_list_start: usize =
                    (2 + u16::from_be_bytes(old_bytes[..2].try_into().unwrap()) * 8)
                        .try_into()
                        .unwrap();
                new_bytes.extend_from_slice(&old_bytes[2..second_list_start]);
                // Append new ID:
                new_bytes.extend_from_slice(&group_id.to_be_bytes());
                new_bytes.extend_from_slice(&old_bytes[second_list_start..]);
                // Write changes:
                perm_t.insert(&fs_node_id.to_be_bytes(), new_bytes.as_slice())?;

                Ok(())
            })
            .map_err(Error::from)
    }

    /**
     * Adds the group Id `group_id` to the list of writeable groups for the file or directory given
     * by `fs_node_id`.
     *
     * If there is no entry for a FsNode with the given ID in the permission table,
     * `Err(Error::NoSuchTarget)` is returned.
     */
    pub fn add_writeable_group(&self, fs_node_id: u64, group_id: u64) -> Result<(), Error> {
        self.permissions_tree
            .transaction(|perm_t| {
                let old_bytes = perm_t
                    .get(&fs_node_id.to_be_bytes())?
                    .ok_or(ConflictableTransactionError::Abort(Error::NoSuchTarget))?;
                let mut new_bytes = Vec::from(old_bytes.as_ref());
                // Increase number of IDs:
                let second_list_start: usize =
                    (2 + u16::from_be_bytes(old_bytes[..2].try_into().unwrap()) * 8)
                        .try_into()
                        .unwrap();
                let new_len = 1 + u16::from_be_bytes(
                    old_bytes[second_list_start..(second_list_start + 2)]
                        .try_into()
                        .unwrap(),
                ); // TODO: Handle overflow
                new_bytes[second_list_start] = new_len.to_be_bytes()[0];
                new_bytes[second_list_start + 1] = new_len.to_be_bytes()[1];
                // Append new ID:
                new_bytes.extend_from_slice(&group_id.to_be_bytes());
                // Write changes:
                perm_t.insert(&fs_node_id.to_be_bytes(), new_bytes.as_slice())?;

                Ok(())
            })
            .map_err(Error::from)
    }
}

//
// Helper functions for serialization and deserialization:
//

fn parse_read_group_ids(bytes: &[u8]) -> Vec<u64> {
    parse_id_list(bytes)
}
fn parse_write_group_ids(bytes: &[u8]) -> Vec<u64> {
    let start = 2 + 8 * (u16::from_be_bytes(bytes[0..2].try_into().unwrap()) as usize);
    parse_id_list(&bytes[start..])
}

fn parse_id_list(bytes: &[u8]) -> Vec<u64> {
    let list_len = u16::from_be_bytes(bytes[0..2].try_into().unwrap())
        .try_into()
        .unwrap();
    let mut id_list = Vec::with_capacity(list_len);
    for i in (2..(2 + 8 * list_len)).step_by(8) {
        id_list.push(u64::from_be_bytes(bytes[i..(i + 8)].try_into().unwrap()));
    }
    id_list
}

fn serialize_id_list(list: &[u64], buf: &mut Vec<u8>) {
    buf.extend_from_slice(
        &u16::try_from(list.len())
            .expect("A given list was to long for the DB.")
            .to_be_bytes(),
    );
    for id in list {
        buf.extend_from_slice(&id.to_be_bytes());
    }
}

fn entry_to_dir_incomplete(id: u64, bytes: &[u8]) -> Dir {
    let parent_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
    let owner_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
    let child_number = u16::from_be_bytes(bytes[16..18].try_into().unwrap()) as usize;

    let mut child_ids = Vec::with_capacity(child_number);
    for i in 0..child_number {
        child_ids.push(u64::from_be_bytes(
            bytes[(20 + i * 8)..(28 + i * 8)].try_into().unwrap(),
        ));
    }

    let name = String::from_utf8(Vec::from(&bytes[(18 + child_number * 8)..]))
        .expect("DB contained Dir with non-UTF-8 name.");

    Dir {
        id,
        parent_id,
        owner_id,
        read_group_ids: vec![],
        write_group_ids: vec![],
        child_ids,
        name,
    }
}

fn parse_db_string(bytes: &[u8]) -> (String, usize) {
    let length = u16::from_be_bytes(bytes[0..2].try_into().unwrap()) as usize;

    (
        String::from_utf8(Vec::from(&bytes[2..(2 + length)]))
            .expect("DB contains non-UTF-8 string."),
        length + 2,
    )
}
fn string_to_bytes(string: &str, buf: &mut Vec<u8>) {
    let length: u16 = string
        .as_bytes()
        .len()
        .try_into()
        .expect("Trying to write a string to the DB, that is to long for u16.");
    buf.extend_from_slice(&length.to_be_bytes());
    buf.extend_from_slice(string.as_bytes());
}
