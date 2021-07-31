use rocket::{
    fs::TempFile,
    http::Status,
    serde::json::Json,
    Route, State,
};

use crate::{
    config::Config,
    database::Database,
    models::{Dir, File, Id, UserSession},
};

pub fn get_routes() -> Vec<Route> {
    routes![
        get_dir_info,
        add_dir,
        update_dir_infos,
        get_file_info,
        add_file,
        upload_file,
        update_file_infos
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
) -> Result<Json<Dir>, Status> {
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

    Ok(Json(dir))
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
    dir_info: Json<Dir>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<Dir>, Status> {
    let mut new_dir = dir_info.into_inner();
    // Make sure, that the Dir does not have any childs:
    if !new_dir.child_ids.is_empty() {
        // TODO: Logging
        println!("Trying to add a directory with childs.");
        return Err(Status::BadRequest);
    }
    // Set the owner_id to the current user:
    new_dir.owner_id = session.user_id;
    // Make sure the user has the necessary rights on the parent directory:
    match db.get_dir(new_dir.parent_id) {
        Ok(Some(d)) => {
            if d.owner_id != session.user_id {
                // TODO: Logging
                println!("Trying to add a dir without the necessary rights.");
                return Err(Status::Forbidden); // Maybe Status::NotFound would be more secure?
            }
        }
        Ok(None) => {
            // TODO: Logging
            println!("Trying to add to a nonexisting dir.");
            return Err(Status::NotFound);
        }
        Err(_) => {
            // TODO: Logging
            println!("Could not get Dir from DB.");
            return Err(Status::InternalServerError);
        }
    };

    // Add Dir to DB:
    if let Err(_) = db.insert_new_dir(&mut new_dir) {
        // TODO: Logging
        println!("Could not insert new Dir into DB.");
        return Err(Status::InternalServerError);
    }

    Ok(Json(new_dir))
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
    dir_infos: Json<Dir>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<Dir>, Status> {
    let dir_id = dir_id.inner();

    // Make sure there aren't two different ids:
    let mut dir_info = dir_infos.into_inner();
    if dir_info.id == 0 {
        // 0 is the default
        dir_info.id = dir_id;
    } else if dir_info.id != dir_id {
        // TODO: Logging
        println!("Two different dir_ids.");
        return Err(Status::BadRequest);
    }

    // Get old Dir from DB:
    let mut dir = match db.get_dir(dir_info.id) {
        Ok(Some(d)) => d,
        Ok(None) => {
            // TODO: Logging
            println!("Trying to update a nonexisting directory.");
            return Err(Status::NotFound);
        }
        Err(_) => {
            // TODO: Logging
            println!("Could not get directory.");
            return Err(Status::NotFound);
        }
    };

    // Make sure the user has the necessary rights:
    if dir.owner_id != session.user_id {
        // TODO: Logging
        println!("User tried to update a directory which he doesn't own.");
        return Err(Status::Forbidden); // Maybe Status::NotFound would be more secure?
    }

    // Set changed fields:
    if dir_info.parent_id != 0 {
        // 0 is the default
        //dir.parent_id = dir_info.parent_id;
        // TODO: Make sure the parent exists and user has the necessary rights; Change new and old
        // parent.
        todo!();
    }
    if dir_info.owner_id != 0 {
        // 0 is the default
        dir.owner_id = dir_info.owner_id;
    }
    if dir_info.name != "[new_dir]" {
        // "[new_dir]" is the default
        dir.name = dir_info.name;
    }

    // Write updated dir to DB:
    if let Err(_) = db.update_dir(&dir) {
        // TODO: Logging
        println!("Could not update directory.");
        return Err(Status::NotFound); // Maybe Status::NotFound would be more secure?
    }

    Ok(Json(dir))
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
    Ok(Json(file))
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
) -> Result<Json<File>, Status> {
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
    Ok(Json(res_file))
}

/*
 * Updates the metadata of a file given by <file_id> to the values given inside the requests body.
 * The body should follow the format of a File struct. Missing fields will be intantiated with
 * default values. Fields with default values will not be updated.
 * The given updates will be written to the database.
 */
#[put("/files/<file_id>", data = "<file_infos>")]
async fn update_file_infos(
    file_id: Id,
    file_infos: Json<File>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<File>, Status> {
    let file_id = file_id.inner();

    let mut file_info = file_infos.into_inner();
    if file_info.id == 0 {
        // 0 is the default
        file_info.id = file_id;
    } else if file_info.id != file_id {
        // TODO: Logging
        println!("Two different file_ids.");
        return Err(Status::BadRequest);
    }

    let mut file = match db.get_file(file_info.id) {
        Ok(Some(f)) => f,
        Ok(None) => {
            // TODO: Logging
            println!("Trying to update a nonexisting file.");
            return Err(Status::NotFound);
        }
        Err(_) => {
            // TODO: Logging
            println!("Could not get file.");
            return Err(Status::NotFound);
        }
    };

    // Make sure the user has the necessary rights:
    if file.owner_id != session.user_id {
        // TODO: Logging
        println!("User tried to update a file which he doesn't own.");
        return Err(Status::Forbidden); // Maybe Status::NotFound would be more secure?
    }

    // Set changed fields:
    if file_info.parent_id != 0 {
        // 0 is the default
        //file.parent_id = file_info.parent_id;
        // TODO: Make sure the parent exists and user has the necessary rights; Change new and old
        // parent.
        todo!();
    }
    if file_info.owner_id != 0 {
        // 0 is the default
        file.owner_id = file_info.owner_id;
    }
    if file_info.name != "[new_file]" {
        // "[new_file]" is the default
        file.name = file_info.name;
    }

    // Write updated file to DB:
    if let Err(_) = db.update_file(&mut file) {
        // TODO: Logging
        println!("Could not update file.");
        return Err(Status::NotFound); // Maybe Status::NotFound would be more secure?
    }

    // Respond with new File:
    Ok(Json(file))
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
    mut file_content: TempFile<'_>,
    session: UserSession,
    db: &State<Database>,
    config: &State<Config>,
) -> Result<Json<File>, Status> {
    let file = match db.get_file(file_id.inner()) {
        Ok(Some(f)) => f,
        Ok(None) => {
            // TODO: Logging
            println!("User tried to change content of nonexisting file.");
            return Err(Status::NotFound);
        }
        Err(_) => {
            // TODO: Logging
            println!("Could not read file from DB.");
            return Err(Status::InternalServerError);
        }
    };

    // Check users permissions:
    if file.owner_id != session.user_id {
        // TODO: Logging
        println!("User tried to change content of file without necessary rights.");
        return Err(Status::Forbidden);
    }

    // Move temporary file to permanent path:
    let mut new_path = config.file_location.clone();
    new_path.push(format!("{:x}", file.id));
    if let Err(_) = file_content.persist_to(new_path).await {
        // TODO: Logging and remove from DB
        println!("Could not persist uploaded file.");
        return Err(Status::InternalServerError);
    }

    // Send file information as respose:
    Ok(Json(file))
}
