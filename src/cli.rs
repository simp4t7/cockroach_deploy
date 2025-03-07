use crate::config::CockroachConfig;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use log::info;
use std::time::Instant;

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
    Clear,
}

#[tokio::main]
async fn main() -> Result<()> {
    let now = Instant::now();
    simple_logger::init_with_level(log::Level::Info)?;
    let cli = Cli::parse();

    //.unwrap() turns into default, so no panic.
    let config_path = cli.config_path.unwrap();

    let cockroach = CockroachConfig::new_from_path(&config_path)?;
    info!("config: {:#?}", &cockroach);
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
                cockroach.clear_database().await?;
                info!("total time to cockroach clear: {:#?}", &now.elapsed());
            }
        },
    }
    Ok(())
}
