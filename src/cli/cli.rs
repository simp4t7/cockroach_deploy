use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use cockroach_deploy::cockroach::delete_cockroach_db;
use cockroach_deploy::cockroach::initialize_cockroach_db;
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    namespace: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Cockroach { command: CockroachCommand },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CockroachCommand {
    Nuke,
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    let now = Instant::now();
    let cli = Cli::parse();
    match &cli.command {
        Commands::Cockroach { command } => match command {
            CockroachCommand::Init => {
                initialize_cockroach_db(cli.namespace).await?;
                println!("total time to cockroach init: {:#?}", &now.elapsed());
            }
            CockroachCommand::Nuke => {
                delete_cockroach_db(cli.namespace).await?;
                println!("total time to cockroach nuke: {:#?}", &now.elapsed());
            }
        },
    }
    Ok(())
}
