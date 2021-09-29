use rocket::{fs::TempFile, http::Status, serde::json::Json, Route, State};

use super::super::FileMsg;
use crate::{
    config::Config,
    controller,
    database::Database,
    models::{File, Id, UserSession},
    Error,
};

pub fn get_routes() -> Vec<Route> {
    routes![add_file, upload_file, get_file_info, update_file_infos]
}

/*
 * If the requests cookies correspond to a valid User (building a UserSession succeeds) who does
 * not have the necessary rights for this action, an appropriate HTTP error Status code is
 * returned.
 * Otherwise the metadata of the file given by <file_id> (JSON representation of a File) is
 * returned.
 */
#[get("/files/<file_id>")]
async fn get_file_info(
    file_id: Id,
    session: Option<UserSession>,
    db: &State<Database>,
) -> Result<Json<File>, Status> {
    let file_id = file_id.inner();

    // TODO: Handle public files (session == None)
    if session.is_none() {
        // TODO: Logging
        println!("Error on GET /rest_api/files/...: No user session.");
        return Err(Status::Unauthorized);
    }

    match controller::get_file_info(file_id, session.unwrap().user_id, db) {
        Ok(file) => Ok(Json(file)),
        Err(Error::NoSuchFile) => {
            // TODO: Logging
            println!("Error on GET /rest_api/files/...: User requested nonexisting dir.");
            Err(Status::NotFound)
        }
        Err(err) => {
            // TODO: Logging
            println!("Error on GET /rest_api/files/...: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 * Adds a file (File) to the database.
 * The new File receives a new unique id. Other fields like name and parent_id should be given by
 * the requests body, which should follow the format of a File struct.
 * If the requests cookies correspond to a valid User (building a UserSession succeeds) who does
 * not have the necessary rights for this action (on the parent directory), an appropriate HTTP
 * error Status code is returned.
 * If a necessary field is missing, an appropriate HTTP error Status code is returned.
 * Otherwise the metadata of the new File (JSON representation of a File) is returned.
 */
#[post("/files", data = "<file_info>")]
async fn add_file(
    file_info: Json<FileMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<File>, Status> {
    let mut file_msg = file_info.into_inner();
    // Set the owner_id to the current user:
    file_msg.owner_id = Some(session.user_id);

    match controller::add_file(db, file_msg, session.user_id) {
        Ok(file) => Ok(Json(file)),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Trying to add a file without the necessary rights.");
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
 * Updates the metadata of a file given by <file_id> to the values given inside the requests body.
 * The body should follow the format of a File struct. Missing fields will be intantiated with
 * default values. Fields with default values will not be updated.
 * The given updates will be written to the database.
 */
#[put("/files/<file_id>", data = "<file_info>")]
async fn update_file_infos(
    file_id: Id,
    file_info: Json<FileMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<File>, Status> {
    let file_id = file_id.inner();
    let mut file_info = file_info.into_inner();

    // Make sure there aren't two different ids:
    if let Some(id) = file_info.id {
        if id != file_id {
            // TODO: Logging
            println!("Two different file_ids.");
            return Err(Status::BadRequest);
        }
    } else {
        file_info.id = Some(file_id);
    }

    // Performe update:
    match controller::update_file_infos(file_info, session.user_id, db) {
        Ok(file) => Ok(Json(file)),
        Err(Error::NoSuchFile) => {
            // TODO: Logging
            println!("Trying to update a nonexisting file.");
            Err(Status::NotFound)
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update a file which he doesn't own.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        }
        Err(err) => {
            // TODO: Logging
            println!("Error when updating file: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 * Set the content of a file given by <file_id> to the content of the reqest body (given by
 * file_content).
 * Fails with an appropriate HTTP Status, if the cookies of the request correspond to a User
 * (building a UserSession succeeds) which does not have the necessary rights for this action.
 */
#[put("/files/<file_id>/data", data = "<file_content>")]
async fn upload_file(
    file_id: Id,
    file_content: TempFile<'_>,
    session: UserSession,
    db: &State<Database>,
    config: &State<Config>,
) -> Result<Json<File>, Status> {
    match controller::update_file_content(
        file_id.inner(),
        session.user_id,
        db,
        config,
        file_content,
    )
    .await
    {
        Ok(file) => Ok(Json(file)),
        Err(Error::NoSuchFile) => {
            // TODO: Logging
            println!("User tried to update content of nonexisting file.");
            Err(Status::NotFound)
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("User tried to update content of a file he doesn't own.");
            Err(Status::Forbidden) // Maybe NotFound would be more secure
        }
        Err(err) => {
            // TODO: Logging
            println!("Error when updating file content: {}", err);
            Err(Status::InternalServerError)
        }
    }
}
