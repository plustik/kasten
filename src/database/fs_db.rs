use std::convert::TryInto;

use rand::{thread_rng, RngCore};
use sled::{transaction::ConflictableTransactionError, Db, Transactional, Tree};

use crate::{
    models::{Dir, File},
    Error,
};


pub struct FsDatabase {
    file_tree: Tree, // K: file_id, V: parent_id, owner_id, name
    dir_tree: Tree,  // K: dir_id, V: parent_id, owner_id, child_number(u16), file/dir_ids..., name
}

impl FsDatabase {
    /// Initializes the database.
    pub fn init(sled_db: &Db) -> Result<FsDatabase, Error> {
        let file_tree = sled_db
            .open_tree("files")
            .expect("Could not open file tree.");
        let dir_tree = sled_db
            .open_tree("dirs")
            .expect("Could not open fs-childs tree.");

        Ok(FsDatabase {
            dir_tree,
            file_tree,
        })
    }

    /// Returns the File with the given ID, if it exists in the DB, or None otherwise.
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
    pub fn get_dir(&self, id: u64) -> Result<Option<Dir>, Error> {
        match self.dir_tree.get(&id.to_be_bytes()) {
            Ok(Some(bytes)) => Dir::from_db_entry(id, &bytes).map(|res| Some(res)),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::from(e)),
        }
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

    /// Returns the IDs of all directories, that are childs of the given directory.
    pub fn get_dirs_by_parent(&self, parent_id: u64) -> Result<Vec<Dir>, Error> {
        let child_ids = self.get_dirs_childs(parent_id)?;

        let mut res = Vec::with_capacity(child_ids.len());
        for id in child_ids {
            if let Some(bytes) = self.dir_tree.get(id.to_be_bytes())? {
                let dir = Dir::from_db_entry(id, &bytes)?;
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
        Ok(self.is_file(id)?
            || self.is_dir(id)?)
    }

    /// Inserts a new file with the given attributes in the DB.
    /// If no errors occour, a representation of the new file is returned.
    pub fn insert_new_file(
        &self,
        parent_id: u64,
        owner_id: u64,
        name: &str,
    ) -> Result<File, Error> {
        // Generate new file-id:
        let mut rng = thread_rng();
        let mut file_id = rng.next_u64();
        while self.contains_node_id(file_id)? || file_id == 0 {
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

    /// Removes the file with the given id from the DB and returns its representation. Returns an
    /// Error with type NoSuchFile, if there is no file with the given id in the DB.
    pub fn remove_file(&self, id: u64) -> Result<File, Error> {
        (&self.file_tree, &self.dir_tree)
            .transaction(|(file_tt, dir_tt)| {
                // Remove file from file-tree:
                let bytes = match file_tt.remove(&id.to_be_bytes())? {
                    Some(b) => b,
                    None => {
                        return Err(ConflictableTransactionError::Abort(Error::NoSuchFile));
                    }
                };

                let res = File {
                    id,
                    parent_id: u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
                    owner_id: u64::from_be_bytes(bytes[8..16].try_into().unwrap()),
                    name: String::from_utf8(Vec::from(&bytes[16..])).unwrap(),
                };

                // Remove file from parent:
                // Get current parent representation:
                let mut parent_bytes = match dir_tt.get(res.parent_id.to_be_bytes()) {
                    Ok(Some(b)) => Vec::from(b.as_ref()),
                    Ok(None) => {
                        return Err(ConflictableTransactionError::Abort(Error::InconsistentDbState));
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
            .map_err(|e| Error::from(e))
    }

    /// Inserts a new dir with the given attributes in the DB.
    /// If no errors occour, a representation of the new dir is returned.
    pub fn insert_new_dir(&self, parent_id: u64, owner_id: u64, name: &str) -> Result<Dir, Error> {
        // Generate new dir-id:
        let mut rng = thread_rng();
        let mut dir_id = rng.next_u64();
        while self.contains_node_id(dir_id)? || dir_id == 0 {
            dir_id = rng.next_u64();
        }

        // Byte representation of new dir:
        let mut data = Vec::from(&parent_id.to_be_bytes()[..]);
        data.extend_from_slice(&owner_id.to_be_bytes());
        data.push(0); // Child number
        data.push(0); // Child number
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

    /// Removes the directory with the given id from the DB and returns its representation.
    /// Returns an Error with type NoSuchDir, if there is no directory with the given id in the DB.
    pub fn remove_dir(&self, id: u64) -> Result<Dir, Error> {
        // Return an Err, if it is the root dir of an user:
        if let Some(b) = dbg!(self.dir_tree.get(id.to_be_bytes()))? {
            let dir = dbg!(Dir::from_db_entry(id, &b))?;
            if dir.parent_id == 0 {
                return Err(Error::ForbiddenAction);
            }
        } else {
            return Err(Error::NoSuchDir);
        };

        dbg!("test l310");
        (&self.dir_tree, &self.file_tree).transaction(|(dir_tt, file_tt)| {
            let dir = if let Some(b) = dbg!(self.dir_tree.get(id.to_be_bytes()))? {
                dbg!(Dir::from_db_entry(id, &b).map_err(|e| ConflictableTransactionError::Abort(e))?)
            } else {
                return Err(ConflictableTransactionError::Abort(Error::NoSuchDir));
            };

            // Remove directory from parent:
            // Get current parent representation:
            let mut parent_bytes = match dir_tt.get(dir.parent_id.to_be_bytes()) {
                Ok(Some(b)) => Vec::from(b.as_ref()),
                Ok(None) => {
                    return Err(ConflictableTransactionError::Abort(Error::NoSuchDir));
                }
                Err(e) => {
                    return Err(ConflictableTransactionError::Abort(Error::from(e)));
                }
            };

            // Decrease child-number:
            let mut child_number = dbg!(u16::from_be_bytes(parent_bytes[16..18].try_into().unwrap()));
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
            if let Err(e) = dir_tt.insert(&dir.parent_id.to_be_bytes(), parent_bytes) {
                return Err(ConflictableTransactionError::Abort(Error::from(e)));
            }

            // Remove childs from DB:
            let mut todo_stack: Vec<u64> = dir.child_ids.iter().map(|int_ref| *int_ref).collect();

            while !todo_stack.is_empty() {
                let next_id = todo_stack.pop().unwrap();
                dbg!(next_id);
                if let Some(bytes) = dir_tt.remove(&next_id.to_be_bytes())? {
                    let dir = dbg!(Dir::from_db_entry(next_id, &bytes).map_err(|e| ConflictableTransactionError::Abort(e))?);
                    for ch in dir.child_ids {
                        todo_stack.push(ch);
                    }
                } else {
                    file_tt.remove(&next_id.to_be_bytes())?;
                }
            }

            Ok(dir)
        }).map_err(|err| Error::from(err))
    }
}
