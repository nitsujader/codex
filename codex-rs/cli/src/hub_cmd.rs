use clap::Parser;
use codex_core::config::find_codex_home;
use codex_hub::DEFAULT_HOST;
use codex_hub::DEFAULT_PORT;
use codex_hub::DaemonOptions;
use codex_hub::StartOptions;
use codex_hub::endpoint;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct HubCli {
    #[command(subcommand)]
    pub subcommand: HubSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum HubSubcommand {
    /// Start the local Codex hub daemon.
    Start(HubStartCommand),
    /// Stop the local Codex hub daemon.
    Stop,
    /// Show status for the local Codex hub daemon.
    Status,
    /// Generate a pairing code for companion clients.
    Pair,
    /// Internal: run the hub daemon process.
    #[clap(hide = true)]
    Daemon(HubDaemonCommand),
}

#[derive(Debug, Parser)]
pub struct HubStartCommand {
    /// Loopback host/IP the hub should bind to.
    #[arg(long = "host", default_value = DEFAULT_HOST)]
    pub host: String,

    /// Local TCP port for hub control and streaming.
    #[arg(long = "port", default_value_t = DEFAULT_PORT)]
    pub port: u16,
}

#[derive(Debug, Parser)]
pub struct HubDaemonCommand {
    /// Explicit codex home path where daemon state is persisted.
    #[arg(long = "codex-home", value_name = "DIR")]
    pub codex_home: PathBuf,

    /// Host/IP to bind.
    #[arg(long = "host", default_value = DEFAULT_HOST)]
    pub host: String,

    /// Port to bind (`0` allows OS-assigned port).
    #[arg(long = "port", default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// Shared admin token used for stop/pair commands.
    #[arg(long = "auth-token", value_name = "TOKEN")]
    pub auth_token: String,
}

pub async fn run_hub_command(command: HubCli) -> anyhow::Result<()> {
    match command.subcommand {
        HubSubcommand::Start(start) => {
            let codex_home = find_codex_home()?;
            let codex_bin = std::env::current_exe()?;
            let status = codex_hub::start_daemon(StartOptions {
                codex_bin,
                codex_home,
                host: start.host.clone(),
                port: start.port,
            })
            .await?;
            let endpoint = status
                .endpoint
                .unwrap_or_else(|| endpoint(&start.host, start.port));
            println!("Codex hub started at {endpoint}");
        }
        HubSubcommand::Stop => {
            let codex_home = find_codex_home()?;
            let status = codex_hub::stop_daemon(codex_home.as_path()).await?;
            if status.running {
                if let Some(endpoint) = status.endpoint {
                    println!("Codex hub is still running at {endpoint}");
                } else {
                    println!("Codex hub is still running.");
                }
            } else {
                println!("Codex hub stopped.");
                if let Some(reason) = status.reason {
                    println!("Details: {reason}");
                }
            }
        }
        HubSubcommand::Status => {
            let codex_home = find_codex_home()?;
            let status = codex_hub::status(codex_home.as_path()).await?;
            if status.running {
                let endpoint = status
                    .endpoint
                    .unwrap_or_else(|| endpoint(DEFAULT_HOST, DEFAULT_PORT));
                println!("Codex hub is running at {endpoint}");
                if let Some(pid) = status.pid {
                    println!("PID: {pid}");
                }
                if let Some(started_at) = status.started_at {
                    println!("Started at (unix seconds): {started_at}");
                }
            } else {
                println!("Codex hub is not running.");
                if let Some(endpoint) = status.endpoint {
                    println!("Last known endpoint: {endpoint}");
                }
                if let Some(reason) = status.reason {
                    println!("Details: {reason}");
                }
            }
        }
        HubSubcommand::Pair => {
            let codex_home = find_codex_home()?;
            let pairing_code = codex_hub::start_pairing(codex_home.as_path()).await?;
            println!("Pairing code: {}", pairing_code.code);
            println!("Expires at (unix seconds): {}", pairing_code.expires_at);
        }
        HubSubcommand::Daemon(daemon) => {
            codex_hub::run_daemon(DaemonOptions {
                codex_home: daemon.codex_home,
                host: daemon.host,
                port: daemon.port,
                auth_token: daemon.auth_token,
            })
            .await?;
        }
    }

    Ok(())
}
