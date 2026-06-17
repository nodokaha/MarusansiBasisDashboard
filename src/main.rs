use axum::{routing::get, response::Html, Router};
use maud::{html, DOCTYPE, Markup};
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

#[tokio::main]
async fn main() {
    // ルートを2つに分ける
    let app = Router::new()
        .route("/", get(handle_index))              // 初回アクセス（土台の画面）
        .route("/api/health", get(handle_health));  // 5秒ごとの更新用API

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("🚀 Dashboard Server running on http://{}", addr);
    
    axum::serve(listener, app).await.unwrap();
}

// 1. 初回アクセス：htmxライブラリを読み込み、土台となるHTMLを返す
async fn handle_index() -> Html<String> {
    let markup = html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Basis Server Dashboard" }
                // 💡 htmx を CDN から読み込む（これだけでJS不要に！）
                script src="https://unpkg.com/htmx.org@1.9.10" {}
                style {
                    "body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; padding: 40px; color: #e2e8f0; }"
                    ".card { background: #1e293b; padding: 28px; border-radius: 12px; box-shadow: 0 10px 15px -3px rgba(0,0,0,0.3); max-width: 600px; margin: 0 auto; border: 1px solid #334155; }"
                    "h1 { color: #38bdf8; margin-top: 0; font-size: 24px; border-bottom: 1px solid #334155; padding-bottom: 12px; }"
                    ".grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 20px; }"
                    ".item { background: #111827; padding: 12px; border-radius: 6px; border: 1px solid #1e293b; }"
                    ".label { font-size: 12px; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; }"
                    ".value { font-size: 18px; font-weight: bold; margin-top: 4px; color: #f8fafc; }"
                    ".status { font-weight: bold; }"
                    ".badge { font-size: 12px; padding: 2px 6px; border-radius: 4px; background: #1e293b; border: 1px solid; margin-left: 8px; }"
                }
            }
            body {
                // 💡 ここの属性が重要！
                // hx-get: どこにリクエストを送るか
                // hx-trigger: どのタイミングで（every 5s = 5秒ごと）
                // hx-swap: 取得したHTMLをどうするか（innerHTML = このdivの中身を置き換える）
                div class="card" hx-get="/api/health" hx-trigger="load, every 5s" hx-swap="innerHTML" {
                    // 初回ロード時はローディングメッセージを表示
                    p { "サーバー情報を読み込み中..." }
                }
            }
        }
    };
    Html(markup.into_string())
}

// 2. 更新用API：basis-server からデータを取得し、「カードの内側だけ」のHTMLを返す
async fn handle_health() -> Html<String> {
    let api_url = "http://localhost:10666/health";
    let response = reqwest::get(api_url).await;

    let markup = match response {
        Ok(res) => {
            if !res.status().is_success() {
                render_error_inner(&format!("Status Error: {}", res.status()))
            } else if let Ok(text) = res.text().await {
                if let Ok(data) = serde_json::from_str::<BasisHealthResponse>(&text) {
                    render_card_inner(&data)
                } else {
                    render_error_inner("JSONパース失敗")
                }
            } else {
                render_error_inner("ボディ読み込み失敗")
            }
        }
        Err(_) => render_error_inner("basis-server への接続失敗"),
    };

    Html(markup.into_string())
}

// カードの内側だけをレンダリングする関数
fn render_card_inner(data: &BasisHealthResponse) -> Markup {
    let status_color = if data.listening { "green" } else { "red" };
    let status_text = if data.listening { "● 接続受付中 (Listening)" } else { "❌ 停止中 (Not Listening)" };
    let ready_color = if data.ready { "#38bdf8" } else { "#faf089" };
    let ready_text = if data.ready { "Ready" } else { "Not Ready" };

    html! {
        h1 { "🖥️ Basis Server Monitor" }
        
        div style="margin-top: 16px; display: flex; align-items: center;" {
            span class="label" { "ステータス: " }
            span class="status" style={"color: " (status_color) "; margin-left: 4px;"} { (status_text) }
            span class="badge" style={"color: " (ready_color) "; border-color: " (ready_color) ";"} { (ready_text) }
        }

        div class="grid" {
            div class="item" { div class="label" { "プレイヤー数 / 容量" } div class="value" { (data.visitors) " / " (data.capacity) } }
            div class="item" { div class="label" { "サーバーバージョン" } div class="value" { "v" (data.version) } }
            div class="item" { div class="label" { "送信データ量 (Sent)" } div class="value" { (data.sent) " bytes" } }
            div class="item" { div class="label" { "受信データ量 (Recv)" } div class="value" { (data.recv) " bytes" } }
        }

        div style="margin-top: 20px; font-size: 11px; color: #64748b; text-align: right;" {
            div { "起動時刻: " (data.start_time) }
            div { "現在時刻: " (data.current_time) }
        }
    }
}

fn render_error_inner(msg: &str) -> Markup {
    html! {
        h2 style="margin-top: 0; color: #fca5a5;" { "⚠️ ダッシュボードエラー" }
        p { "データ更新に失敗しました:" }
        pre style="background: #451a03; padding: 12px; border-radius: 4px; color: #fca5a5;" { (msg) }
    }
}
