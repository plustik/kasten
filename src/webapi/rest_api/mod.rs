use rocket::Route;

mod dir_api;
mod file_api;
mod group_api;
mod user_api;

pub fn get_routes() -> Vec<Route> {
    let mut res = file_api::get_routes();
    res.extend(dir_api::get_routes());
    res.extend(user_api::get_routes());
    res.extend(group_api::get_routes());
    res
}
