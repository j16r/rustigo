#[macro_use]
extern crate rocket;

use server;

#[launch]
fn rocket() -> _ {
    server::rocket()
}
