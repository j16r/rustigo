use std::path::PathBuf;

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rocket_include_static_resources;

use conv::TryFrom;
use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::{channel, error::RecvError, Sender};
use rocket::{Shutdown, State};
use rocket_include_static_resources::{EtagIfNoneMatch, StaticContextManager, StaticResponse};

mod board;

#[get("/")]
fn redirect_to_root() -> Redirect {
    Redirect::to("/index.html")
}

#[get("/index.html")]
fn serve_static_index(
    static_resources: &State<StaticContextManager>,
    etag_if_none_match: EtagIfNoneMatch,
) -> StaticResponse {
    static_resources.build(&etag_if_none_match, "index")
}

#[get("/favicon.ico")]
fn serve_static_favicon(
    static_resources: &State<StaticContextManager>,
    etag_if_none_match: EtagIfNoneMatch,
) -> StaticResponse {
    static_resources.build(&etag_if_none_match, "favicon")
}

#[get("/images/<file..>")]
fn serve_static_image(
    file: PathBuf,
    static_resources: &State<StaticContextManager>,
    etag_if_none_match: EtagIfNoneMatch,
) -> Result<StaticResponse, Status> {
    let name = file.to_str().ok_or(Status::UnprocessableEntity)?;
    static_resources
        .try_build(&etag_if_none_match, name)
        .map_err(|_| Status::NotFound)
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
    let config = rocket::Config::figment().merge(("port", 8080));

    rocket::custom(config)
        .attach(static_resources_initializer!(
                "index" => "site/index.html",
                "favicon" => "site/images/favicon.ico",

                "blackpiece.png" => "site/images/blackpiece.png",
                "whitepiece.png" => "site/images/whitepiece.png",
                "tilecenter.png" => "site/images/tilecenter.png",
        ))
        .manage(channel::<Message>(1024).0)
        .mount(
            "/",
            routes![
                redirect_to_root,
                serve_static_favicon,
                serve_static_index,
                serve_static_image,
                create_game,
                play_piece,
                events
            ],
        )
}
