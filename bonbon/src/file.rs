use rand::{thread_rng, RngCore};
use sled::{transaction::ConflictableTransactionError, Db, Transactional};

use std::{convert::TryInto, path::PathBuf, string::String};

pub fn filelist(args: Vec<String>) {
    if args.len() < 3 {
        println!("Usage: bonbon filelist <db-location>");
        return;
    }

    let sled_db = match open_db(args[2].as_str()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let file_tree = sled_db
        .open_tree(b"files")
        .expect("Could not open files tree.");

    for res in &file_tree {
        match res {
            Ok((id_bytes, file_bytes)) => {
                let file_id = u64::from_be_bytes(id_bytes.as_ref().try_into().unwrap());
                let parent_id = u64::from_be_bytes(file_bytes[0..8].try_into().unwrap());
                let filename = String::from_utf8(Vec::from(&file_bytes[16..])).unwrap();

                println!("{:x} (<- {:x}): \t{}", file_id, parent_id, filename);
            }
            Err(e) => {
                println!("Error while reading from DB:\n{}", e);
                return;
            }
        }
    }
}

pub fn filerm(args: Vec<String>) {
    if args.len() < 4 {
        println!("Usage: bonbon filerm <db-location> <file-id>");
        return;
    }

    let id = if let Ok(v) = u64::from_str_radix(args[3].as_str(), 16) {
        v.to_be_bytes()
    } else {
        println!("Could not parse file ID.");
        return;
    };

    let sled_db = match open_db(args[2].as_str()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let file_tree = sled_db
        .open_tree(b"files")
        .expect("Could not open files tree.");
    let permissions_tree = sled_db
        .open_tree(b"fs_node_permissions")
        .expect("Could not open files tree.");

    if let Err(_) = file_tree.remove(id) {
        println!("Error while removing from files-tree.");
    }
    if let Err(_) = permissions_tree.remove(id) {
        println!("Error while removing from fs_node_permissions-tree.");
    }
}

fn open_db(location: &str) -> Result<Db, &'static str> {
    let db_location = PathBuf::from(location);
    if !db_location.is_dir() {
        return Err("The given db-location is not a directory.");
    }

    Ok(sled::open(db_location.as_path()).expect("Could not open database."))
}
