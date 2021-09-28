use rocket::{http::Status, serde::json::Json, Route, State};

use crate::{
    controller,
    database::Database,
    Error,
    models::{Id, User, UserSession},
};
use super::super::UserMsg;

pub fn get_routes() -> Vec<Route> {
    routes![
        add_user,
        get_user_info,
        update_user_infos
    ]
}


/*
 * If the user of the given session has the necessary rights to view the information of the user
 * given by user_id, these information will be returned as JSON.
 */
#[get("/users/<user_id>")]
async fn get_user_info(
    user_id: Id,
    session: UserSession,
    db: &State<Database>
) -> Result<Json<User>, Status> {
    let user_id = user_id.inner();

    match controller::user::get_user_info(user_id, session.user_id, &db) {
        Ok(user) => Ok(Json(user)),
        Err(Error::NoSuchTarget) => {
            // TODO: Logging
            println!("Error on GET /rest_api/users/...: Tried to get a nonexisting User.");
            Err(Status::NotFound)
        },
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on GET /rest_api/users/...: Missing rights to get User.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        },
        Err(err) => {
            // TODO: Logging
            println!("Error on GET /rest_api/users/...: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 */
#[post("/users", data = "<user_info>")]
async fn add_user(
    user_info: Json<UserMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<User>, Status> {
    let user_msg = user_info.into_inner();

    match controller::user::add_user(user_msg, session.user_id, &db) {
        Ok(user) => Ok(Json(user)),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users/: Missing rights to add User.");
            Err(Status::Forbidden)
        },
        Err(Error::TargetExists) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users: Name exists.");
            Err(Status::Forbidden) // TODO: Replace with correct HTTP status code
        },
        Err(err) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 */
#[put("/files/<user_id>", data = "<user_info>")]
async fn update_user_infos(
    user_id: Id,
    user_info: Json<UserMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<User>, Status> {
    todo!()
}
