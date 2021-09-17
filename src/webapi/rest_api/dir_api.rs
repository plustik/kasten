use rocket::{http::Status, serde::json::Json, Route, State};

use super::super::DirMsg;
use crate::{
    controller,
    database::Database,
    models::{Dir, Id, UserSession},
    Error,
};

pub fn get_routes() -> Vec<Route> {
    routes![
        add_dir,
        get_dir_info,
        update_dir_infos,
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
pub async fn get_dir_info(
    dir_id: Id,
    session: Option<UserSession>,
    db: &State<Database>,
) -> Result<Json<Dir>, Status> {
    let dir_id = dir_id.inner();

    match controller::get_dir_info(dir_id, session.as_ref().map(|s| s.user_id), db) {
        Ok(dir) => Ok(Json(dir)),
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
pub async fn add_dir(
    dir_info: Json<DirMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<Dir>, Status> {
    let mut dir_msg = dir_info.into_inner();
    // Set the owner_id to the current user:
    dir_msg.owner_id = Some(session.user_id);

    match controller::add_dir(db, dir_msg, session.user_id) {
        Ok(dir) => Ok(Json(dir)),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Trying to add a dir without the necessary rights.");
            return Err(Status::Forbidden); // Maybe Status::NotFound would be more secure?
        }
        Err(Error::NoSuchDir) => {
            // TODO: Logging
            println!("Trying to add to a nonexisting dir.");
            return Err(Status::NotFound);
        }
        Err(Error::BadCall) => {
            // TODO: Logging
            println!("Trying to add a dir without parent.");
            return Err(Status::BadRequest); // Maybe Status::NotFound would be more secure?
        }
        Err(_) => {
            // TODO: Logging
            println!("Could not insert Dir to DB.");
            return Err(Status::InternalServerError);
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
pub async fn update_dir_infos(
    dir_id: Id,
    dir_infos: Json<DirMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<Dir>, Status> {
    let dir_id = dir_id.inner();
    let mut dir_info = dir_infos.into_inner();

    // Make sure there aren't two different ids:
    if let Some(id) = dir_info.id {
        if id != dir_id {
            // TODO: Logging
            println!("Two different dir_ids.");
            return Err(Status::BadRequest);
        }
    } else {
        dir_info.id = Some(dir_id);
    }

    // Performe update:
    match controller::update_dir_infos(dir_info, session.user_id, &db) {
        Ok(dir) => Ok(Json(dir)),
        Err(Error::NoSuchDir) => {
            // TODO: Logging
            println!("Trying to update a nonexisting directory.");
            // TODO: Different status if the parent dir doesn't exist. (see TODO at
            // update_dir_infos)
            return Err(Status::NotFound);
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update a directory which he doesn't own.");
            return Err(Status::Forbidden); // Maybe Status::NotFound would be more secure?
        }
        Err(err) => {
            // TODO: Logging
            println!("Error when updating directory: {}", err);
            return Err(Status::InternalServerError);
        }
    }
}
