use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct PostClipboardRequest {
    pub(crate) contents: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GetClipboadResponse {
    pub(crate) contents: String,
}
