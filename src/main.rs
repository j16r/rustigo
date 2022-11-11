use std::path::PathBuf;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rocket_include_static_resources;

use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::stream::{Event, EventStream};
use rocket::response::Redirect;
use rocket::serde::json::{from_str, to_string, Json};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BlackGameState {
    size: u8,
    private_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WhiteGameState {
    size: u8,
    public_key: String,
}

#[get("/new?<size..>")]
fn serve_new_game(size: board::Size, cookies: &CookieJar<'_>) -> Redirect {
    let game_id = Uuid::new_v4();
    let size = size as u8;

    let black_game_state = BlackGameState {
        size,
        private_key: "".to_string(),
    };
    let mut game_cookie = Cookie::named("b");
    game_cookie.set_value(to_string(&black_game_state).unwrap());
    cookies.add(game_cookie);

    Redirect::to(format!("/{}/game.html", game_id))
}

#[get("/<game_id>/join.html")]
fn serve_join_game(game_id: Uuid) -> Template {
    Template::render("join", context! { game_id })
}

#[get("/<game_id>/game.html")]
fn serve_game(game_id: Uuid, cookies: &CookieJar<'_>) -> Template {
    if let Some(game_cookie) = cookies.get("b") {
        let black_game_state: BlackGameState = from_str(game_cookie.value()).unwrap();

        let size = black_game_state.size;

        let board_size = (1..=size).collect::<Vec<_>>();
        let piece_size = format!("{:.2}", 80.0 / size as f32);
        Template::render(
            "board",
            context! { game_id, size, board_size, piece_size, black_player: true },
        )
    } else if let Some(game_cookie) = cookies.get("w") {
        let white_game_state: WhiteGameState = from_str(game_cookie.value()).unwrap();

        let size = white_game_state.size;

        let board_size = (1..=size).collect::<Vec<_>>();
        let piece_size = format!("{:.2}", 80.0 / size as f32);
        Template::render(
            "board",
            context! { game_id, size, board_size, piece_size, black_player: false },
        )
    } else {
        unimplemented!("404 here");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStateMessage {
    Join { id: Uuid },
    JoinAccepted { id: Uuid, size: u8 },
    Update { board: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptPlayerMessage {
    pub size: board::Size,
}

#[put("/<game_id>/players", format = "application/json", data = "<message>")]
fn accept_player(
    game_id: Uuid,
    message: Json<AcceptPlayerMessage>,
    queue: &State<Sender<GameStateMessage>>,
) -> Result<Json<GameStateMessage>, Status> {
    let state = GameStateMessage::JoinAccepted {
        id: game_id.clone(),
        size: message.size as u8,
    };
    let result = queue.send(state.clone());
    if result.is_err() {
        eprintln!("Failed to post to SSE queue {:?}", result.err());
        // TODO: 500
    }
    Ok(Json(state))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinMessage {}

#[put("/<game_id>/joins", format = "application/json", data = "<message>")]
fn request_join(
    game_id: Uuid,
    message: Json<JoinMessage>,
    queue: &State<Sender<GameStateMessage>>,
) -> Result<Json<GameStateMessage>, Status> {
    let state = GameStateMessage::Join { id: game_id };
    let result = queue.send(state.clone());
    if result.is_err() {
        eprintln!("Failed to post to SSE queue {:?}", result.err());
        // TODO: 500
    }
    Ok(Json(state))
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
            Ok(game) => game,
            Err(err) => {
                println!("Invalid board {:?}, error: {:?}", message.board, err);
                return Err(Status::UnprocessableEntity);
            }
        }
    };

    dbg!(&game);

    if game.play_stone(message.coordinate, message.stone) {
        println!(
            "Valid play {:?}:{:?}, new game: {:?}",
            message.coordinate, message.stone, &game
        );
        let state = GameStateMessage::Update {
            board: board::encode(&game),
        };
        let result = queue.send(state.clone());
        if result.is_err() {
            eprintln!("Failed to post to SSE queue {:?}", result.err());
            // TODO: 500
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
                "join.png" => "site/images/join.png",
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
                serve_new_game,
                serve_join_game,
                serve_game,
                accept_player,
                request_join,
                play_piece,
                events
            ],
        )
}
