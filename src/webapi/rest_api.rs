use rocket::{
    http::{ContentType, Status},
    serde::json::Json,
    Route, State,
};

use crate::{
    database::Database,
    models::{File, Id, UserSession},
};

pub fn get_routes() -> Vec<Route> {
    routes![get_file_info, get_dir_info, add_file,]
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
) -> Result<(ContentType, String), Status> {
    let dir_id = dir_id.inner();

    // Check, if the user is allowed to access the directory:
    let dir = match db.get_dir(dir_id) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err(Status::NotFound);
        }
        Err(e) => {
            // TODO: Logging
            println!("Error on GET /rest_api/dir/...: {}", e);
            return Err(Status::InternalServerError);
        }
    };
    if session.is_some() && dir.owner_id != session.unwrap().user_id {
        // TODO: Match against existing rules
        return Err(Status::Unauthorized);
    }

    // Responde with directory as JSON:
    let res = match serde_json::to_string(&dir) {
        Ok(v) => v,
        Err(_) => {
            return Err(Status::InternalServerError);
        }
    };

    Ok((ContentType::JSON, res))
}

    dir_id: Id,


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
) -> Result<(ContentType, String), Status> {
    let file_id = file_id.inner();

    // Check, if the user is allowed to access the file:
    let file = match db.get_file(file_id) {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err(Status::NotFound);
        }
        Err(e) => {
            // TODO: Logging
            println!("Error on GET /rest_api/files/...: {}", e);
            return Err(Status::InternalServerError);
        }
    };
    if session.is_some() && file.owner_id != session.unwrap().user_id {
        // TODO: Match against existing rules
        return Err(Status::Unauthorized);
    }

    // Responde with file as JSON:
    let res = match serde_json::to_string(&file) {
        Ok(v) => v,
        Err(_) => {
            return Err(Status::InternalServerError);
        }
    };

    Ok((ContentType::JSON, res))
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
    file_info: Json<File>,
    session: UserSession,
    db: &State<Database>,
) -> Result<(ContentType, String), Status> {
    let new_file = file_info.into_inner();
    // Check, if the user has the necessary rights:
    let parent_dir = db
        .get_dir(new_file.parent_id)
        .map_err(|err| {
            // TODO: Logging
            println!("{}", err);
            Status::InternalServerError
        })?
        .ok_or(Status::InternalServerError)?;

    if parent_dir.owner_id != session.user_id {
        return Err(Status::Forbidden);
    }

    // Add new file:
    let res_file = db
        .insert_new_file(new_file.parent_id, new_file.owner_id, &new_file.name)
        .map_err(|err| {
            // TODO: Logging
            println!("{}", err);
            Status::InternalServerError
        })?;

    // Responde with new file as JSON:
    let res = match serde_json::to_string(&res_file) {
        Ok(v) => v,
        Err(_) => {
            return Err(Status::InternalServerError);
        }
    };

    Ok((ContentType::JSON, res))
}
