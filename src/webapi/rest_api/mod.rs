use rocket::Route;

mod file_api;
mod dir_api;
mod user_api;

pub fn get_routes() -> Vec<Route> {
    let mut res = file_api::get_routes();
    res.extend(dir_api::get_routes());
    res.extend(user_api::get_routes());
    res
}
