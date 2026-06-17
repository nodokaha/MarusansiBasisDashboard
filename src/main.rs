use axum::{routing::get, response::Html, Router};
use maud::{html, DOCTYPE};
use serde::Deserialize;
use std::net::SocketAddr;

// 1. APIから返ってくるJSONの構造体（型安全にパース）
#[derive(Deserialize, Debug)]
struct ApiResponse {
    slideshow: Slideshow,
}

#[derive(Deserialize, Debug)]
struct Slideshow {
    author: String,
    title: String,
}

#[tokio::main]
async fn main() {
    // ルーティングの設定
    let app = Router::new().route("/", get(handle_server_info));

    // サーバーの起動 (ポート 3000)
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("🚀 Server running on http://{}", addr);
    
    axum::serve(listener, app).await.unwrap();
}

// 2. リクエストを受け取るハンドラ
async fn handle_server_info() -> Html<String> {
    // 3. 外部のJSON APIを叩く (reqwest)
    let api_url = "http://basis-server:10666/health";
    let response = reqwest::get(api_url).await;

    // エラーハンドリングもRustらしく綺麗に
    let page_content = match response {
        Ok(res) => {
            if let Ok(data) = res.json::<ApiResponse>().await {
                // API取得成功時のHTML生成
                render_html(&data.slideshow.title, &data.slideshow.author)
            } else {
                render_error("JSONのパースに失敗しました")
            }
        }
        Err(_) => render_error("APIの取得に失敗しました"),
    };

    Html(page_content)
}

// 4. Maudマクロによる超高速・省メモリなHTMLレンダリング
fn render_html(title: &str, author: &str) -> String {
    let markup = html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "サーバー情報ダッシュボード" }
                style {
                    "body { font-family: sans-serif; background: #f4f7f6; padding: 40px; color: #333; }"
                    ".card { background: white; padding: 24px; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); max-width: 500px; }"
                    "h1 { color: #0076ff; margin-top: 0; }"
                }
            }
            body {
                div class="card" {
                    h1 { "🖥️ サーバー監視状況" }
                    p { strong { "対象システム: " } (title) }
                    p { strong { "管理者: " } (author) }
                    p { "ステータス: " span style="color: green;" { "● 正常稼働中" } }
                }
            }
        }
    };
    markup.into_string()
}

fn render_error(msg: &str) -> String {
    html! {
        (DOCTYPE)
        html {
            body {
                h1 style="color: red;" { "エラーが発生しました" }
                p { (msg) }
            }
        }
    }.into_string()
}
