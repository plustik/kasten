use rocket::response::content::Html;
use rocket_contrib::templates::Template;
use tera::Context;

use crate::{
    database::Database,
    Error
};

pub fn dir_page(db: &Database, user_id: u64, dir_id: u64) -> Result<Html<Template>, Error> {
    let user = if let Some(u) = db.get_user(user_id)? {
        u
    } else {
        return Err(Error::NoSuchUser);
    };

    let mut cont = Context::new();
    cont.insert("USERNAME", &user.name);
    cont.insert("DIR_ID", format!("{:x}", dir_id).as_str());

    let files = db.get_files_by_parent(dir_id)?;
    cont.insert("FILES", &files);
    let dirs = db.get_dirs_by_parent(dir_id)?;
    cont.insert("DIRS", &dirs);

    Ok(Html(Template::render("dirview", cont.into_json())))
}
