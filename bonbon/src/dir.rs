use sled::Db;

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
                let name_start = 18 + (8 * u16::from_be_bytes(dir_bytes[16..18].try_into().unwrap()) as usize);
                let dirname = String::from_utf8(Vec::from(&dir_bytes[name_start..])).unwrap();

                println!("{:x}: \t{}", dir_id, dirname);
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
