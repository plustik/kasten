use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use rocket::{
    http::Status,
    request::{Form, FromForm},
    response::content::Html,
    Rocket, State,
};
use rocket_contrib::templates::Template;
use tera::Context;

use crate::database::Database;
use crate::models::UserSession;

pub fn init(db: Database) -> Result<(), ()> {
    Rocket::ignite()
        .attach(Template::fairing())
        .manage(db)
        .mount("/", routes![index_login, index, login])
        .launch();

    Ok(())
}

// Show login form:
#[get("/", rank = 2)]
fn index_login() -> Html<Template> {
    let context = Context::new();
    Html(Template::render("login", context.into_json()))
}

#[derive(FromForm)]
struct LoginCreds {
    username: String,
    password: String,
}

#[post("/login.html", data = "<credentials>")]
fn login(credentials: Form<LoginCreds>, db: State<Database>) -> Result<Html<Template>, Status> {
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
        // Right password:
        let mut cont = Context::new();
        cont.insert("USERNAME", &user.name);
        cont.insert("FILES", &vec!["file1", "file2"]);
        Ok(Html(Template::render("index", cont.into_json())))
    } else {
        // Wrong password:
        let mut context = Context::new();
        context.insert("WARNING", &"The password was wrong.");
        Ok(Html(Template::render("login", context.into_json())))
    }
}

// Show own and shared directories:
#[get("/", rank = 3)]
fn index(db: State<Database>, session: UserSession) -> Html<Template> {
    let user = db
        .get_user(session.user_id)
        .expect("Could not read user from database.")
        .unwrap();

    let mut cont = Context::new();
    cont.insert("USERNAME", &user.name);

    let files = db.get_files_by_parent(user.root_dir_id).unwrap(); // TODO: How to handle errors?
    let dirs = db.get_dirs_by_parent(user.root_dir_id).unwrap(); // TODO: How to handle errors?
    let names: Vec<String> = files
        .into_iter()
        .map(|file| file.name)
        .chain(dirs.into_iter().map(|dir| dir.name))
        .collect();
    cont.insert("FILES", &names);

    Html(Template::render("index", cont.into_json()))
}
