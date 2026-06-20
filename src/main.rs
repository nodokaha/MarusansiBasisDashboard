use askama::Template;
use axum::{
    routing::get,
    response::{Html, IntoResponse},
    Json,
    Router,
};
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

#[tokio::main]
async fn main() {
    // 1. サーバー起動ポートを環境変数から取得 (デフォルト: 4000)
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "4000".to_string())
        .parse()
        .expect("PORT must be a valid number");

    // ルートの追加
    let app = Router::new()
        .route("/", get(handle_index))               // 初回アクセス（土台の画面）
        .route("/api/health", get(handle_health))    // 5秒ごとの更新用API (HTML)
        .route("/api/health/json", get(handle_health_json)); // JSONをそのまま返すAPI

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
