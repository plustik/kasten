use rocket::response::content::Html;
use rocket_contrib::templates::Template;
use tera::Context;

use crate::{database::Database, Error};

pub fn dir_page(db: &Database, user_id: u64, dir_id: u64) -> Result<Html<Template>, Error> {
    let user = if let Some(u) = db.get_user(user_id)? {
        u
    } else {
        return Err(Error::NoSuchUser);
    };

    // Get dir as struct:
    let dir = if let Some(d) = db.get_dir(dir_id)? {
        d
    } else {
        return Err(Error::NoSuchDir);
    };

    // Create Vec of ancestors:
    let mut path_nodes = Vec::new();
    path_nodes.push(dir);
    while path_nodes.last().unwrap().parent_id != 0 {
        let next_parent = if let Some(d) = db.get_dir(path_nodes.last().unwrap().parent_id)? {
            d
        } else {
            return Err(Error::NoSuchDir);
        };
        path_nodes.push(next_parent);
    }

    // Create context:
    let mut cont = Context::new();
    cont.insert("USERNAME", &user.name);
    cont.insert("PATH_NODES", &path_nodes);

    let files = db.get_files_by_parent(dir_id)?;
    cont.insert("FILES", &files);
    let dirs = db.get_dirs_by_parent(dir_id)?;
    cont.insert("DIRS", &dirs);

    Ok(Html(Template::render("dirview", cont.into_json())))
}
