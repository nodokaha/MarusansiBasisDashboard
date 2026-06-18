use axum::{routing::get, response::Html, Router};
use askama::Template;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BasisHealthResponse {
    listening: bool,
    ready: bool,
    visitors: u32,
    capacity: u32,
    sent: u64,
    recv: u64,
    current_time: String,
    start_time: String,
    version: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

#[derive(Template)]
#[template(path = "card_inner.html")]
struct CardInnerTemplate {
    data: BasisHealthResponse, 
}

#[derive(Template)]
#[template(path = "error_inner.html")]
struct ErrorInnerTemplate {
    msg: String,
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

async fn handle_index() -> Html<String> {
    let tpl = IndexTemplate;
    // index.html と合体させてブラウザに返す
    Html(tpl.render().unwrap())
}

async fn handle_health() -> Html<String> {
let api_url = "http://localhost:10666/health";
    let response = reqwest::get(api_url).await;

    let html_content = match response {
        Ok(res) => {
            if !res.status().is_success() {
                let tpl = ErrorInnerTemplate { msg: format!("Status Error: {}", res.status()) };
                tpl.render().unwrap()
            } else if let Ok(text) = res.text().await {
                if let Ok(res_data) = serde_json::from_str::<BasisHealthResponse>(&text) {
                    let tpl = CardInnerTemplate { data: res_data };
                    tpl.render().unwrap()
                } else {
                    let tpl = ErrorInnerTemplate { msg: "JSONパース失敗".to_string() };
                    tpl.render().unwrap()
                }
            } else {
                let tpl = ErrorInnerTemplate { msg: "ボディ読み込み失敗".to_string() };
                tpl.render().unwrap()
            }
        }
        Err(_) => {
            let tpl = ErrorInnerTemplate { msg: "basis-server への接続失敗".to_string() };
            tpl.render().unwrap()
        }
    };

    Html(html_content)
}
