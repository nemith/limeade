use futures_util::TryStreamExt;
use std::io::Write;
use tokio_util::io::ReaderStream;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use log::{info, warn};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,

    /// The server to connect to for client commands.
    #[arg(long, default_value = "localhost:2490")]
    server: String,

    /// Flags defined for lemonade that don't make sense with a proper subcommand parser.  Keep them for backwards compatability.
    #[command(flatten)]
    compat_flags: CompatFlags,
}

/// Flags for be backwards compatable with lemonade
#[derive(Args, Debug)]
struct CompatFlags {
    /// TCP Port number
    #[arg(long, global = true, hide = true)]
    port: Option<i16>,

    /// Covert Line Endings (CR/CRLF)
    #[arg(long, global = true, hide = true)]
    line_ending: Option<bool>,

    /// Allowed IP range (Server only)
    #[arg(long, global = true, hide = true)]
    allow: Vec<String>,

    /// Destination Hostname (Client only)
    #[arg(long, global = true, hide = true)]
    host: Option<String>,

    /// Do not show fallback messages (Client only)
    #[arg(long, global = true, hide = true)]
    no_fallback_messages: Option<bool>,

    /// Translate loopback address (`open` command only)
    #[arg(long, global = true, hide = true)]
    trans_loopback: Option<bool>,

    /// Translate local file path (`open` command only)
    #[arg(long, global = true, hide = true)]
    trans_localfile: Option<bool>,

    /// Log Leve (4 = Critical, 0 = Debug)
    #[arg(long, global = true, hide = true)]
    log_level: Option<u8>,
}

#[derive(Subcommand, Debug)]
enum Command {
    // Copy text
    Copy {
        text: Option<String>,
    },

    // Paste text
    Paste,

    /// Start limeade server.
    Server {
        #[arg(long, default_value = "localhost:2490")]
        addr: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let log_level = match cli.compat_flags.log_level {
        None => log::LevelFilter::Info, // TODO: Have our own verbosity level
        Some(0) => log::LevelFilter::Debug,
        Some(1) => log::LevelFilter::Info,
        Some(2) => log::LevelFilter::Warn,
        Some(_) => log::LevelFilter::Error,
    };

    env_logger::Builder::from_env("LIMEADE")
        .filter_level(log_level)
        .init();

    if let Some(true) = cli.compat_flags.trans_loopback {
        anyhow::bail!("legacy flag --trans-loopback is not supported in limeade");
    }

    if let Some(true) = cli.compat_flags.trans_localfile {
        anyhow::bail!("legacy flag --trans-localfile is not supported in limeade");
    }

    // Build server URL based on legacy flags or use --server
    let server_url = build_server_url(&cli.server, &cli.compat_flags);

    match cli.cmd {
        Command::Copy { text } => {
            let client = limeade::client::Client::new(server_url)
                .context("Failed to create client")?;

            if let Some(text) = text {
                client.copy(&text).await
                    .context("Failed to copy text to clipboard")?;
            } else {
                let stdin = tokio::io::stdin();
                let stdin = ReaderStream::new(stdin);
                client.copy_stream(stdin).await
                    .context("Failed to copy from stdin to clipboard")?;
            }
        }
        Command::Paste { .. } => {
            let client = limeade::client::Client::new(server_url)
                .context("Failed to create client")?;
            let mut stream = client.paste_stream().await
                .context("Failed to get clipboard content")?;
                
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            
            while let Some(chunk) = stream.try_next().await
                .context("Failed to read from clipboard stream")? {
                handle.write_all(&chunk)
                    .context("Failed to write to stdout")?;
                handle.flush()
                    .context("Failed to flush stdout")?;
            }
        }
        Command::Server { mut addr } => {
            if let Some(leagacy_port) = cli.compat_flags.port {
                addr = format!(":{}", leagacy_port);
                warn!("legacy --port used, overiding addr to {addr}");
            }

            info!("starting server on {addr}");
            limeade::server::serve().await
                .context("Server failed")?;
        }
    }
    Ok(())
}

fn build_server_url(server_addr: &str, compat_flags: &CompatFlags) -> String {
    match (&compat_flags.host, compat_flags.port) {
        (Some(host), Some(port)) => format!("http://{}:{}", host, port),
        (Some(host), None) => format!("http://{}:2490", host),
        (None, Some(port)) => format!("http://localhost:{}", port),
        (None, None) => {
            if server_addr.starts_with("http://") || server_addr.starts_with("https://") {
                server_addr.to_string()
            } else {
                format!("http://{}", server_addr)
            }
        }
    }
}
