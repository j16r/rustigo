use std::path::{Path, PathBuf};

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;

use conv::TryFrom;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::response::stream::{EventStream, Event};
use rocket::serde::json::Json;
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::{State, Shutdown};

mod board;

#[get("/")]
fn redirect_to_root() -> Redirect {
    Redirect::to("/index.html")
}

#[get("/index.html")]
async fn serve_static_index() -> Option<NamedFile> {
    NamedFile::open(Path::new("site/index.html")).await.ok()
}

#[get("/images/<file..>")]
async fn serve_static_image(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("site/images/").join(file))
        .await
        .ok()
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
    match <board::Size as TryFrom<_>>::try_from(message.size) {
        Ok(size) => {
            let game = board::new(size);
            Ok(Json(GameStateMessage {
                board: board::encode(&game),
            }))
        }
        _ => Err(Status::UnprocessableEntity),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacePieceMessage {
    pub board: String,
    pub coordinate: board::Coordinate,
    pub stone: board::Stone,
}

#[put("/games", format = "application/json", data = "<message>")]
fn play_piece(message: Json<PlacePieceMessage>) -> Result<Json<GameStateMessage>, Status> {
    match board::decode(&message.board) {
        Some(mut game) => {
            if game.play_stone(message.coordinate, message.stone) {
                Ok(Json(GameStateMessage {
                    board: board::encode(&game),
                }))
            } else {
                println!("Invalid play {:?}:{:?}", message.coordinate, message.stone);
                Err(Status::UnprocessableEntity)
            }
        }
        _ => Err(Status::UnprocessableEntity),
    }
}

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate = "rocket::serde")]
struct Message {
    #[field(validate = len(..30))]
    pub room: String,
}

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

#[launch]
fn rocket() -> _ {
    let config = rocket::Config::figment()
        .merge(("port", 8080));

    rocket::custom(config)
        .manage(channel::<Message>(1024).0)
        .mount(
            "/",
            routes![
                redirect_to_root,
                serve_static_index,
                serve_static_image,
                create_game,
                play_piece,
                events
            ],
        )
}
