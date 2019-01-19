#![feature(proc_macro_hygiene, decl_macro)]
use std::env;

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
    debug: bool,
}

#[derive(Serialize, Debug, Default)]
/// Context on the current answered questions
/// For each atom in `atoms` there is a corresponding answer at the same index
/// in `selected_answers`.
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

                println!("{}", context);
                for pairs in context.split(',') {
                    let mut iter = pairs.split(':').filter(|&w| w != "").peekable();
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
    let path = "assets/knowledge_base/knowledge_base.yaml";
    let mut inference_engine = inference_engine::prepare().with_knowledge_base_file(path);
    if let Some(context) = context {
        for (atom, answer) in context.atoms.iter().zip(context.selected_answers.clone()) {
            let atom = Atom::new(atom.clone());
            inference_engine.add_state(atom, &answer);
        }
    }
    inference_engine
}

fn get_next_question(context: &Option<Context>) -> Option<(Question, Vec<String>)> {
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

#[get("/consult?<context>&<debug>")]
fn consult(context: Option<Context>, debug: Option<bool>) -> Template {
    // If `debug` is not provided the value is `false`.
    let debug = debug.unwrap_or_default();

    if let Some((question, answers)) = get_next_question(&context) {
        let template_context = TemplateContext {
            atom: Some(question.atom.text),
            question: Some(question.text),
            answers,
            context,
            debug,
        };
        Template::render("question", &template_context)
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
    let port = env::var("PORT").unwrap_or("8000".to_string());
    let port = port.parse().unwrap();
    let mut config = rocket::ignite().config().clone();
    config.set_port(port);
    rocket::custom(config)
        .mount("/", routes![redirect, index, consult]) // Attach the routes specified above.
        .mount("/static", StaticFiles::from("assets/static"))
        .attach(Template::fairing()) // Attach the fairing that automagically reads the templates.
        .register(catchers![not_found]) // Attach the catchers to fire when a particular error is thrown
}

fn main() {
    rocket().launch();
}
