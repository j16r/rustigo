#![feature(plugin)]
#![plugin(rocket_codegen)]

//extern crate logger;
extern crate ws;
#[macro_use] extern crate conv;
#[macro_use] extern crate custom_derive;
extern crate env_logger;
extern crate rocket;
extern crate rocket_contrib;

mod board;

use std::path::{Path, PathBuf};
use std::thread;
extern crate serde;
#[macro_use]
extern crate serde_derive;
use rocket::response::{NamedFile, Redirect};
use rocket::http::Status;
use rocket_contrib::json::Json;
//use logger::Logger;
use ws::listen;
use conv::TryFrom;

#[get("/")]
fn redirect_to_root() -> Redirect {
    Redirect::to("/index.html")
}

#[get("/index.html")]
fn serve_static_index() -> Option<NamedFile> {
    NamedFile::open(Path::new("site/index.html")).ok()
}

#[get("/images/<file..>")]
fn serve_static_image(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("site/images/").join(file)).ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewGameMessage {
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameStateMessage {
    pub board: String,
}

#[post("/games", format = "application/json", data = "<message>")]
fn create_game(message: Json<NewGameMessage>) -> Result<Json<GameStateMessage>, Status> {
    match board::Size::try_from(message.size) {
        Ok(size) => {
            let game = board::new(size);
            Ok(Json(GameStateMessage{
                board: board::encode(&game),
            }))
        },
        _ => return Err(Status::UnprocessableEntity),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacePieceMessage {
    pub board: String,
    pub coordinate: board::Coordinate,
    pub stone : board::Stone,
}

#[put("/games", format = "application/json", data = "<message>")]
fn play_piece(message: Json<PlacePieceMessage>) -> Result<Json<GameStateMessage>, Status> {
    match board::decode(&message.board) {
        Some(mut game) => {
            if game.play_stone(message.coordinate, message.stone) {
                Ok(Json(GameStateMessage{
                    board: board::encode(&game),
                }))
            } else {
                println!("Invalid play {:?}:{:?}", message.coordinate, message.stone);
                return Err(Status::UnprocessableEntity);
            }
        },
        _ => return Err(Status::UnprocessableEntity),
    }
}

fn websocket_server_start() {
    println!("Starting websocket server on :3012");

    if let Err(error) = listen("0.0.0.0:3012", |out| {
        move |message| {
            println!("Server got message '{}'. ", message);
            out.send(message)
        }
    }) {
        println!("Failed to create WebSocket due to {:?}", error);
    }
}

fn web_server_start() {
    let config = rocket::config::Config::build(rocket::config::Environment::Development)
        .address("0.0.0.0")
        .port(8080)
        .finalize().expect("Could not create config");

    rocket::custom(config, false)
        .mount("/", routes![
               redirect_to_root,
               serve_static_index,
               serve_static_image,
               create_game,
               play_piece])
        .launch();
}

fn main() {
    env_logger::init();

    let websocket_handler = thread::spawn(|| {
        websocket_server_start();
    });
    
    let webserver_handler = thread::spawn(|| {
        web_server_start();
    });

    websocket_handler.join().unwrap();
    webserver_handler.join().unwrap();
}
