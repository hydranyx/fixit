#![feature(proc_macro_hygiene, decl_macro)]

use rocket::response::Redirect;
use rocket::{catch, catchers, get, routes, uri, Request};
use rocket_contrib::templates::Template;
use serde_derive::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    items: Vec<&'static str>,
}

#[get("/")]
fn index() -> Redirect {
    Redirect::to(uri!(get: name = "Unknown"))
}

#[get("/hello/<name>")]
fn get(name: String) -> Template {
    let context = TemplateContext {
        name,
        items: vec!["One", "Two", "Three"],
    };
    Template::render("index", &context)
}

#[catch(404)] // Replace the default 404 with the definition below.
fn not_found(req: &Request) -> Template {
    let mut map = HashMap::new();
    map.insert("path", req.uri().path());
    Template::render("error/404", &map)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, get]) // Attach the routes specified above.
        .attach(Template::fairing()) // Attach the fairing that automagically reads the templates.
        .register(catchers![not_found]) // Attach the catchers to fire when a particular error is thrown
}

fn main() {
    let path = "assets/knowledge_base/plumbing_knowledge_base.lms";
    let _inference_engine = inference_engine::prepare().with_knowledge_base_file(path);
    rocket().launch();
}
