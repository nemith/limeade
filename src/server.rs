use arboard::Clipboard; 
use axum::{Router};
use axum::routing::{post, get};

pub async fn serve() {
    let app = Router::new().
        route("/clipboard", post(post_clipboard)).
        route("/clipboard", get(get_clipboard));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:2490").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


async fn post_clipboard(data: String) {
    let mut clipboard = Clipboard::new().unwrap();
    clipboard.set_text(&data).unwrap();
}


async fn get_clipboard() -> String {
    // TODO: handle errors
    let mut clipboard = Clipboard::new().unwrap();
    let contents = clipboard.get_text().unwrap();
    contents
}

