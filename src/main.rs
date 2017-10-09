extern crate iron;
extern crate logger;
extern crate mount;
#[macro_use]
extern crate router;
extern crate staticfile;
extern crate ws;
#[macro_use] extern crate conv;
#[macro_use] extern crate custom_derive;
extern crate env_logger;
extern crate urlencoded;

mod board;

use iron::prelude::*;
use iron::modifiers::Redirect;
use iron::status;
use iron::{Iron, Request, Response, IronResult, Url, Chain};
use logger::Logger;
use mount::Mount;
use router::Router;
use staticfile::Static;
use ws::listen;
use urlencoded::UrlEncodedQuery;

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

fn create_game(req: &mut Request) -> IronResult<Response> {
    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            match hashmap.get("size") {
                Some(ref values) => {
                    if values.len() != 1 {
                        return Ok(Response::with(status::UnprocessableEntity));
                    }

                    match values[0].parse::<u8>() {
                        Ok(ref value) => {
                            Ok(Response::with((status::Ok, "hello")))
                        },
                        Err(ref e) => return Ok(Response::with(status::UnprocessableEntity)),
                    }
                },
                None => return Ok(Response::with(status::UnprocessableEntity)),
            }
        },
        Err(ref e) => return Ok(Response::with(status::UnprocessableEntity)),
    }
}

fn web_server_start() {
    println!("starting web server on http://localhost:8080/");

    let router = router!(
        index: get "/" => redirect_to_root,
        post: post "/games" => create_game);

    let mut mount = Mount::new();
    mount
        .mount("/", router)
        .mount("/images/", Static::new(Path::new("site/images/")))
        .mount("/index.html", Static::new(Path::new("site/index.html")));

    let (logger_before, logger_after) = Logger::new(None);
    let mut chain = Chain::new(mount);
    chain.link_before(logger_before);
    chain.link_after(logger_after);

    Iron::new(chain)
        .http("0.0.0.0:8080")
        .unwrap_or_else(|error| panic!("Unable to start server: {}", error));
}

fn main() {
    env_logger::init().unwrap();

    let websocket_handler = thread::spawn(|| {
        websocket_server_start();
    });
    
    let webserver_handler = thread::spawn(|| {
        web_server_start();
    });

    websocket_handler.join().unwrap();
    webserver_handler.join().unwrap();
}
