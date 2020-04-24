use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::lib::auth::Claims;
use crate::game::player;
use crate::AppState;

#[derive(Copy, Clone, Serialize, Deserialize)]
enum LobbyStatus{
    Gathering,
    InProgress
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Lobby {
    id: Uuid,
    status: LobbyStatus,
    creator: Option<player::PlayerID>,
}

#[get("/")]
pub async fn get_lobbies(state: web::Data<AppState>, claims: Claims) -> Option<HttpResponse> {
    Some(HttpResponse::Ok()
        .json(state.lobbies
            .read()
            .unwrap()
            .iter()
            .map(|(_, lobby)| lobby.clone())
            .collect::<Vec<Lobby>>()
        )
    )
}

#[get("/{id}")]
pub async fn get_lobby(info: web::Path<(Uuid,)>, state: web::Data<AppState>) -> Option<HttpResponse> {
    let lobbies = state.lobbies.read().unwrap();
    lobbies
        .get(&info.0)
        .map(| lobby | {
            HttpResponse::Ok().json(lobby)
        })
}

#[post("/")]
pub async fn create_lobby(state: web::Data<AppState>, claims: Claims) -> Option<HttpResponse> {
    let id = Uuid::new_v4();
    let mut lobbies = state.lobbies.write().unwrap();
    lobbies.insert(id, Lobby{
        id: id,
        status: LobbyStatus::Gathering,
        creator: Some(claims.pid)
    });
    Some(HttpResponse::Created().json(lobbies.get(&id)))
}
