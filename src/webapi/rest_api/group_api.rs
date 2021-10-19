use rocket::{http::Status, serde::json::Json, Route, State};

use super::super::GroupMsg;
use crate::{
    controller,
    database::Database,
    models::{Group, Id, UserSession},
    Error,
};

pub fn get_routes() -> Vec<Route> {
    routes![add_group, get_group_info, update_group_infos]
}

/*
 * If the user of the given session has the necessary rights to view the information of the group
 * given by `group_id`, these information will be returned as JSON.
 */
#[get("/groups/<group_id>")]
async fn get_group_info(
    group_id: Id,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<Group>, Status> {
    let group_id = group_id.inner();

    match controller::group::get_group_info(group_id, session.user_id, db) {
        Ok(group) => Ok(Json(group)),
        Err(Error::NoSuchTarget) => {
            // TODO: Logging
            println!("Error on GET /rest_api/groups/...: Tried to get a nonexisting Group.");
            Err(Status::NotFound)
        }
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on GET /rest_api/groups/...: Missing rights to get Group.");
            Err(Status::Forbidden) // Maybe Status::NotFound would be more secure?
        }
        Err(err) => {
            // TODO: Logging
            println!("Error on GET /rest_api/groups/...: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 * If the user of the given session has the necessary rights to add another group, a new group with
 * the attributes given by the requests body will be added to the DB.
 * A Json representation of the new group including its new ID will be returned.
 */
#[post("/groups", data = "<group_info>")]
async fn add_group(
    group_info: Json<GroupMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<Group>, Status> {
    let group_msg = group_info.into_inner();

    match controller::group::add_group(group_msg, session.user_id, db) {
        Ok(group) => Ok(Json(group)),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on POST /rest_api/groups/: Missing rights to add group.");
            Err(Status::Forbidden)
        }
        Err(Error::TargetExists) => {
            // TODO: Logging
            println!("Error on POST /rest_api/groups: Name exists.");
            Err(Status::Forbidden) // TODO: Replace with correct HTTP status code
        }
        Err(err) => {
            // TODO: Logging
            println!("Error on POST /rest_api/groups: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

/*
 * If the user of the given session has the necessary rights to change a groups attributes, the
 * attributes of the group given by `group_id` will be changed to the values given by the requests
 * body.
 */
#[put("/groups/<group_id>", data = "<group_info>")]
async fn update_group_infos(
    group_id: Id,
    group_info: Json<GroupMsg>,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<Group>, Status> {
    let mut group_msg = group_info.into_inner();

    // Make sure there aren't two different group IDs:
    if group_msg.id.is_some() && group_msg.id.unwrap() == group_id.inner() {
        // TODO: Logging
        println!("Error on PUT /rest_api/groups/...: Two different group IDs.");
        return Err(Status::BadRequest);
    }
    // Set ID attribute of group_msg:
    if group_msg.id.is_none() {
        group_msg.id = Some(group_id.inner());
    }

    match controller::group::update_group_infos(group_msg, session.user_id, db) {
        Ok(group) => Ok(Json(group)),
        Err(Error::MissingAuthorization) => {
            // TODO: Logging
            println!("Error on PUT /rest_api/groups/...: Missing rights to update group.");
            Err(Status::Forbidden)
        }
        Err(Error::TargetExists) => {
            // TODO: Logging
            println!("Error on PUT /rest_api/groups/...: Name exists.");
            Err(Status::Conflict)
        }
        Err(Error::NoSuchTarget) => {
            // TODO: Logging
            println!("Error on PUT /rest_api/groups/...: No group with given ID.");
            Err(Status::NotFound)
        }
        Err(err) => {
            // TODO: Logging
            println!("Error on POST /rest_api/users: {}", err);
            Err(Status::InternalServerError)
        }
    }
}
