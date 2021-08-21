use rocket::fs::TempFile;

use crate::{
    config::Config,
    database::Database,
    models::{Dir, DirBuilder, File, FileBuilder, FsNode},
    webapi::{DirMsg, FileMsg},
    Error,
};

/**
 * Adds a directory (`Dir`) to the database.
 *
 * The new directory receives a new unique id and has no childs. Other fields like `name` and
 * `parent_id` should be given by the argument `dir_infos`.
 * If the id given by `user_id` does not correspond to a user who has the necessary rights (for the
 * parent directory), an `Err` is returned.
 * If a necessary field is missing, an `Err` is returned.
 * Otherwise the new directory is returned.
 */
pub fn add_dir(db: &Database, dir_infos: DirMsg, user_id: u64) -> Result<Dir, Error> {
    // Make sure the user has the necessary rights:
    if !dir_infos
        .parent_id
        .ok_or(Error::BadCall)
        .map(|p_id| db.get_dir(p_id)?.ok_or(Error::NoSuchDir))??
        .may_write(user_id)
    {
        return Err(Error::MissingAuthorization);
    }

    let mut dir_builder = DirBuilder::new()
        .with_id(0)
        .with_parent_id(dir_infos.parent_id.unwrap())
        .with_owner_id(user_id);
    if let Some(n) = dir_infos.name {
        dir_builder.set_name(n);
    }
    let mut new_dir = dir_builder.build();

    db.insert_new_dir(&mut new_dir)?;

    Ok(new_dir)
}

/**
 * If the requests cookies correspond to a valid User (building a UserSession succeeds) who does
 * not have the necessary rights for this action, an appropriate HTTP error Status code is
 * returned.
 * Otherwise the metadata of the directory given by <dir_id> (JSON representation of a Dir) is
 * returned.
 */
pub fn get_dir_info(dir_id: u64, user_id: Option<u64>, db: &Database) -> Result<Dir, Error> {
    // Check, if the user is allowed to access the directory:
    let dir = db.get_dir(dir_id)?.ok_or(Error::NoSuchDir)?;

    if user_id.is_some() && dir.may_read(user_id.unwrap()) {
        Ok(dir)
    } else {
        Err(Error::MissingAuthorization)
    }
}

/**
 * Updates the metadata of a directory given by field `id` of the given `DirMsg` to the values
 * given by the not `None` fields of the same struct.
 * This function will ignore the child_ids field of Dir and therefore it will not remove or add any
 * childs from a directory, even if the child_ids field in the request body does not contain all or
 * none of the directory's childs.
 * The given updates will be written to the database.
 */
pub fn update_dir_infos(dir_info: DirMsg, user_id: u64, db: &Database) -> Result<Dir, Error> {
    // Get old Dir from DB:
    let mut dir = db
        .get_dir(dir_info.id.ok_or(Error::BadCall)?)?
        .ok_or_else(|| {
            // TODO: Logging
            println!("Trying to update a nonexisting directory.");
            // TODO: Throw different errors for missing dir or missing parent dir.
            Error::NoSuchDir
        })?;

    // Make sure the user has the necessary rights:
    if !dir.may_write(user_id) {
        // TODO: Logging
        println!("User tried to update a directory which he doesn't own.");
        return Err(Error::MissingAuthorization);
    }

    // Set changed fields:
    dir_info.apply_changes(&mut dir);

    // Write updated dir to DB:
    db.update_dir(&dir)?;

    Ok(dir)
}

/**
 * Adds a file to the database.
 * The new File receives a new unique id. Other fields like name and parent_id should be given by
 * the argument `file_info`.
 * If the id given by `user_id` does not correspond to a User who has the necessary rights for this
 * action (on the parent directory), an Err is retuned.
 * If a necessary field is missing, an Err is returned.
 * Otherwise the new File is returned.
 */
pub fn add_file(db: &Database, file_info: FileMsg, user_id: u64) -> Result<File, Error> {
    // Make sure the user has the necessary rights:
    if !file_info
        .parent_id
        .ok_or(Error::BadCall)
        .map(|p_id| db.get_dir(p_id)?.ok_or(Error::NoSuchDir))??
        .may_write(user_id)
    {
        return Err(Error::MissingAuthorization);
    }

    let mut file_builder = FileBuilder::new()
        .with_parent_id(file_info.parent_id.unwrap())
        .with_owner_id(user_id);
    if let Some(n) = file_info.name {
        file_builder.set_name(n);
    }
    let mut new_file = file_builder.build();

    // Add new file:
    db.insert_new_file(&mut new_file)?;

    Ok(new_file)
}

/**
 * If the user given by `user_id` has the necessary rights to view the file given by `file_id` this
 * File is returned. Otherwise an Error is retuned.
 */
pub fn get_file_info(file_id: u64, user_id: u64, db: &Database) -> Result<File, Error> {
    // Get file from db:
    let file = db.get_file(file_id)?.ok_or(Error::NoSuchFile)?;

    // Check, if the user is allowed to access the file:
    if !file.may_read(user_id) {
        // TODO: Match against existing rules
        Err(Error::MissingAuthorization)
    } else {
        Ok(file)
    }
}

/**
 * Updates the metadata of a file given by field `id` of the given `FileMsg` to the values
 * given by the not `None` fields of the same struct.
 * The given updates will be written to the database.
 */
pub fn update_file_infos(file_info: FileMsg, user_id: u64, db: &Database) -> Result<File, Error> {
    // Get old File from DB:
    let mut file = db
        .get_file(file_info.id.ok_or(Error::BadCall)?)?
        .ok_or_else(|| {
            // TODO: Logging
            println!("Trying to update a nonexisting file.");
            Error::NoSuchFile
        })?;

    // Make sure the user has the necessary rights:
    if !file.may_write(user_id) {
        // TODO: Logging
        println!("User tried to update a file which he doesn't own.");
        return Err(Error::MissingAuthorization);
    }

    // Set changed fields:
    file_info.apply_changes(&mut file);

    // Write updated dir to DB:
    db.update_file(&file)?;

    Ok(file)
}

/**
 * Updates the content of a file given by `file_id` to the content of the given `TempFile`
 * `new_content`.
 *
 * The function checks whether the user given by `user_id` has the necessary rights to update the
 * file and returns an Error if not.
 */
pub async fn update_file_content(
    file_id: u64,
    user_id: u64,
    db: &Database,
    config: &Config,
    mut new_content: TempFile<'_>,
) -> Result<File, Error> {
    let file = db.get_file(file_id)?.ok_or(Error::NoSuchFile)?;

    // Check users permissions:
    if file.may_write(user_id) {
        return Err(Error::MissingAuthorization);
    }

    // Move temporary file to permanent path:
    let mut new_path = config.file_location.clone();
    new_path.push(format!("{:x}", file.id));
    new_content.persist_to(new_path).await?;

    // Send file information as respose:
    Ok(file)
}
