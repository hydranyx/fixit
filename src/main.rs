#![feature(proc_macro_hygiene, decl_macro)]

use inference_engine::{self, Atom, Question};
use rocket::http::RawStr;
use rocket::request::FromFormValue;
use rocket::response::Redirect;
use rocket::{catch, catchers, get, routes, uri, Request};
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use serde_derive::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct TemplateContext {
    atom: Option<String>,
    question: Option<String>,
    answers: Vec<String>,
    context: Option<Context>,
}

#[derive(Serialize, Debug, Default)]
struct Context {
    atoms: Vec<String>,
    selected_answers: Vec<String>,
}

impl<'f> FromFormValue<'f> for Context {
    type Error = (); // TODO Use more descriptive error.
    fn from_form_value(item: &RawStr) -> Result<Self, ()> {
        match item.percent_decode() {
            Ok(context) => {
                let mut atoms = Vec::new();
                let mut selected_answers = Vec::new();
                for pairs in context.split_whitespace() {
                    let mut iter = pairs.split(':').filter(|&w| w != "");
                    let atom = iter.next();
                    let answer = iter.next();

                    if let (Some(atom), Some(answer)) = (atom, answer) {
                        atoms.push(atom.to_string());
                        selected_answers.push(answer.to_string());
                    } else {
                        return Err(());
                    }
                }
                Ok(Context {
                    atoms,
                    selected_answers,
                })
            }
            Err(_) => Err(()),
        }
    }
}

fn create_inference_engine(context: &Option<Context>) -> inference_engine::InferenceEngine {
    let path = "assets/knowledge_base/plumbing_knowledge_base.lms";
    let mut inference_engine = inference_engine::prepare().with_knowledge_base_file(path);
    if let Some(context) = context {
        for (atom, answer) in context.atoms.iter().zip(context.selected_answers.clone()) {
            let atom = Atom::new(atom.clone());
            inference_engine.add_state(atom, &answer);
        }
    }
    inference_engine
}

fn get_next(context: &Option<Context>) -> Option<(Question, Vec<String>)> {
    let inference_engine = create_inference_engine(context);

    if let Some(question) = inference_engine.next_question() {
        Some((question.clone(), question.choices))
    } else {
        None
    }
}

fn get_answer(context: &Option<Context>) -> Option<String> {
    let inference_engine = create_inference_engine(context);

    if let Some(answer) = inference_engine.reached_goal() {
        Some(answer.text)
    } else {
        None
    }
}

#[get("/")]
fn redirect() -> Redirect {
    Redirect::to(uri!(index))
}

#[get("/index")]
fn index() -> Template {
    let map: HashMap<String, String> = HashMap::new();
    Template::render("index", &map)
}

#[get("/consult?<context>")]
fn consult(context: Option<Context>) -> Template {
    if let Some((question, answers)) = get_next(&context) {
        let context = TemplateContext {
            atom: Some(question.atom.text),
            question: Some(question.text),
            answers,
            context,
        };
        Template::render("question", &context)
    } else {
        let mut template_context = HashMap::new();
        template_context.insert("answer", get_answer(&context));
        Template::render("done", &template_context)
    }
}

#[catch(404)] // Replace the default 404 with the definition below.
fn not_found(req: &Request) -> Template {
    let mut map = HashMap::new();
    map.insert("path", req.uri().path());
    Template::render("error/404", &map)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![redirect, index, consult]) // Attach the routes specified above.
        .mount("/static", StaticFiles::from("assets/static"))
        .attach(Template::fairing()) // Attach the fairing that automagically reads the templates.
        .register(catchers![not_found]) // Attach the catchers to fire when a particular error is thrown
}

fn main() {
    rocket().launch();
}
