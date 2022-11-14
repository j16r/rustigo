use server;

#[shuttle_service::main]
async fn rocket() -> shuttle_service::ShuttleRocket {
    Ok(server::rocket())
}
