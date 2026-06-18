use askama::Template;
use axum::{
    routing::get,
    response::Html,
    Router,
};
use axum::http::HeaderMap;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize, Debug)]
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
    // ルートを2つに分ける
    let app = Router::new()
        .route("/", get(handle_index))              // 初回アクセス（土台の画面）
        .route("/api/health", get(handle_health));  // 5秒ごとの更新用API

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
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

async fn handle_index(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);
    Html(IndexTemplate { i18n }.render().unwrap())
}

async fn handle_health(headers: HeaderMap) -> Html<String> {
    let i18n = get_locale(&headers);
    let api_url = "http://localhost:10666/health";
    let response = reqwest::get(api_url).await;

    let html_content = match response {
        Ok(res) => {
            if !res.status().is_success() {
                let err_msg = format!("Status: {}", res.status());
                ErrorInnerTemplate { msg: err_msg, i18n }.render().unwrap()
            } else {
                match res.text().await {
                    Ok(text) => {
                        match serde_json::from_str::<BasisHealthResponse>(&text) {
                            Ok(res_data) => {
                                CardInnerTemplate { data: res_data, i18n }.render().unwrap()
                            }
                            Err(err) => {
                                ErrorInnerTemplate { msg: err.to_string(), i18n }.render().unwrap()
                            }
                        }
                    }
                    Err(err) => {
                        ErrorInnerTemplate { msg: err.to_string(), i18n }.render().unwrap()
                    }
                }
            }
        }
        Err(err) => {
            ErrorInnerTemplate { msg: err.to_string(), i18n }.render().unwrap()
        }
    };
    
    Html(html_content)
}
