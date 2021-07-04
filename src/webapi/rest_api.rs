use rocket::{
    http::{ContentType, Status},
    Route, State,
};

use crate::{
    database::Database,
    models::{Id, UserSession},
};

pub fn get_routes() -> Vec<Route> {
    routes![get_file_info,]
}

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
