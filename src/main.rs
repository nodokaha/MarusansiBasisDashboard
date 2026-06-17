use axum::{routing::get, response::Html, Router};
use maud::{html, DOCTYPE};
use serde::Deserialize;
use std::net::SocketAddr;

// basis-server の実際のレスポンスに合わせた構造体
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")] // currentTime などのキャメルケースに対応
struct BasisHealthResponse {
    listening: bool,
    visitors: String,
    capacity: String,
    sent: String,
    recv: String,
    current_time: String,
    start_time: String,
    version: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handle_server_info));

    // コンテナ外部（ホスト側）からアクセスできるように 0.0.0.0 にバインド
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("🚀 Dashboard Server running on http://{}", addr);
    
    axum::serve(listener, app).await.unwrap();
}

async fn handle_server_info() -> Html<String> {
    // Docker Compose内のネットワーク名（サービス名）を指定
    let api_url = "http://host.docker.internal:10666/health";
    
    println!("🔄 Fetching health data from: {}", api_url);
    let response = reqwest::get(api_url).await;

    let page_content = match response {
        Ok(res) => {
            // ステータスコードが 200 OK かチェック
            if !res.status().is_success() {
                let status_err = format!("サーバーがエラーを返しました (Status: {})", res.status());
                eprintln!("❌ [Error] {}", status_err);
                render_error(&status_err)
            } else {
                // レスポンスのテキストを一旦取得（パース失敗時のログ用）
                match res.text().await {
                    Ok(text) => {
                        // JSONをパース
                        match serde_json::from_str::<BasisHealthResponse>(&text) {
                            Ok(data) => {
                                println!("✅ Successfully fetched and parsed server info.");
                                render_html(&data)
                            }
                            Err(err) => {
                                // JSONのパースエラーログを詳細に出力
                                let parse_err = format!("JSONパース失敗: {}. 生データ: {}", err, text);
                                eprintln!("❌ [Error] {}", parse_err);
                                render_error(&parse_err)
                            }
                        }
                    }
                    Err(err) => {
                        let text_err = format!("レスポンスボディの読み込み失敗: {}", err);
                        eprintln!("❌ [Error] {}", text_err);
                        render_error(&text_err)
                    }
                }
            }
        }
        Err(err) => {
            // 通信自体が失敗した場合のエラー（Connection Refusedなど）
            let conn_err = format!("basis-server への接続に失敗しました: {}", err);
            eprintln!("❌ [Error] {}", conn_err);
            render_error(&conn_err)
        }
    };

    Html(page_content)
}

// 実際のデータを表示するHTMLテンプレート
fn render_html(data: &BasisHealthResponse) -> String {
    let status_color = if data.listening { "green" } else { "red" };
    let status_text = if data.listening { "● 接続受付中 (Listening)" } else { "❌ 停止中 (Not Listening)" };

    let markup = html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Basis Server Dashboard" }
                style {
                    "body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; padding: 40px; color: #e2e8f0; }"
                    ".card { background: #1e293b; padding: 28px; border-radius: 12px; box-shadow: 0 10px 15px -3px rgba(0,0,0,0.3); max-width: 600px; margin: 0 auto; border: 1px solid #334155; }"
                    "h1 { color: #38bdf8; margin-top: 0; font-size: 24px; border-bottom: 1px solid #334155; padding-bottom: 12px; }"
                    ".grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 20px; }"
                    ".item { background: #111827; padding: 12px; border-radius: 6px; border: 1px solid #1e293b; }"
                    ".label { font-size: 12px; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; }"
                    ".value { font-size: 18px; font-weight: bold; margin-top: 4px; color: #f8fafc; }"
                    ".status { font-weight: bold; color: " (status_color) "; }"
                }
            }
            body {
                div class="card" {
                    h1 { "🖥️ Basis Server Monitor" }
                    
                    div style="margin-top: 16px;" {
                        span class="label" { "ステータス: " }
                        span class="status" { (status_text) }
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
        }
    };
    markup.into_string()
}

// エラー発生時のHTMLテンプレート
fn render_error(msg: &str) -> String {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Dashboard Error" }
                style {
                    "body { font-family: sans-serif; background: #0f172a; padding: 40px; color: #f8fafc; }"
                    ".error-card { background: #7f1d1d; border: 1px solid #f87171; padding: 24px; border-radius: 8px; max-width: 600px; margin: 0 auto; }"
                    "pre { background: #451a03; padding: 12px; border-radius: 4px; overflow-x: auto; color: #fca5a5; border: 1px solid #78350f; font-family: monospace; white-space: pre-wrap; }"
                }
            }
            body {
                div class="error-card" {
                    h2 style="margin-top: 0; color: #fca5a5;" { "⚠️ ダッシュボードエラー" }
                    p { "basis-server からの情報取得中に以下のエラーが発生しました:" }
                    pre { (msg) }
                    p style="font-size: 12px; color: #fca5a5;" { "※詳細なログはコンテナの標準出力（docker compose logs）を確認してください。" }
                }
            }
        }
    }.into_string()
}
