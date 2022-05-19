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
use rocket::serde::json::Json;
// use ws::listen;

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

// fn websocket_server_start() {
//     println!("Starting websocket server on :3012");

//     if let Err(error) = listen("0.0.0.0:3012", |out| {
//         move |message| {
//             println!("Server got message '{}'. ", message);
//             out.send(message)
//         }
//     }) {
//         println!("Failed to create WebSocket due to {:?}", error);
//     }
// }

#[launch]
fn rocket() -> _ {
    let config = rocket::Config::figment()
        .merge(("port", 8080));

    rocket::custom(config).mount(
        "/",
        routes![
            redirect_to_root,
            serve_static_index,
            serve_static_image,
            create_game,
            play_piece
        ],
    )
}
