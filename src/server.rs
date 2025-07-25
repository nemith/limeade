use arboard::Clipboard;
use axum::routing::{get, post};
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    clipboard: Arc<Mutex<Clipboard>>,
}

impl AppState {
    pub fn new() -> Result<Self, Error> {
        let clipboard = Clipboard::new()?;
        Ok(AppState {
            clipboard: Arc::new(Mutex::new(clipboard)),
        })
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to create clipboard instance: {0}")]
    ClipboardInit(#[from] arboard::Error),

    #[error("Failed to access clipboard: {0}")]
    ClipboardAccess(arboard::Error),

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Error::ClipboardAccess(arboard::Error::ClipboardOccupied) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Error::ClipboardInit(_) | Error::ClipboardAccess(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::Network(_) => StatusCode::SERVICE_UNAVAILABLE,
        };

        (status, self.to_string()).into_response()
    }
}

pub async fn serve() -> Result<(), Error> {
    let state = AppState::new()?;

    let app = Router::new()
        .route("/clipboard", post(post_clipboard))
        .route("/clipboard", get(get_clipboard))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:2490").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn post_clipboard(State(state): State<AppState>, data: String) -> Result<StatusCode, Error> {
    let mut clipboard = state.clipboard.lock().await;
    clipboard.set_text(&data).map_err(Error::ClipboardAccess)?;
    Ok(StatusCode::OK)
}

async fn get_clipboard(State(state): State<AppState>) -> Result<String, Error> {
    let mut clipboard = state.clipboard.lock().await;
    let contents = clipboard.get_text().map_err(Error::ClipboardAccess)?;
    Ok(contents)
}
