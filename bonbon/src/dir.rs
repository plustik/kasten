use rand::{thread_rng, RngCore};
use sled::{transaction::ConflictableTransactionError, Db, Transactional};

use std::{convert::TryInto, path::PathBuf, string::String};

pub fn dirlist(args: Vec<String>) {
    if args.len() < 3 {
        println!("Usage: bonbon dirlist <db-location>");
        return;
    }

    let sled_db = match open_db(args[2].as_str()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let dir_tree = sled_db
        .open_tree(b"dirs")
        .expect("Could not open dirs tree.");

    for res in &dir_tree {
        match res {
            Ok((id_bytes, dir_bytes)) => {
                let dir_id = u64::from_be_bytes(id_bytes.as_ref().try_into().unwrap());
                let parent_id = u64::from_be_bytes(dir_bytes[0..8].try_into().unwrap());
                let name_start =
                    18 + (8 * u16::from_be_bytes(dir_bytes[16..18].try_into().unwrap()) as usize);
                let dirname = String::from_utf8(Vec::from(&dir_bytes[name_start..])).unwrap();

                println!("{:x} (<- {:x}): \t{}", dir_id, parent_id, dirname);
            }
            Err(e) => {
                println!("Error while reading from DB:\n{}", e);
                return;
            }
        }
    }
}

pub fn diradd(args: Vec<String>) {
    if args.len() < 6 {
        println!("Usage: bonbon diradd <db-location> <dirname> <parent_id> <owner_id>");
        return;
    }
    let name = args[3].clone();
    let parent_id =
        u64::from_str_radix(&args[4], 16).expect("Could not parse parent_id to integer.");
    let owner_id = u64::from_str_radix(&args[5], 16).expect("Could not parse owner_id to integer.");

    let sled_db = match open_db(args[2].as_str()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let dir_tree = sled_db
        .open_tree(b"dirs")
        .expect("Could not open dirs tree.");
    let file_tree = sled_db
        .open_tree(b"files")
        .expect("Could not open files tree.");
    let permissions_tree = sled_db
        .open_tree(b"fs_node_permissions")
        .expect("Could not open permissions tree.");

    // Byte representation of new dir:
    let mut data = Vec::from(&parent_id.to_be_bytes()[..]);
    data.extend_from_slice(&owner_id.to_be_bytes());
    data.push(0); // Child number
    data.push(0); // Child number
    data.extend_from_slice(name.as_bytes());

    (&dir_tree, &file_tree, &permissions_tree)
        .transaction(|(dir_t, file_t, perm_t)| {
            // Generate new dir-id:
            let mut rng = thread_rng();
            let mut dir_id = rng.next_u64();
            while dir_t.get(&dir_id.to_be_bytes())?.is_some()
                || file_t.get(&dir_id.to_be_bytes())?.is_some()
                || dir_id == 0
            {
                dir_id = rng.next_u64();
            }
            let parent_bytes = if let Some(b) = dir_t.get(parent_id.to_be_bytes())? {
                b
            } else {
                return Err(ConflictableTransactionError::Abort(
                    "The parent directory does not exist.",
                ));
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
            dir_t.insert(&parent_id.to_be_bytes(), new_parent_bytes)?;
            // Insert directory into dir-tree:
            dir_t.insert(&dir_id.to_be_bytes(), data.as_slice())?;
            // Insert permissions into permissions-tree:
            perm_t.insert(&dir_id.to_be_bytes(), &[0u8, 0u8, 0u8, 0u8])?;

            Ok(())
        })
        .expect("Could not insert directory: Transaction failed.");
}

fn open_db(location: &str) -> Result<Db, &'static str> {
    let db_location = PathBuf::from(location);
    if !db_location.is_dir() {
        return Err("The given db-location is not a directory.");
    }

    Ok(sled::open(db_location.as_path()).expect("Could not open database."))
}
