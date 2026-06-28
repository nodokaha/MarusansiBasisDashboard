use askama::Template;
use axum::{
    routing::get,
    response::{Html, IntoResponse},
    Json,
    Router,
};
use axum::extract::Path;
use reqwest::header::AUTHORIZATION;
use tower_http::cors::{Any, CorsLayer};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::env;

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct World {
    id: String,
    name: Option<String>,
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
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET]);

    // 1. サーバー起動ポートを環境変数から取得 (デフォルト: 4000)
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "4000".to_string())
        .parse()
        .expect("PORT must be a valid number");

 let app = Router::new()
    .route("/", get(handle_index))
    .route("/api/health", get(handle_health))
    .route("/api/health/json", get(handle_health_json))
    // --- 新設：操作用コントロールパネルのエンドポイント ---
    .route("/admin", get(handle_admin))
    .route("/admin/players", get(handle_get_players))
    .route("/admin/worlds", get(handle_get_worlds))
    .route("/admin/worlds/load", axum::routing::post(handle_post_world_load))
    .route("/admin/worlds/switch", axum::routing::post(handle_switch_world))
    .route("/admin/worlds/clear", axum::routing::delete(handle_clear_worlds))
    .route("/admin/worlds/{id}", axum::routing::delete(handle_delete_world))
    .route("/admin/announce", axum::routing::post(handle_announce_all))
    .route("/admin/announce/{uuid}", axum::routing::post(handle_announce_user))
    .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("🚀 Dashboard Server running on http://{}", addr);
    
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
    
    let mut req = client.request(method, &url)
        .header(AUTHORIZATION, format!("Bearer {}", token));

    if let Some(b) = body {
        req = req.json(&b);
    }

    req.send().await.map_err(|e| e.to_string())
}

// 共通の外部APIリクエスト処理用ヘルパー関数
async fn fetch_health_data() -> Result<BasisHealthResponse, String> {
    // 2. Healthポートを環境変数から取得 (デフォルト: 10666)
    let health_port = env::var("HEALTH_PORT").unwrap_or_else(|_| "10666".to_string());
    let api_url = format!("http://localhost:{}/health", health_port);

    let response = reqwest::get(&api_url).await.map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| e.to_string())?;
    let res_data = serde_json::from_str::<BasisHealthResponse>(&text).map_err(|e| e.to_string())?;
    
    Ok(res_data)
}

async fn handle_index(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);
    Html(IndexTemplate { i18n }.render().unwrap())
}

// HTMLテンプレートを返す既存のハンドラ (共通関数を使う形に整理)
async fn handle_health(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);

    let html_content = match fetch_health_data().await {
        Ok(res_data) => CardInnerTemplate { data: res_data, i18n }.render().unwrap(),
        Err(err_msg) => ErrorInnerTemplate { msg: err_msg, i18n }.render().unwrap(),
    };
    
    Html(html_content)
}

// 3. JSONをそのまま返す新しいハンドラ
async fn handle_health_json() -> impl IntoResponse {
    match fetch_health_data().await {
        Ok(res_data) => (StatusCode::OK, Json(res_data)).into_response(),
        Err(err_msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err_msg })),
        ).into_response(),
    }
}

// 管理画面のベース表示
async fn handle_admin(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);
    Html(AdminTemplate { i18n }.render().unwrap())
}

// プレイヤー一覧の取得・描画
async fn handle_get_players(headers: HeaderMap) -> impl IntoResponse {
    let i18n = get_locale(&headers);
    match send_api_request::<()>(reqwest::Method::GET, "/api/players", None).await {
        Ok(res) if res.status().is_success() => {
            let players = res.json::<Vec<Player>>().await.unwrap_or_default();
            Html(PlayersInnerTemplate { players, i18n }.render().unwrap()).into_response()
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch players").into_response(),
    }
}

// ワールド一覧の取得・描画
async fn handle_get_worlds(headers: HeaderMap) -> impl IntoResponse {
    let i18n = get_locale(&headers);
    match send_api_request::<()>(reqwest::Method::GET, "/api/worlds", None).await {
        Ok(res) if res.status().is_success() => {
            let worlds = res.json::<Vec<World>>().await.unwrap_or_default();
            Html(WorldsInnerTemplate { worlds, i18n }.render().unwrap()).into_response()
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch worlds").into_response(),
    }
}

// ワールドロードの実行
async fn handle_post_world_load(Json(payload): Json<LoadWorldPayload>) -> impl IntoResponse {
    match send_api_request(reqwest::Method::POST, "/api/worlds", Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ワールドのアンロード
async fn handle_delete_world(Path(id): Path<String>) -> impl IntoResponse {
    match send_api_request::<()>(reqwest::Method::DELETE, &format!("/api/worlds/{}", id), None).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// すべてのワールドをクリア
async fn handle_clear_worlds() -> impl IntoResponse {
    match send_api_request::<()>(reqwest::Method::DELETE, "/api/worlds", None).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ワールドの同期切り替え
async fn handle_switch_world(Json(payload): Json<SwitchWorldPayload>) -> impl IntoResponse {
    match send_api_request(reqwest::Method::POST, "/api/worlds/switch", Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// 全体アナウンス
async fn handle_announce_all(Json(payload): Json<AnnouncePayload>) -> impl IntoResponse {
    match send_api_request(reqwest::Method::POST, "/api/announce", Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// 特定プレイヤーへのアナウンス
async fn handle_announce_user(Path(uuid): Path<String>, Json(payload): Json<AnnouncePayload>) -> impl IntoResponse {
    match send_api_request(reqwest::Method::POST, &format!("/api/announce/{}", uuid), Some(payload)).await {
        Ok(res) if res.status().is_success() => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
