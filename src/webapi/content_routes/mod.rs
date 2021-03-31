use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use chrono::{offset::Utc, Duration};
use rand::{thread_rng, RngCore};
use rocket::{
    data::{Data, FromDataSimple, Outcome},
    http::{ContentType, Cookie, Cookies, RawStr, SameSite, Status},
    request::{Form, FromForm, Request},
    response::{
        content::{Content, Html},
        Stream,
    },
    Route, State,
};
use rocket_contrib::templates::{tera::Context, Template};

use std::{
    fs::{copy, File},
    path::PathBuf,
};

use crate::{
    config::Config,
    database::Database,
    models::{Id, UserSession},
    Error,
};

mod content_pages;

pub fn get_routes() -> Vec<Route> {
    routes![
        index_login,
        index,
        login,
        logout,
        upload_file,
        download_file
    ]
}

// Show login form:
#[get("/", rank = 3)]
fn index_login() -> Html<Template> {
    let context = Context::new();
    Html(Template::render("login", context))
}

#[derive(FromForm)]
struct LoginCreds {
    username: String,
    password: String,
}

#[post("/login.html", data = "<credentials>")]
fn login(
    credentials: Form<LoginCreds>,
    mut cookies: Cookies,
    db: State<Database>,
) -> Result<Html<Template>, Status> {
    // Try to get the user id:
    let user_id = match db.get_userid_by_name(&credentials.username) {
        Ok(Some(id)) => id,
        Ok(None) => {
            // Answer 'username does not exist':
            let mut context = Context::new();
            context.insert("WARNING", &"The given username does not exist.");
            return Ok(Html(Template::render("login", context)));
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
        content_pages::dir_page(&db, user.id, user.root_dir_id).map_err(|err| {
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
        Ok(Html(Template::render("login", context)))
    }
}

#[get("/logout.html")]
fn logout(
    session: UserSession,
    mut cookies: Cookies,
    db: State<Database>,
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
    Ok(Html(Template::render("login", context)))
}

// Show own and shared directories:
#[get("/", rank = 2)]
fn index(db: State<Database>, session: UserSession) -> Result<Html<Template>, Status> {
    let user = match db.get_user(session.user_id) {
        Ok(opt) => opt.unwrap(),
        Err(e) => {
            // TODO: Add logging
            //error!("DB-Error while GET /: {}", e);
            println!("DB-Error while GET /: {}", e);

            return Err(Status::InternalServerError);
        }
    };

    content_pages::dir_page(&db, user.id, user.root_dir_id).map_err(|err| {
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

struct UploadFile {
    location: PathBuf,
}
impl FromDataSimple for UploadFile {
    type Error = Error;

    fn from_data(_: &Request<'_>, data: Data) -> Outcome<Self, Self::Error> {
        // Get path for temp-file:
        let mut temp_path = std::env::temp_dir();
        let mut rng = thread_rng();
        let random_part = rng.next_u64();
        temp_path.push(format!("kasten_upload_{:x}", random_part));

        if let Err(e) = data.stream_to_file(&temp_path) {
            return Outcome::Failure((Status::InternalServerError, Error::from(e)));
        }

        Outcome::Success(UploadFile {
            location: temp_path,
        })
    }
}

#[post("/upload/<parent_id>/<upload_name>", data = "<tmp_file>")]
fn upload_file(
    parent_id: Id,
    upload_name: &RawStr,
    session: UserSession,
    db: State<Database>,
    config: State<Config>,
    tmp_file: UploadFile,
) -> Result<Content<String>, Status> {
    // Insert new file to DB:
    let name = match upload_name.url_decode() {
        Ok(s) => s,
        Err(_) => {
            return Err(Status::BadRequest);
        }
    };
    let new_file = match db.insert_new_file(parent_id.inner(), session.user_id, name.as_str()) {
        Ok(v) => v,
        Err(_) => {
            return Err(Status::InternalServerError);
        }
    };

    // Copy temporary file to new location:
    let mut new_path = config.file_location.clone();
    new_path.push(format!("{:x}", new_file.id));
    if let Err(_) = copy(tmp_file.location, new_path) {
        // TODO: Logging and remove from DB
        return Err(Status::InternalServerError);
    }

    let res = match serde_json::to_string(&new_file) {
        Ok(v) => v,
        Err(_) => {
            // TODO: Logging and remove from DB
            return Err(Status::InternalServerError);
        }
    };

    Ok(Content(ContentType::JSON, res))
}

#[get("/files/<file_id>")]
fn download_file(
    file_id: Id,
    session: UserSession,
    db: State<Database>,
    config: State<Config>,
) -> Result<Stream<File>, Status> {
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
    if file.owner_id == session.user_id {
        // Open file:
        let mut file_path = config.file_location.clone();
        file_path.push(format!("{:x}", file_id));
        match File::open(file_path) {
            Ok(file) => Ok(Stream::chunked(file, 16384)),
            Err(e) => {
                // TODO: Logging
                println!("Error on GET /files/...: {}", e);
                Err(Status::InternalServerError)
            }
        }
    } else {
        // TODO: Match against existing rules
        Err(Status::Unauthorized)
    }
}