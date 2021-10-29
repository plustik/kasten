use rocket::{http::Status, serde::json::Json, Route, State};

use super::super::{DirMsg, GroupMsg};
use crate::{
    controller,
    database::Database,
    models::{Id, UserSession},
    Error,
};

pub fn get_routes() -> Vec<Route> {
    routes![
        add_dir,
        get_dir_info,
        update_dir_infos,
        add_read_permission,
        add_write_permission
    ]
}

/*
 * If the requests cookies correspond to a valid User (building a UserSession succeeds) who does
 * not have the necessary rights for this action, an appropriate HTTP error Status code is
 * returned.
 * Otherwise the metadata of the directory given by <dir_id> (JSON representation of a Dir) is
 * returned.
 */
#[get("/dirs/<dir_id>")]
async fn get_dir_info(
    dir_id: Id,
    session: Option<UserSession>,
    db: &State<Database>,
) -> Result<Json<DirMsg>, Status> {
    let dir_id = dir_id.inner();

    match controller::get_dir_info(dir_id, session.as_ref().map(|s| s.user_id), db) {
        Ok(dir) => Ok(Json(DirMsg::from(dir))),
        Err(Error::NoSuchDir) => Err(Status::NotFound),
        Err(Error::MissingAuthorization) => {
            if session.is_some() {
                Err(Status::Forbidden)
            } else {
                Err(Status::Unauthorized)
            }
        }
        Err(_) => Err(Status::InternalServerError),
    }
}

/*
 * Adds a directory (Dir) to the database.
 * The new directory receives a new unique id and has no childs. Other fields like name and
 * parent_id should be given by the requests body, which should follow the format of a Dir struct.
 * If the requests cookies correspond to a valid User (building a UserSession succeeds) who does
 * not have the necessary rights for this action (on the parent directory), an appropriate HTTP
 * error Status code is returned.
 * If a necessary field is missing, an appropriate HTTP error Status code is returned.
 * Otherwise the metadata of the new directory (JSON representation of a Dir) is returned.
 */
#[post("/dirs", data = "<dir_info>")]
async fn add_dir(
    dir_info: Json<DirMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<DirMsg>, Status> {
    let mut dir_msg = dir_info.into_inner();
    // Set the owner_id to the current user:
    dir_msg.owner_id = Some(Id::from(session.user_id));

    match controller::add_dir(db, dir_msg, session.user_id) {
        Ok(dir) => Ok(Json(DirMsg::from(dir))),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Trying to add a dir without the necessary rights.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        }
        Err(Error::NoSuchDir) => {
            // TODO: Logging
            println!("Trying to add to a nonexisting dir.");
            Err(Status::NotFound)
        }
        Err(Error::BadCall) => {
            // TODO: Logging
            println!("Trying to add a dir without parent.");
            Err(Status::BadRequest) // Maybe Status::NotFound would be more secure?
        }
        Err(_) => {
            // TODO: Logging
            println!("Could not insert Dir to DB.");
            Err(Status::InternalServerError)
        }
    }
}

/*
 * Updates the metadata of a directory given by <dir_id> to the values given inside the requests
 * body.
 * The body should follow the format of a Dir struct. Missing fields will be intantiated with
 * default values. Fields with default values will not be updated.
 * This function will ignore the child_ids field of Dir and therefore it will not remove or add any
 * childs from a directory, even if the child_ids field in the request body does not contain all or
 * none of the directory's childs.
 * The given updates will be written to the database.
 */
#[put("/dirs/<dir_id>", data = "<dir_infos>")]
async fn update_dir_infos(
    dir_id: Id,
    dir_infos: Json<DirMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<DirMsg>, Status> {
    let mut dir_info = dir_infos.into_inner();

    // Make sure there aren't two different ids:
    if let Some(ref id) = dir_info.id {
        if id != &dir_id {
            // TODO: Logging
            println!("Two different dir_ids.");
            return Err(Status::BadRequest);
        }
    } else {
        dir_info.id = Some(dir_id);
    }

    // Performe update:
    match controller::update_dir_infos(dir_info, session.user_id, db) {
        Ok(dir) => Ok(Json(DirMsg::from(dir))),
        Err(Error::NoSuchDir) => {
            // TODO: Logging
            println!("Trying to update a nonexisting directory.");
            // TODO: Different status if the parent dir doesn't exist. (see TODO at
            // update_dir_infos)
            Err(Status::NotFound)
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update a directory which he doesn't own.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        }
        Err(err) => {
            // TODO: Logging
            println!("Error when updating directory: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 * Give read permissions for a given directory to a given group.
 *
 * Add the group given by `group_id` to the list of readable groups of the directory given by
 * `dir_id`.
 * Fails with an appropriate HTTP Status, if the cookies of the request correspond to a User
 * (building a UserSession succeeds) which does not have the necessary rights for this action.
 * Otherwise a JSON representation of the new directory is returned.
 */
#[post("/dirs/<dir_id>/permissions/read", data = "<group>")]
async fn add_read_permission(
    dir_id: Id,
    group: Json<GroupMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<DirMsg>, Status> {
    let dir_id = dir_id.inner();
    let group_id = group.into_inner().id.ok_or(Status::BadRequest)?.as_int();

    // Make sure the given dir exists:
    let mut dir = match controller::get_dir_info(dir_id, Some(session.user_id), db) {
        Ok(d) => d,
        Err(Error::NoSuchDir) => {
            // TODO: Logging
            println!("User tried to update permissions of nonexisting directory.");
            return Err(Status::NotFound);
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update permissions of directory, but wasn't allowed to.");
            return Err(Status::Forbidden); // Maybe Status::NotFound would be more secure?
        }
        Err(e) => {
            // TODO: Logging
            println!(
                "Unexpected error while user tried to update permissions of directory: {}",
                e
            );
            return Err(Status::InternalServerError);
        }
    };

    match controller::add_read_permission(dir_id, group_id, session.user_id, db) {
        Ok(()) => {
            dir.read_group_ids.push(group_id);
            Ok(Json(DirMsg::from(dir)))
        }
        Err(Error::NoSuchTarget) => {
            // TODO: Logging
            println!("User tried to update permissions of nonexisting directory.");
            Err(Status::NotFound)
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update permissions of directory, but wasn't allowed to.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        }
        Err(e) => {
            // TODO: Logging
            println!(
                "Unexpected error while user tried to update permissions of directory: {}",
                e
            );
            Err(Status::InternalServerError)
        }
    }
}
/*
 * Give write permissions for a given directory to a given group.
 *
 * Add the group given by `group_id` to the list of writeable groups of the directory given by
 * `dir_id`.
 * Fails with an appropriate HTTP Status, if the cookies of the request correspond to a User
 * (building a UserSession succeeds) which does not have the necessary rights for this action.
 * Otherwise a JSON representation of the new directory is returned.
 */
#[post("/dirs/<dir_id>/permissions/write", data = "<group>")]
async fn add_write_permission(
    dir_id: Id,
    group: Json<GroupMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<DirMsg>, Status> {
    let dir_id = dir_id.inner();
    let group_id = group.into_inner().id.ok_or(Status::BadRequest)?.as_int();

    // Make sure the given dir exists:
    let mut dir = match controller::get_dir_info(dir_id, Some(session.user_id), db) {
        Ok(d) => d,
        Err(Error::NoSuchDir) => {
            // TODO: Logging
            println!("User tried to update permissions of nonexisting directory.");
            return Err(Status::NotFound);
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update permissions of directory, but wasn't allowed to.");
            return Err(Status::Forbidden); // Maybe Status::NotFound would be more secure?
        }
        Err(e) => {
            // TODO: Logging
            println!(
                "Unexpected error while user tried to update permissions of directory: {}",
                e
            );
            return Err(Status::InternalServerError);
        }
    };

    match controller::add_write_permission(dir_id, group_id, session.user_id, db) {
        Ok(()) => {
            dir.write_group_ids.push(group_id);
            Ok(Json(DirMsg::from(dir)))
        }
        Err(Error::NoSuchTarget) => {
            // TODO: Logging
            println!("User tried to update permissions of nonexisting directory.");
            Err(Status::NotFound)
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update permissions of directory, but wasn't allowed to.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        }
        Err(e) => {
            // TODO: Logging
            println!(
                "Unexpected error while user tried to update permissions of directory: {}",
                e
            );
            Err(Status::InternalServerError)
        }
    }
}
