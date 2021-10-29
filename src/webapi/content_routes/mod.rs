use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use chrono::{offset::Utc, Duration};
use rocket::{
    form::{Form, FromForm},
    fs::TempFile,
    http::{Cookie, CookieJar, SameSite, Status},
    response::content::Html,
    serde::json::Json,
    Route, State,
};
use rocket_dyn_templates::{tera::Context, Template};

use crate::{
    config::Config,
    database::Database,
    models::{DirBuilder, FileBuilder, Id, UserSession},
    webapi::{DirMsg, FileMsg},
    Error,
};

mod content_pages;

pub fn get_routes() -> Vec<Route> {
    routes![
        index_login,
        get_login,
        index,
        login,
        logout,
        logout_no_session,
        dir_view,
        mkdir,
        upload_file,
        download_file,
        remove_dir,
        remove_file,
    ]
}

// Show login form:
#[get("/", rank = 3)]
fn index_login() -> Html<Template> {
    let context = Context::new();
    Html(Template::render("login", context.into_json()))
}
#[get("/login.html", rank = 3)]
fn get_login() -> Html<Template> {
    let context = Context::new();
    Html(Template::render("login", context.into_json()))
}

#[derive(FromForm)]
struct LoginCreds {
    username: String,
    password: String,
}

#[post("/login.html", data = "<credentials>")]
fn login(
    credentials: Form<LoginCreds>,
    cookies: &CookieJar<'_>,
    db: &State<Database>,
) -> Result<Html<Template>, Status> {
    // Try to get the user id:
    let user_id = match db.get_userid_by_name(&credentials.username) {
        Ok(Some(id)) => id,
        Ok(None) => {
            // Answer 'username does not exist':
            let mut context = Context::new();
            context.insert("WARNING", &"The given username does not exist.");
            return Ok(Html(Template::render("login", context.into_json())));
        }
        Err(_) => {
            // Send server error:
            return Err(Status::InternalServerError);
        }
    };

    // Try to get the user:
    let user = if let Ok(Some(u)) = db.get_user(user_id) {
        u
    } else {
        // Send server error:
        return Err(Status::InternalServerError);
    };

    // Verify password:
    let parsed_hash = PasswordHash::new(&user.pwd_hash).unwrap();
    let hasher = Argon2::default();
    if hasher
        .verify_password(credentials.password.as_bytes(), &parsed_hash)
        .is_ok()
    {
        // Right password.
        // Remove all existing expired sessions:
        if let Err(e) = db.filter_user_sessions(user.id, |session| {
            Utc::now().signed_duration_since(session.creation_date) < Duration::hours(24)
        }) {
            // TODO: Add logging
            //error!("DB-Error while GET /: {}", e);
            println!("DB-Error while GET /: {}", e);
        }

        // Create session:
        let session = match db.create_user_session(user.id) {
            Ok(s) => s,
            Err(e) => {
                // TODO: Add logging
                //error!("DB-Error while GET /: {}", e);
                println!("DB-Error while GET /: {}", e);

                return Err(Status::InternalServerError);
            }
        };
        // Set cookies:
        cookies.add(
            Cookie::build("session_id", format!("{:x}", session.session_id))
                .same_site(SameSite::Strict)
                .secure(true)
                .finish(),
        );
        // Send response:
        content_pages::dir_page(db, user.id, user.root_dir_id).map_err(|err| {
            if let Error::DbError(e) = err {
                // TODO: Add logging
                //error!("DB-Error while GET /: {}", e);
                println!("DB-Error while GET /: {}", e);

                Status::InternalServerError
            } else {
                panic!("Error: {}", err);
            }
        })
    } else {
        // Wrong password:
        let mut context = Context::new();
        context.insert("WARNING", &"The password was wrong.");
        Ok(Html(Template::render("login", context.into_json())))
    }
}

#[get("/logout.html", rank = 2)]
fn logout(
    session: UserSession,
    cookies: &CookieJar<'_>,
    db: &State<Database>,
) -> Result<Html<Template>, Status> {
    // Try to remove the user session from the DB:
    if let Err(e) = db.remove_user_session(session.session_id) {
        // TODO: Logging
        println!("Error on POST /logout.html: {}", e);
        return Err(Status::InternalServerError);
    }

    // Remove the cookie:
    cookies.remove(Cookie::named("session_id"));

    // Send login page:
    let context = Context::new();
    Ok(Html(Template::render("login", context.into_json())))
}

#[get("/logout.html", rank = 3)]
fn logout_no_session() -> Result<Html<Template>, Status> {
    // Send login page:
    let context = Context::new();
    Ok(Html(Template::render("login", context.into_json())))
}

// Show own and shared directories:
#[get("/", rank = 2)]
fn index(db: &State<Database>, session: UserSession) -> Result<Html<Template>, Status> {
    let user = match db.get_user(session.user_id) {
        Ok(opt) => opt.unwrap(),
        Err(e) => {
            // TODO: Add logging
            //error!("DB-Error while GET /: {}", e);
            println!("DB-Error while GET /: {}", e);

            return Err(Status::InternalServerError);
        }
    };

    content_pages::dir_page(db, user.id, user.root_dir_id).map_err(|err| {
        if let Error::DbError(e) = err {
            // TODO: Add logging
            //error!("DB-Error while GET /: {}", e);
            println!("DB-Error while GET /: {}", e);

            Status::InternalServerError
        } else {
            panic!("Error: {}", err);
        }
    })
}

// Shows the contents of the given directory.
#[get("/dirs/<dir_id>/view.html")]
fn dir_view(
    dir_id: Id,
    session: UserSession,
    db: &State<Database>,
) -> Result<Html<Template>, Status> {
    // Check if user is allowed to see that directory:
    let dir = match db.get_dir(dir_id.inner()) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err(Status::NotFound);
        }
        Err(e) => {
            // TODO: Logging
            println!("Error on GET /files/...: {}", e);
            return Err(Status::InternalServerError);
        }
    };
    if dir.owner_id != session.user_id {
        // TODO: Match against existing rules
        return Err(Status::Unauthorized);
    }

    // Responde with dirview page:
    content_pages::dir_page(db, session.user_id, dir_id.inner()).map_err(|err| {
        match err {
            Error::DbError(e) => {
                // TODO: Add logging
                //error!("DB-Error while GET /: {}", e);
                println!("DB-Error while GET /: {}", e);

                Status::InternalServerError
            }
            Error::NoSuchDir => Status::NotFound,
            err => {
                panic!("Error: {}", err);
            }
        }
    })
}

#[post("/mkdir/<parent_id>/<dir_name>")]
fn mkdir(
    parent_id: Id,
    dir_name: &str,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<DirMsg>, Status> {
    // Insert new dir to DB:
    let mut new_dir = DirBuilder::new()
        .with_parent_id(parent_id.inner())
        .with_owner_id(session.user_id)
        .with_name(dir_name)
        .build();
    // TODO: Check the users rights
    if db.insert_new_dir(&mut new_dir).is_err() {
        return Err(Status::InternalServerError);
    };

    Ok(Json(DirMsg::from(new_dir)))
}

#[post("/upload/<parent_id>/<upload_name>", data = "<tmp_file>")]
async fn upload_file(
    parent_id: Id,
    upload_name: &str,
    session: UserSession,
    db: &State<Database>,
    config: &State<Config>,
    mut tmp_file: TempFile<'_>,
) -> Result<Json<FileMsg>, Status> {
    // Insert new file to DB:
    let mut new_file = FileBuilder::new()
        .with_parent_id(parent_id.inner())
        .with_owner_id(session.user_id)
        .with_name(upload_name)
        .build();
    // TODO: Check the users rights
    if db.insert_new_file(&mut new_file).is_err() {
        return Err(Status::InternalServerError);
    };

    // Copy temporary file to new location:
    let mut new_path = config.file_location.clone();
    new_path.push(format!("{:x}", new_file.id));
    if tmp_file.persist_to(new_path).await.is_err() {
        // TODO: Logging and remove from DB
        return Err(Status::InternalServerError);
    }

    Ok(Json(FileMsg::from(new_file)))
}

#[get("/files/<file_id>")]
fn download_file(
    file_id: Id,
    session: UserSession,
    db: &State<Database>,
    config: &State<Config>,
) -> Result<std::fs::File, Status> {
    let file_id = file_id.inner();

    // Check, if the user is allowed to access the file:
    let file = match db.get_file(file_id) {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err(Status::NotFound);
        }
        Err(e) => {
            // TODO: Logging
            println!("Error on GET /files/...: {}", e);
            return Err(Status::InternalServerError);
        }
    };
    if file.owner_id != session.user_id {
        // TODO: Match against existing rules
        return Err(Status::Unauthorized);
    }

    // Respond with streamed file:
    let mut file_path = config.file_location.clone();
    file_path.push(format!("{:x}", file_id));
    match std::fs::File::open(file_path) {
        Ok(file) => Ok(file),
        Err(e) => {
            // TODO: Logging
            println!("Error on GET /files/...: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[delete("/dirs/<dir_id>")]
fn remove_dir(
    dir_id: Id,
    session: UserSession,
    db: &State<Database>,
) -> Result<Json<DirMsg>, Status> {
    // Check, if the user is allowed to access the directory:
    let dir = match db.get_dir(dir_id.inner()) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err(Status::NotFound);
        }
        Err(e) => {
            // TODO: Logging
            println!("Error on GET /files/...: {}", e);
            return Err(Status::InternalServerError);
        }
    };
    if dir.owner_id != session.user_id {
        // TODO: Match against existing rules
        return Err(Status::Unauthorized);
    }

    // Remove directory:
    match db.remove_dir(dir_id.inner()) {
        Ok(dir) => {
            // Send directory as response:
            Ok(Json(DirMsg::from(dir)))
        }
        Err(Error::NoSuchDir) => Err(Status::NotFound),
        Err(e) => {
            // TODO: Logging
            println!("Error on GET /files/...: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[delete("/files/<file_id>")]
fn remove_file(
    file_id: Id,
    session: UserSession,
    db: &State<Database>,
    config: &State<Config>,
) -> Result<Json<FileMsg>, Status> {
    // Check, if the user is allowed to access the file:
    let file = match db.get_file(file_id.inner()) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err(Status::NotFound);
        }
        Err(e) => {
            // TODO: Logging
            println!("Error on DELETE /files/<file_id>: {}", e);
            return Err(Status::InternalServerError);
        }
    };
    if file.owner_id != session.user_id {
        // TODO: Match against existing rules
        return Err(Status::Unauthorized);
    }

    // Remove file from DB:
    let res = match db.remove_file(file_id.inner()) {
        Ok(file) => file,
        Err(Error::NoSuchDir) => {
            return Err(Status::NotFound);
        }
        Err(e) => {
            // TODO: Logging
            println!("Error on DELETE /files/<file_id>: {}", e);
            return Err(Status::InternalServerError);
        }
    };

    // Remove file from FS:
    let mut file_path = config.file_location.clone();
    file_path.push(format!("{:x}", file_id.inner()));
    if let Err(e) = std::fs::remove_file(file_path) {
        // TODO: Logging
        println!("Error on DELETE /files/<file_id>: {}", e);
        return Err(Status::InternalServerError);
    }

    Ok(Json(FileMsg::from(res)))
}
