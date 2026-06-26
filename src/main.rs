mod agent;
mod archive;
mod cli;
mod config;
mod ids;
mod protocol;
mod server;
mod storage;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli::CliAction::Run(options) = cli::parse()? else {
        print!("{}", cli::usage());
        return Ok(());
    };

    let config =
        config::AppConfig::load(options.config_path.as_deref()).context("load configuration")?;
    init_tracing(&config.core.log_level)?;

    match config.core.role {
        config::Role::Server => {
            let server = config
                .server
                .clone()
                .context("missing [server] configuration")?;
            server::run(config.core, server, config.server_agents).await
        }
        config::Role::Agent => {
            let agent = config
                .agent
                .clone()
                .context("missing [agent] configuration")?;
            agent::run(config.core, agent).await
        }
    }
}

fn init_tracing(level: &str) -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_new(level)
        .or_else(|_| tracing_subscriber::EnvFilter::try_new("info"))?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    Ok(())
}
