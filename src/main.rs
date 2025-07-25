use std::process::ExitCode;
use futures_util::TryStreamExt;
use std::io::Write;
use tokio_util::io::ReaderStream;
use bytes::Bytes;

use clap::{Args, Parser, Subcommand};
use log::{error, info, warn};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,

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
async fn main() -> ExitCode {
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
        error!("legacy flag --trans-loopback is not supported in limeade");
        return ExitCode::from(1);
    }

    if let Some(true) = cli.compat_flags.trans_localfile {
        error!("legacy flag --trans-localfile is not supported in limeade");
        return ExitCode::from(1);
    }

    match cli.cmd {
        Command::Copy { text } => {
            let client = limeade::client::Client::new("http://localhost:2490".to_string()).unwrap();

            if let Some(text) = text {
                client.copy(&text).await.unwrap();
            } else {
                let stdin = tokio::io::stdin();
                let stdin = ReaderStream::new(stdin);
                client.copy_stream(stdin).await.unwrap();
            }
        }
        Command::Paste { .. } => {
            let client = limeade::client::Client::new("http://localhost:2490".to_string()).unwrap();
            let stream = client.paste_stream().await.unwrap();
            stream
                .try_for_each(|chunk: Bytes| async move {
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();
                    handle.write_all(&chunk).unwrap();
                    handle.flush().unwrap();
                    Ok(())
                })
                .await.unwrap();
        }
        Command::Server { mut addr } => {
            if let Some(leagacy_port) = cli.compat_flags.port {
                addr = format!(":{}", leagacy_port);
                warn!("legacy --port used, overiding addr to {addr}");
            }

            info!("starting server on {addr}");
            limeade::server::serve().await;
        }
    }
    ExitCode::SUCCESS
}
