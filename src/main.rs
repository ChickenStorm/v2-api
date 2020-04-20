use actix_web::{web, App, HttpServer};
use game::lobby;
use std::collections::HashMap;
use uuid::Uuid;
use actix::Actor;
use std::clone::Clone;
use std::sync::{Arc, Mutex};

mod ws;
mod game;

pub struct AppState {
    lobbies: Arc<Mutex<HashMap<Uuid, lobby::Lobby>>>, // <- Mutex is necessary to mutate safely across threads
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Start chat server actor
    let server = ws::server::LobbyWebsocket::default().start();
    let state = web::Data::new(AppState {
        lobbies: Arc::new(Mutex::new(HashMap::new())),
    });

    HttpServer::new(move || {
        App::new()
            .data(server.clone())
            .app_data(state.clone())
            .service(
                web::scope("/lobbies")
                .service(lobby::create_lobby)
                .service(lobby::get_lobbies)
                .service(lobby::get_lobby)
            )
            .service(web::resource("/ws/").to(ws::client::entrypoint))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}