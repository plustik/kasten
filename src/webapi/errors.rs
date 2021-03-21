use rocket::{
    catchers,
    http::Status,
    response::{content::Html, status},
    Catcher,
};
use rocket_contrib::templates::Template;
use tera::Context;

pub fn error_catchers() -> Vec<Catcher> {
    catchers![internal_server_error]
}

#[catch(500)]
fn internal_server_error() -> status::Custom<Html<Template>> {
    let mut context = Context::new();
    context.insert("STATUS_CODE", "500");
    context.insert("STATUS_MSG", "Internal Server Error");

    status::Custom(
        Status::InternalServerError,
        Html(Template::render("error", context.into_json())),
    )
}
