use rand::{thread_rng, RngCore};

use crate::{
    database::Database,
    models::{Dir, User},
    webapi::UserMsg,
    Error,
};

/**
 * If the user given by `querying_user_id` has the necessary rights to view the user given by
 * `queried_user_id`, the latter `User` is returned.
 * If the user given by 'queried_user_id' does not exist, `Error::NoSuchTarget` is returned. If the
 * user given by `querying_user_id` does not have the necessary rights, to view the user given by
 * `queried_user_id`, `Error::MissingAuthorization` is retuned.
 */
pub fn get_user_info(
    queried_user_id: u64,
    querying_user_id: u64,
    db: &Database,
) -> Result<User, Error> {
    // Check, if the querying user is allowed to view the queried user:
    let user = db.get_user(queried_user_id)?.ok_or(Error::NoSuchTarget)?;

    if querying_user_id == queried_user_id {
        Ok(user)
    } else {
        Err(Error::MissingAuthorization)
    }
}

/**
 * If the user given by `acting_user_id` has the rights necessary to add a new user, the user
 * with the attrbutes given by user_infos is added to the DB. If some necessary attrbutes are
 * missing, `Error::BadCall` is returned. If the given name allready exists in the DB,
 * `Error::TargetExists` is retuned.
 * If the user given by `acting_user_id` does not have the rights necessary to add a new user,
 * `Error::MissingAuthorization` is retuned.
 */
pub fn add_user(user_infos: UserMsg, acting_user_id: u64, db: &Database) -> Result<User, Error> {
    // Make sure the acting user has the necessary permissions:
    if acting_user_id == 0 { // TODO: Implement permissions
        // TODO: Implement permissions
        return Err(Error::MissingAuthorization);
    }

    // Make sure all necessary fields exists:
    if user_infos.name.is_none() {
        return Err(Error::BadCall);
    }
    if user_infos.password.is_none() {
        return Err(Error::BadCall);
    }
    // Make sure the id is not set:
    if user_infos.id.is_some() {
        return Err(Error::BadCall);
    }

    // Check if username allready exists:
    db.get_userid_by_name(user_infos.name.as_ref().unwrap())?
        .ok_or(Error::TargetExists)?;

    // Get new random user id:
    let mut rng = thread_rng();
    let mut user_id = rng.next_u64();
    while db.get_user(user_id)?.is_some() {
        user_id = rng.next_u64();
    }

    // Create new root dir:
    let mut root_dir = Dir {
        id: 0, // Will be updated by `insert_new_dir()`
        parent_id: 0,
        owner_id: user_id,
        child_ids: Vec::new(),
        name: String::from("home"),
    };
    db.insert_new_dir(&mut root_dir)?;

    let mut new_user = User {
        // Bad pattern; Instead create User from UserMsg directly
        id: user_id,
        name: String::from(""),
        pwd_hash: String::from(""),
        root_dir_id: root_dir.id,
    };
    user_infos.apply_changes(&mut new_user);

    db.insert_user(&new_user)?;

    Ok(new_user)
}

/**
 * If the user given by `acting_user_id` has the rights necessary to change the attributes of the
 * user given by `user_infos.id`, these attrbutes will be updated to the values given by
 * `user_infos` and the changes will be written to the DB.
 * If `user_infos.id` equals `None`, `Error::BadCall` is returned.
 * If there is no user with the given ID in the DB, `Error::NoSuchTarget` is retuned.
 * If the given name allready exists in the DB, `Error::TargetExists` is retuned.
 * If the user given by `acting_user_id` does not have the rights necessary to add a new user,
 * `Error::MissingAuthorization` is retuned.
 */
pub fn update_user_infos(
    user_infos: UserMsg,
    acting_user_id: u64,
    db: &Database,
) -> Result<User, Error> {
    // Make sure the acting user has the necessary permissions:
    if acting_user_id == 0 { // TODO: Implement permissions
        // TODO: Implement permissions
        return Err(Error::MissingAuthorization);
    }

    // Make sure the id is set:
    let user_id = if let Some(id) = user_infos.id {
        id
    } else {
        return Err(Error::BadCall);
    };

    // Check if username allready exists:
    db.get_userid_by_name(user_infos.name.as_ref().unwrap())?
        .ok_or(Error::TargetExists)?;

    let mut user = db.get_user(user_id)?.ok_or(Error::NoSuchTarget)?;
    user_infos.apply_changes(&mut user);

    db.insert_user(&user)?;

    Ok(user)
}
