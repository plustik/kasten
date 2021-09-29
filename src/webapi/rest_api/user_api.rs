use rocket::{http::Status, serde::json::Json, Route, State};

use super::super::UserMsg;
use crate::{
    controller,
    database::Database,
    models::{Id, User, UserSession},
    Error,
};

pub fn get_routes() -> Vec<Route> {
    routes![add_user, get_user_info, update_user_infos]
}

/*
 * If the user of the given session has the necessary rights to view the information of the user
 * given by user_id, these information will be returned as JSON.
 */
#[get("/users/<user_id>")]
async fn get_user_info(
    user_id: Id,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<User>, Status> {
    let user_id = user_id.inner();

    match controller::user::get_user_info(user_id, session.user_id, db) {
        Ok(user) => Ok(Json(user)),
        Err(Error::NoSuchTarget) => {
            // TODO: Logging
            println!("Error on GET /rest_api/users/...: Tried to get a nonexisting User.");
            Err(Status::NotFound)
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on GET /rest_api/users/...: Missing rights to get User.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        }
        Err(err) => {
            // TODO: Logging
            println!("Error on GET /rest_api/users/...: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 * If the user of the given session has the necessary rights to add another user, a new user with
 * the attributes given by the requests body will be added to the DB.
 * A Json representation of the new user including its new ID will be returned.
 */
#[post("/users", data = "<user_info>")]
async fn add_user(
    user_info: Json<UserMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<User>, Status> {
    let user_msg = user_info.into_inner();

    match controller::user::add_user(user_msg, session.user_id, db) {
        Ok(user) => Ok(Json(user)),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users/: Missing rights to add User.");
            Err(Status::Forbidden)
        }
        Err(Error::TargetExists) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users: Name exists.");
            Err(Status::Forbidden) // TODO: Replace with correct HTTP status code
        }
        Err(err) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 * If the user of the given session has the necessary rights to change a users attributes, the
 * attributes of the user given by `user_id` will be changed to the values given by the requests
 * body.
 */
#[put("/users/<user_id>", data = "<user_info>")]
async fn update_user_infos(
    user_id: Id,
    user_info: Json<UserMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<User>, Status> {
    let mut user_msg = user_info.into_inner();

    // Make sure there aren't two different user IDs:
    if user_msg.id.is_some() && user_msg.id.unwrap() == user_id.inner() {
        // TODO: Logging
        println!("Error on PUT /rest_api/users/...: Two different user IDs.");
        return Err(Status::BadRequest);
    }
    // Set ID attribute of user_msg:
    if user_msg.id.is_none() {
        user_msg.id = Some(user_id.inner());
    }

    match controller::user::update_user_infos(user_msg, session.user_id, db) {
        Ok(user) => Ok(Json(user)),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on PUT /rest_api/users/...: Missing rights to update User.");
            Err(Status::Forbidden)
        }
        Err(Error::TargetExists) => {
            // TODO: Logging
            println!("Error on PUT /rest_api/users/...: Name exists.");
            Err(Status::Conflict)
        }
        Err(Error::NoSuchTarget) => {
            // TODO: Logging
            println!("Error on PUT /rest_api/users/...: No user with given ID.");
            Err(Status::NotFound) // TODO: Replace with correct HTTP status code
        }
        Err(err) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users: {}", err);
            Err(Status::InternalServerError)
        }
    }
}
