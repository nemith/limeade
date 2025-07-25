use bytes::Bytes;
use futures_util::{Stream, StreamExt, TryStream};
use std::time::Duration;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("transport error")]
    Transport(#[from] reqwest::Error),
    #[error("invalid addr: {0}")]
    Url(#[from] url::ParseError),
}

type Result<T> = std::result::Result<T, Error>;

pub struct Client {
    client: reqwest::Client,
    base_url: Url,
}

impl Client {
    pub fn new(url: String) -> Result<Self> {
        let url = if url.starts_with("http://") || url.starts_with("https://") {
            url
        } else {
            format!("http://{}", url)
        };

        let client = reqwest::Client::builder()
            .user_agent("limeade/1.0")
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: Url::parse(&url)?,
        })
    }

    fn clipboard_url(&self) -> Url {
        self.base_url
            .join("/clipboard")
            .expect("couldn't create clipboard url")
    }

    pub async fn copy(&self, text: &str) -> Result<()> {
        let url = self.clipboard_url();
        self.client
            .post(url)
            .header("Content-Type", "application/octet-stream")
            .body(text.to_string())
            .send()
            .await?;
        Ok(())
    }

    pub async fn copy_stream<S>(&self, stream: S) -> Result<()>
    where
        S: TryStream + Send + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        Bytes: From<S::Ok>,
    {
        let url = self.clipboard_url();
        let body = reqwest::Body::wrap_stream(stream);
        self.client
            .post(url)
            .header("Content-Type", "application/octet-stream")
            .body(body)
            .send()
            .await?;

        Ok(())
    }

    pub async fn paste(&self) -> Result<Bytes> {
        let url = self.clipboard_url();
        Ok(self.client.get(url).send().await?.bytes().await?)
    }

    pub async fn paste_stream(&self) -> Result<impl Stream<Item = Result<Bytes>>> {
        let url = self.clipboard_url();
        let response = self.client.get(url).send().await?;

        let stream = response
            .bytes_stream()
            .map(|result| result.map_err(Error::from));

        Ok(stream)
    }
}
