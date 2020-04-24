use actix_web::{get, post, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::lib::auth;

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: Uuid,
    pub username: String
}

#[post("/login")]
pub async fn login() -> Option<HttpResponse> {
    let player = Player {
        id: Uuid::new_v4(),
        username: String::from("")
    };
    #[derive(Serialize)]
    struct TokenResponse {
        token: String
    };
    Some(auth::create_jwt(auth::Claims { player, exp: 10000000000 })
        .map(|token| HttpResponse::Ok().json(TokenResponse{ token }))
        .map_err(|_| HttpResponse::Unauthorized())
        .unwrap())
}

#[get("/me")]
pub async fn get_current_player(claims: auth::Claims) -> Option<HttpResponse> {
    Some(HttpResponse::Ok().json(claims.player))
}