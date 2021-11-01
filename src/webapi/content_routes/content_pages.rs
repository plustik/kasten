use rocket::response::content::Html;
use rocket_dyn_templates::tera::Context;
use rocket_dyn_templates::Template;
use serde::Serialize;

use crate::{
    controller,
    database::Database,
    models::{Dir, File, FsNode, User},
    Error,
};

#[derive(Debug, Serialize)]
struct DirContext {
    id: u64,
    name: String,
    may_read: bool,
    may_write: bool,
}
impl DirContext {
    fn from_dir(dir: &Dir, user: &User) -> Self {
        DirContext {
            id: dir.id,
            name: dir.name.clone(),
            may_read: dir.may_read(user),
            may_write: dir.may_write(user),
        }
    }
}

#[derive(Debug, Serialize)]
struct FileContext {
    id: u64,
    name: String,
    may_read: bool,
    may_write: bool,
    size: u64,
}
impl FileContext {
    fn from_file(file: &File, user: &User) -> Self {
        FileContext {
            id: file.id,
            name: file.name.clone(),
            may_read: file.may_read(user),
            may_write: file.may_write(user),
            size: 64, // TODO: Change to real value, when saving a files size is implemented.
        }
    }
}

pub fn dir_page(db: &Database, user_id: u64, dir_id: u64) -> Result<Html<Template>, Error> {
    let user = if let Some(u) = db.get_user(user_id)? {
        u
    } else {
        return Err(Error::NoSuchUser);
    };

    // Get dir as struct:
    let dir = controller::get_dir_info(dir_id, Some(user_id), db)?;

    // Create context:
    let mut cont = Context::new();
    cont.insert("USERNAME", &user.name);
    cont.insert("USERID", &user.id);

    // Insert owner:
    cont.insert("OWNERID", &dir.owner_id);
    cont.insert(
        "OWNERNAME",
        &controller::user::resolve_user_name(
            dir.owner_id,
            || controller::get_dir_info(dir_id, Some(user_id), db),
            db,
        )?,
    );

    // Create Vec of ancestors:
    let mut path_nodes = vec![DirContext::from_dir(&dir, &user)];
    let mut current_node = dir.clone();
    while current_node.parent_id != 0 {
        current_node = if let Some(d) = db.get_dir(current_node.parent_id)? {
            d
        } else {
            return Err(Error::NoSuchDir);
        };
        path_nodes.push(DirContext::from_dir(&current_node, &user));
    }
    cont.insert("PATH_NODES", &path_nodes);

    // Insert permission lists:
    let read_groups: Vec<_> = dir
        .read_group_ids
        .iter()
        .filter_map(|id| {
            controller::user::resolve_user_name(
                *id,
                || controller::get_dir_info(dir_id, Some(user_id), db),
                db,
            )
            .ok()
        })
        .collect();
    cont.insert("READABLE_GROUPS", &read_groups);
    let write_groups: Vec<_> = dir
        .read_group_ids
        .iter()
        .filter_map(|id| {
            controller::user::resolve_user_name(
                *id,
                || controller::get_dir_info(dir_id, Some(user_id), db),
                db,
            )
            .ok()
        })
        .collect();
    cont.insert("WRITEABLE_GROUPS", &write_groups);

    // Insert list of contained files:
    let files: Vec<FileContext> = db
        .get_files_by_parent(dir_id)?
        .into_iter()
        .map(|f| FileContext::from_file(&f, &user))
        .collect();
    cont.insert("FILES", &files);
    // Insert list of contained directories:
    let mut self_dir = dir.clone();
    self_dir.name = String::from(".");
    let dirs: Vec<DirContext> = std::iter::once(self_dir)
        .chain(
            controller::get_dir_info(dir.parent_id, Some(user_id), db)
                .into_iter()
                .map(|mut d| {
                    d.name = String::from("..");
                    d
                }),
        )
        .chain(db.get_dirs_by_parent(dir_id)?)
        .map(|d| DirContext::from_dir(&d, &user))
        .collect();
    cont.insert("DIRS", &dirs);

    Ok(Html(Template::render("dirview", cont.into_json())))
}
