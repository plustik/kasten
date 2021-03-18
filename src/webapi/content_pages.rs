
use rocket::response::content::Html;
use rocket_contrib::templates::Template;
use tera::Context;

use crate::Error;
use crate::database::Database;


pub fn dir_page(db: &Database, user_id: u64, dir_id: u64) -> Result<Html<Template>, Error> {
    let user = if let Some (u) = db.get_user(user_id)? {
        u
    } else {
        return Err(Error::NoSuchUser);
    };

    let mut cont = Context::new();
    cont.insert("USERNAME", &user.name);

    let files = db.get_files_by_parent(dir_id)?;
    let dirs = db.get_dirs_by_parent(dir_id)?;
    let names: Vec<String> = files
        .into_iter()
        .map(|file| file.name)
        .chain(dirs.into_iter().map(|dir| dir.name))
        .collect();
    cont.insert("FILES", &names);

    Ok(Html(Template::render("dirview", cont.into_json())))
}
