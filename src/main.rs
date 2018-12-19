#![feature(proc_macro_hygiene, decl_macro)]

use inference_engine;
use rocket::response::Redirect;
use rocket::{catch, catchers, get, routes, uri, Request};
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use serde_derive::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    question: Option<String>,
    answers: Vec<String>,
}

fn get_next() -> Option<(String, Vec<String>)> {
    let path = "assets/knowledge_base/plumbing_knowledge_base.lms";
    let inference_engine = inference_engine::prepare().with_knowledge_base_file(path);
    if let Some(question) = inference_engine.next_question() {
        Some((question.text, question.choices))
    } else {
        None
    }
}

#[get("/index")]
fn index() -> Template {
    let map: HashMap<String, String> = HashMap::new();
    Template::render("index", &map)
}

#[get("/")]
fn redirect() -> Redirect {
    Redirect::to("/index")
}

// #[get("/")]
// fn index() -> Redirect {
//     Redirect::to(uri!(get: name = "Unknown"))
// }

#[get("/hello/<name>")]
fn get(name: String) -> Template {
    let context = if let Some((question, answers)) = get_next() {
        TemplateContext {
            name,
            question: Some(question),
            answers,
        }
    } else {
        TemplateContext {
            name,
            question: None,
            answers: Vec::new(),
        }
    };
    Template::render("question", &context)
}

#[catch(404)] // Replace the default 404 with the definition below.
fn not_found(req: &Request) -> Template {
    let mut map = HashMap::new();
    map.insert("path", req.uri().path());
    Template::render("error/404", &map)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![redirect, index, get]) // Attach the routes specified above.
        .mount(
            "/static",
            StaticFiles::from(format!("{}/assets/static", env!("CARGO_MANIFEST_DIR"))),
        )
        .attach(Template::fairing()) // Attach the fairing that automagically reads the templates.
        .register(catchers![not_found]) // Attach the catchers to fire when a particular error is thrown
}

fn main() {
    rocket().launch();
}
