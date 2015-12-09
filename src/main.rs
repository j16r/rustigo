extern crate ws;
#[macro_use] extern crate conv;
#[macro_use] extern crate custom_derive;

mod board;

use ws::listen;

fn main() {
    if let Err(error) = listen("127.0.0.1:3012", |out| {
        move |msg| {
            println!("Server got message '{}'. ", msg);
            out.send(msg)
        }
    }) {
        println!("Failed to create WebSocket due to {:?}", error);
    }
}
