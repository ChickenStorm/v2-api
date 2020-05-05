use actix_web::{delete, get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    lib::auth::Claims,
    game::player,
    ws::protocol,
    AppState,
};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Debug)]
pub struct LobbyID(Uuid);

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum LobbyStatus{
    Gathering,
    InProgress,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Lobby {
    pub id: LobbyID,
    pub status: LobbyStatus,
    pub creator: Option<player::PlayerID>,
    pub players: HashSet<player::PlayerID>,
}

impl Lobby {
    pub fn ws_broadcast<T: 'static>(
        &self,
        players: &HashMap<player::PlayerID, player::Player>,
        message: &protocol::Message<T>,
        skip_id: Option<&player::PlayerID>
    ) where
        T: Clone + Send + Serialize
    {
        for (id, player) in players.iter() {
            if Some(id) != skip_id && self.players.contains(id) {
                player.websocket.as_ref().map(|ws| {
                    ws.do_send(message.clone());
                });
            }
        }
    }

    pub fn has_player(&self, pid: player::PlayerID) -> bool {
        self.players.contains(&pid)
    }
}

#[get("/")]
pub async fn get_lobbies(state: web::Data<AppState>) -> Option<HttpResponse> {
    #[derive(Serialize)]
    struct LobbyData{
        id: LobbyID,
        status: LobbyStatus,
        creator: Option<player::PlayerData>,
        nb_players: usize
    }
    Some(HttpResponse::Ok()
        .json(state.lobbies
            .read()
            .unwrap()
            .iter()
            .map(|(_, lobby)| LobbyData{
                id: lobby.id.clone(),
                status: lobby.status.clone(),
                creator: Some(state.players.read().unwrap().get(&lobby.creator.unwrap()).unwrap().data.clone()),
                nb_players: lobby.players.len()
            })
            .collect::<Vec<LobbyData>>()
        )
    )
}

#[get("/{id}")]
pub async fn get_lobby(state: web::Data<AppState>, info: web::Path<(LobbyID,)>) -> Option<HttpResponse> {
    let lobbies = state.lobbies.read().unwrap();
    let players = state.players.read().unwrap();
    lobbies
        .get(&info.0)
        .map(| lobby | {
            #[derive(Serialize)]
            struct LobbyData{
                id: LobbyID,
                status: LobbyStatus,
                creator: Option<player::PlayerData>,
                players: HashSet<player::PlayerData>,
            }
            let mut data = LobbyData{
                id: lobby.id.clone(),
                status: lobby.status.clone(),
                creator: Some(players.get(&lobby.creator.unwrap()).unwrap().data.clone()),
                players: HashSet::new()
            };
            for pid in lobby.players.iter() {
                data.players.insert(players.get(pid).unwrap().data.clone());
            }
            HttpResponse::Ok().json(data)
        })
}

#[post("/")]
pub async fn create_lobby(state: web::Data<AppState>, claims: Claims) -> Option<HttpResponse> {
    let id = LobbyID(Uuid::new_v4());
    let data = state.players.write().unwrap().get_mut(&claims.pid).map(|p| {
        if p.data.lobby != None {
            panic!("player is already in a lobby");
        }
        p.data.lobby = Some(id.clone());
        p.data.clone()
    })?;
    let mut lobbies = state.lobbies.write().unwrap();
    lobbies.insert(id.clone(), Lobby {
        id: id.clone(),
        status: LobbyStatus::Gathering,
        creator: Some(claims.pid.clone()),
        players: [claims.pid].iter().cloned().collect(),
    });

    state.ws_broadcast(&protocol::Message::<Lobby>{
        action: protocol::Action::LobbyCreated,
        data: lobbies.get(&id.clone()).unwrap().clone()
    }, Some(data.id.clone()), Some(true));

    Some(HttpResponse::Created().json(lobbies.get(&id)))
}

#[delete("/{id}/players/")]
pub async fn leave_lobby(state:web::Data<AppState>, claims:Claims, info:web::Path<(LobbyID,)>)
    -> Option<HttpResponse>
{
    let mut players = state.players.write().unwrap();
    let mut lobbies = state.lobbies.write().unwrap();
    let mut lobby = lobbies.get_mut(&info.0).unwrap();
    let data = players.get_mut(&claims.pid).map(|p| {
        if p.data.lobby != Some(lobby.id.clone()) {
            panic!("player was not in this lobby")
        }
        p.data.lobby = None;
        p.data.clone()
    })?;
    lobby.players.remove(&claims.pid);
    lobby.ws_broadcast(&players, &protocol::Message::<player::PlayerData>{
        action: protocol::Action::PlayerDisconnected,
        data: data.clone()
    }, Some(&claims.pid));
    drop(players);

    if lobby.players.is_empty() {
        state.ws_broadcast(&protocol::Message::<Lobby>{
            action: protocol::Action::LobbyRemoved,
            data: lobby.clone()
        }, Some(data.id.clone()), Some(true));
        lobbies.remove(&info.0);
    }
    Some(HttpResponse::Ok().finish())
}

#[post("/{id}/players/")]
pub async fn join_lobby(info: web::Path<(LobbyID,)>, state: web::Data<AppState>, claims: Claims)
    -> Option<HttpResponse>
{
    let mut lobbies = state.lobbies.write().unwrap();
    let mut players = state.players.write().unwrap();
    let mut lobby = lobbies.get_mut(&info.0)?;
    let data = players.get_mut(&claims.pid).map(|p| {
        if p.data.lobby != None {
            panic!("already joined a lobby")
        }
        p.data.lobby = Some(lobby.id.clone());
        p.data.clone()
    })?;

    lobby.players.insert(claims.pid);
    lobby.ws_broadcast(&players, &protocol::Message::<player::PlayerData>{
        action: protocol::Action::PlayerJoined,
        data
    }, Some(&claims.pid));

    Some(HttpResponse::NoContent().finish())
}