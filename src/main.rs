extern crate iron;
extern crate mount;
extern crate router;
extern crate staticfile;
extern crate ws;
#[macro_use] extern crate conv;
#[macro_use] extern crate custom_derive;

mod board;

use iron::modifiers::Redirect;
use iron::status;
use iron::{Iron, Request, Response, IronResult, Url};
use mount::Mount;
use router::Router;
use staticfile::Static;
use ws::listen;

use std::path::Path;
use std::thread;

fn websocket_server_start() {
    println!("starting websocket server on :3012");

    if let Err(error) = listen("0.0.0.0:3012", |out| {
        move |msg| {
            println!("Server got message '{}'. ", msg);
            out.send(msg)
        }
    }) {
        println!("Failed to create WebSocket due to {:?}", error);
    }
}

fn redirect_to_root(req: &mut Request) -> IronResult<Response> {
    let redirect_url_str = format!("{}index.html", req.url);
    let url = Url::parse(&redirect_url_str).unwrap();

    Ok(Response::with((status::MovedPermanently, Redirect(url))))
}

fn web_server_start() {
    println!("starting web server on http://localhost:8080/");

    let mut router = Router::new();
    router.get("/", redirect_to_root, "index");

    let mut mount = Mount::new();
    mount
        .mount("/", router)
        .mount("/", Static::new(Path::new("site/")));

    Iron::new(mount)
        .http("0.0.0.0:8080")
        .unwrap_or_else(|error| panic!("Unable to start server: {}", error));
}

fn main() {
    let websocket_handler = thread::spawn(|| {
        websocket_server_start();
    });
    
    let webserver_handler = thread::spawn(|| {
        web_server_start();
    });

    websocket_handler.join().unwrap();
    webserver_handler.join().unwrap();
}
