use std::path::PathBuf;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rocket_include_static_resources;

use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::serde::uuid::Uuid;
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::{channel, error::RecvError, Sender};
use rocket::{Shutdown, State};
use rocket_dyn_templates::{context, Template};
use rocket_include_static_resources::{EtagIfNoneMatch, StaticContextManager, StaticResponse};

mod board;

#[get("/")]
fn redirect_to_root() -> Redirect {
    Redirect::to("/index.html")
}

#[get("/index.html")]
fn serve_index(
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

#[get("/images/<file..>", rank = 1)]
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

#[get("/board.html?<size..>")]
fn serve_new_board(size: board::Size) -> Redirect {
    let game_id = Uuid::new_v4();
    let size = size as u8;
    Redirect::to(format!("/{}/board.html?size={}", game_id, size))
}

#[get("/<game_id>/board.html?<size..>")]
fn serve_board(game_id: Uuid, size: board::Size) -> Template {
    let piece_size = format!("{:.2}", 80.0 / ((size as u8) as f32));
    let size = (1..=(size as u8)).collect::<Vec<_>>();
    Template::render("board", context! { size, piece_size })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameStateMessage {
    pub board: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacePieceMessage {
    pub board: String,
    pub coordinate: board::Coordinate,
    pub stone: board::Stone,
    pub size: board::Size,
}

#[put("/<game_id>/games", format = "application/json", data = "<message>")]
fn play_piece(
    game_id: Uuid,
    message: Json<PlacePieceMessage>,
    queue: &State<Sender<GameStateMessage>>,
) -> Result<Json<GameStateMessage>, Status> {
    println!(
        "Got play {:?}:{:?} = {:?}",
        message.coordinate, message.stone, message.board
    );

    let mut game = if message.board.is_empty() {
        println!("Board empty, initialize a new one");
        board::new(message.size)
    } else {
        match board::decode(&message.board) {
            Some(game) => game,
            _ => return Err(Status::UnprocessableEntity),
        }
    };

    dbg!(&game);

    if game.play_stone(message.coordinate, message.stone) {
        println!(
            "Valid play {:?}:{:?}, new game: {:?}",
            message.coordinate, message.stone, &game
        );
        let state = GameStateMessage {
            board: board::encode(&game),
        };
        let result = queue.send(state.clone());
        if result.is_err() {
            eprintln!("Failed to post to SSE queue {:?}", result.err());
        }
        Ok(Json(state))
    } else {
        println!("Invalid play {:?}:{:?}", message.coordinate, message.stone);
        Err(Status::UnprocessableEntity)
    }
}

#[get("/<game_id>/events")]
async fn events(
    game_id: Uuid,
    queue: &State<Sender<GameStateMessage>>,
    mut end: Shutdown,
) -> EventStream![] {
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
        .attach(Template::custom(move |engines| {
            engines.handlebars.set_strict_mode(true);
        }))
        .manage(channel::<GameStateMessage>(1024).0)
        .mount(
            "/",
            routes![
                redirect_to_root,
                serve_static_favicon,
                serve_static_image,
                serve_index,
                serve_new_board,
                serve_board,
                play_piece,
                events
            ],
        )
}
