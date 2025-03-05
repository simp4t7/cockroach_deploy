use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use cockroach_deploy::cockroach::CockroachResources;
use log::info;
use scopeguard::defer;
use simple_logger::SimpleLogger;
use std::time::Instant;
use tokio::sync::watch::channel;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, default_value = "cockroach_config.toml")]
    config_path: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Cockroach { command: CockroachCommand },
}

#[derive(Debug, Clone, ValueEnum)]
enum CockroachCommand {
    Nuke,
    Init,
    Test,
    Clear,
}

#[tokio::main]
async fn main() -> Result<()> {
    let now = Instant::now();
    SimpleLogger::new().init()?;
    let cli = Cli::parse();
    let config_path = cli.config_path.unwrap();

    let cockroach = CockroachResources::new(&config_path)?;
    info!("config: {:#?}", &cockroach.config);
    match &cli.command {
        Commands::Cockroach { command } => match command {
            CockroachCommand::Init => {
                cockroach.init_cockroach().await?;
                info!("total time to cockroach init: {:#?}", &now.elapsed());
            }
            CockroachCommand::Nuke => {
                cockroach.delete_cockroach().await?;
                info!("total time to cockroach nuke: {:#?}", &now.elapsed());
            }
            CockroachCommand::Clear => {
                let (tx, rx) = channel(false);
                let c = cockroach.clone();
                tokio::task::spawn(async move {
                    c.cockroach_port_forward(rx)
                        .await
                        .expect("cockroach_port_forward error");
                });

                defer!(tx.send(true).expect("send err"););
                cockroach.refresh_database().await?;
                info!("total time to cockroach clear: {:#?}", &now.elapsed());
            }
            CockroachCommand::Test => {}
        },
    }
    Ok(())
}
