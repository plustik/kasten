use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use rand::{thread_rng, RngCore};
use sled::{
    transaction::{ConflictableTransactionResult, UnabortableTransactionError},
    Db, Transactional,
};

use std::{convert::TryInto, path::PathBuf, string::String};

pub fn useradd(args: Vec<String>) {
    if args.len() < 5 {
        println!("Usage: bonbon useradd <db-location> <username> <password>");
        return;
    }

    let sled_db = match open_db(args[2].as_str()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
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
        .open_tree(b"userid_rootdir")
        .expect("Could not open root dir tree.");
    let dir_tree = sled_db
        .open_tree(b"dirs")
        .expect("Could not open directory tree.");
    let file_tree = sled_db
        .open_tree(b"files")
        .expect("Could not open file tree.");

    // Check if username allready exists:
    let username = args[3].as_bytes();
    match username_id_tree.contains_key(username) {
        Ok(true) => {
            println!("The given username allready exists.");
            return;
        }
        Err(e) => {
            println!("Could not access id-tree: {}", e);
            return;
        }
        _ => (),
    }

    // Hash password:
    let password = args[4].as_bytes();
    let mut rng = thread_rng();
    let salt = SaltString::generate(&mut rng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password_simple(password, salt.as_ref())
        .expect("Could not hash password.")
        .to_string();

    // Get new random user id:
    let mut user_id = [0u8; 8];
    rng.fill_bytes(&mut user_id);
    while userid_name_tree
        .contains_key(user_id)
        .expect("Could not access username-tree.")
    {
        rng.fill_bytes(&mut user_id);
    }

    // Create new root dir:
    // Get new dir_id/fs_id:
    let mut dir_id = rng.next_u64();
    while dir_id == 0
        || dir_tree
            .contains_key(dir_id.to_be_bytes())
            .expect("Could not access directory tree.")
        || file_tree
            .contains_key(dir_id.to_be_bytes())
            .expect("Could not access file tree.")
    {
        dir_id = rng.next_u64();
    }
    // Append parent id (0):
    let mut dir_bytes = Vec::from(0u64.to_be_bytes());
    // Append owner id:
    dir_bytes.extend_from_slice(&user_id);
    // Append number of childs:
    dir_bytes.extend_from_slice(&0u16.to_be_bytes());
    // Append name:
    dir_bytes.extend_from_slice(b"home");

    // Insert values into trees in a single transaction:
    (&username_id_tree, &userid_name_tree, &userid_pwd_tree, &userid_rootdir_tree, &dir_tree).transaction(|(id_tt, name_tt, pwd_tt, root_tt, dir_tt)| -> ConflictableTransactionResult<(), UnabortableTransactionError> {
        id_tt.insert(username, &user_id)?;
        name_tt.insert(&user_id, username)?;
        pwd_tt.insert(&user_id, password_hash.as_bytes())?;
        root_tt.insert(&user_id, &dir_id.to_be_bytes())?;
        dir_tt.insert(&dir_id.to_be_bytes(), dir_bytes.as_slice())?;
        Ok(())
    }).expect("Could not apply transaction.");
}

pub fn userlist(args: Vec<String>) {
    if args.len() < 3 {
        println!("Usage: bonbon userlist <db-location>");
        return;
    }

    let sled_db = match open_db(args[2].as_str()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let userid_name_tree = sled_db
        .open_tree(b"userids_names")
        .expect("Could not open username tree.");

    for res in &userid_name_tree {
        match res {
            Ok((id_bytes, name_bytes)) => {
                let user_id = u64::from_be_bytes(id_bytes.as_ref().try_into().unwrap());
                let username = String::from_utf8(Vec::from(name_bytes.as_ref())).unwrap();

                println!("{}: \t{}", user_id, username);
            }
            Err(e) => {
                println!("Error while reading from DB:\n{}", e);
                return;
            }
        }
    }
}

fn open_db(location: &str) -> Result<Db, &'static str> {
    let db_location = PathBuf::from(location);
    if !db_location.is_dir() {
        return Err("The given db-location is not a directory.");
    }

    Ok(sled::open(db_location.as_path()).expect("Could not open database."))
}
