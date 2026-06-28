use askama::Template;
use axum::{
    routing::get,
    response::{Html, IntoResponse},
    Json,
    Router,
};
use axum::extract::Path;
use reqwest::header::AUTHORIZATION;
use tracing_subscriber::EnvFilter;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::env;

use tracing::{info, warn, error};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct BasisHealthResponse {
    listening: bool,
    visitors: u32,
    capacity: u32,
    sent: u64,
    recv: u64,
    current_time: String,
    start_time: String,
    version: String,
}

#[derive(Deserialize, Debug, Clone)]
struct LocaleDict {
    title: String,
    status_label: String,
    status_listening: String,
    status_stopped: String,
    players_label: String,
    version_label: String,
    sent_label: String,
    recv_label: String,
    start_time_label: String,
    current_time_label: String,
    loading: String,
    error_title: String,
    error_desc: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    i18n: LocaleDict,
}

#[derive(Template)]
#[template(path = "card_inner.html")]
struct CardInnerTemplate {
    data: BasisHealthResponse,
    i18n: LocaleDict,
}

#[derive(Template)]
#[template(path = "error_inner.html")]
struct ErrorInnerTemplate {
    msg: String,
    i18n: LocaleDict,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Player {
    #[serde(rename = "netId")]
    net_id: u64,
    uuid: String,
    #[serde(rename = "displayName")]
    display_name: String,
    platform: String,
}

#[derive(Deserialize, Debug)]
struct PlayersResponse {
    players: Vec<Player>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct World {
    #[serde(rename = "netId")]
    net_id: String,
    url: String,
    persistent: bool,
    #[serde(rename = "adminLocked")]
    admin_locked: bool,
    strategy: u32,
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct WorldsResponse {
    worlds: Vec<World>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct LoadWorldPayload {
    url: String,
    password: Option<String>,
    persistent: bool,
    strategy: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct SwitchWorldPayload {
    url: String,
    password: Option<String>,
    persistent: bool,
    strategy: String,
    announce: Option<String>,
    delay: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct AnnouncePayload {
    message: String,
}

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminTemplate {
    i18n: LocaleDict,
}

#[derive(Template)]
#[template(path = "players_inner.html")]
struct PlayersInnerTemplate {
    players: Vec<Player>,
    i18n: LocaleDict,
}

#[derive(Template)]
#[template(path = "worlds_inner.html")]
struct WorldsInnerTemplate {
    worlds: Vec<World>,
    i18n: LocaleDict,
}

#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("MarusansiBasisDashboard=info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::DELETE]);

    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "4000".to_string())
        .parse()
        .expect("PORT must be a valid number");

    let app = Router::new()
        .route("/", get(handle_index))
        .route("/api/health", get(handle_health))
        .route("/api/health/json", get(handle_health_json))
        .route("/admin", get(handle_admin))
        .route("/admin/players", get(handle_get_players))
        .route("/admin/worlds", get(handle_get_worlds))
        .route("/admin/worlds/load", axum::routing::post(handle_post_world_load))
        .route("/admin/worlds/switch", axum::routing::post(handle_switch_world))
        .route("/admin/worlds/clear", axum::routing::delete(handle_clear_worlds))
        .route("/admin/worlds/{net_id}", axum::routing::delete(handle_delete_world))
        .route("/admin/announce", axum::routing::post(handle_announce_all))
        .route("/admin/announce/{uuid}", axum::routing::post(handle_announce_user))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("🚀 Dashboard Server running on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

fn get_locale(headers: &HeaderMap) -> LocaleDict {
    let accept_lang = headers
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let toml_str = if accept_lang.contains("ja") {
        include_str!("../locales/ja.toml")
    } else {
        include_str!("../locales/en.toml")
    };

    toml::from_str(toml_str).unwrap()
}

async fn send_api_request<T: Serialize>(
    method: reqwest::Method,
    path: &str,
    body: Option<T>,
) -> Result<reqwest::Response, String> {
    let api_base = env::var("API_URL_BASE").unwrap_or_else(|_| "http://localhost:10667".to_string());
    let token = env::var("API_BEARER_TOKEN").unwrap_or_else(|_| "default_token".to_string());

    let client = reqwest::Client::new();
    let url = format!("{}{}", api_base, path);

    info!("Sending internal API request: {} {}", method, url);

    let mut req = client.request(method.clone(), &url)
        .header(AUTHORIZATION, format!("Bearer {}", token));

    if let Some(b) = body {
        req = req.json(&b);
    }

    let response = req.send().await.map_err(|e| {
        error!("Failed to send internal API request ({} {}): {}", method, url, e);
        e.to_string()
    })?;

    if !response.status().is_success() {
        warn!("Internal API ({} {}) responded with error status: {}", method, url, response.status());
    } else {
        info!("Internal API ({} {}) completed with status: {}", method, url, response.status());
    }

    Ok(response)
}

async fn fetch_health_data() -> Result<BasisHealthResponse, String> {
    let health_port = env::var("HEALTH_PORT").unwrap_or_else(|_| "10666".to_string());
    let api_url = format!("http://localhost:{}/health", health_port);

    info!("Fetching health data from: {}", api_url);

    let response = reqwest::get(&api_url).await.map_err(|e| {
        error!("Failed to connect to health API ({}): {}", api_url, e);
        e.to_string()
    })?;

    if !response.status().is_success() {
        let err_msg = format!("Status: {}", response.status());
        warn!("Health API returned non-success status: {}", err_msg);
        return Err(err_msg);
    }

    let text = response.text().await.map_err(|e| {
        error!("Failed to read health API response string: {}", e);
        e.to_string()
    })?;

    let res_data = serde_json::from_str::<BasisHealthResponse>(&text).map_err(|e| {
        error!("JSON deserialization failed for health data. Raw text: [{}], Error: {}", text, e);
        e.to_string()
    })?;

    Ok(res_data)
}

async fn handle_index(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);
    Html(IndexTemplate { i18n }.render().unwrap())
}

async fn handle_health(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);

    let html_content = match fetch_health_data().await {
        Ok(res_data) => CardInnerTemplate { data: res_data, i18n }.render().unwrap(),
        Err(err_msg) => {
            warn!("Rendering error card template due to: {}", err_msg);
            ErrorInnerTemplate { msg: err_msg, i18n }.render().unwrap()
        }
    };

    Html(html_content)
}

async fn handle_health_json() -> impl IntoResponse {
    match fetch_health_data().await {
        Ok(res_data) => (StatusCode::OK, Json(res_data)).into_response(),
        Err(err_msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err_msg })),
        ).into_response(),
    }
}

async fn handle_admin(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);
    Html(AdminTemplate { i18n }.render().unwrap())
}

async fn handle_get_players(headers: HeaderMap) -> impl IntoResponse {
    let i18n = get_locale(&headers);
    match send_api_request::<()>(reqwest::Method::GET, "/api/players", None).await {
        Ok(res) if res.status().is_success() => {
            let res_body = res.json::<PlayersResponse>().await.unwrap_or(PlayersResponse { players: vec![] });
            let players = res_body.players;
            Html(PlayersInnerTemplate { players, i18n }.render().unwrap()).into_response()
        }
        Ok(res) => {
            error!("Failed to get players. Backend status: {}", res.status());
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch players").into_response()
        }
        Err(err) => {
            error!("Player list fetch threw system error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch players").into_response()
        }
    }
}

async fn handle_get_worlds(headers: HeaderMap) -> impl IntoResponse {
    let i18n = get_locale(&headers);
    match send_api_request::<()>(reqwest::Method::GET, "/api/worlds", None).await {
        Ok(res) if res.status().is_success() => {
            let res_body = res.json::<WorldsResponse>().await.unwrap_or(WorldsResponse { worlds: vec![] });
            let worlds = res_body.worlds;
            Html(WorldsInnerTemplate { worlds, i18n }.render().unwrap()).into_response()
        }
        Ok(res) => {
            error!("Failed to get worlds. Backend status: {}", res.status());
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch worlds").into_response()
        }
        Err(err) => {
            error!("World list fetch threw system error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch worlds").into_response()
        }
    }
}

async fn handle_post_world_load(Json(payload): Json<LoadWorldPayload>) -> impl IntoResponse {
    info!("Triggering world load for URL: {}", payload.url);
    match send_api_request(reqwest::Method::POST, "/api/worlds", Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn handle_delete_world(Path(net_id): Path<String>) -> impl IntoResponse {
    info!("Triggering world unload for ID: {}", net_id);
    match send_api_request::<()>(reqwest::Method::DELETE, &format!("/api/worlds/{}", net_id), None).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn handle_clear_worlds() -> impl IntoResponse {
    info!("Triggering clear-all worlds command");
    match send_api_request::<()>(reqwest::Method::DELETE, "/api/worlds", None).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn handle_switch_world(Json(payload): Json<SwitchWorldPayload>) -> impl IntoResponse {
    info!("Triggering world switch sync to URL: {} with delay: {}s", payload.url, payload.delay);
    match send_api_request(reqwest::Method::POST, "/api/worlds/switch", Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn handle_announce_all(Json(payload): Json<AnnouncePayload>) -> impl IntoResponse {
    info!("Broadcasting global announcement: {}", payload.message);
    match send_api_request(reqwest::Method::POST, "/api/announce", Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn handle_announce_user(Path(uuid): Path<String>, Json(payload): Json<AnnouncePayload>) -> impl IntoResponse {
    info!("Sending targeted announcement to user [{}]: {}", uuid, payload.message);
    match send_api_request(reqwest::Method::POST, &format!("/api/announce/{}", uuid), Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
