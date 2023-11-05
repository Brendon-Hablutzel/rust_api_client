use api_client::{from_file, interactive};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive api client
    Interactive {
        #[arg(short, long)]
        log_file: Option<String>,
    },
    /// Send API requests from JSON file
    FromFile {
        /// The JSON file with the requests to send
        file: String,
        #[arg(short, long)]
        /// Whether the program should stop sending requests early if one errors
        stop_early_on_fail: bool,
        #[arg(short, long)]
        /// An optional file to log reuqest and response history to
        log_file: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Interactive { log_file } => interactive(log_file),
        Commands::FromFile {
            file,
            stop_early_on_fail,
            log_file,
        } => {
            match from_file(file, *stop_early_on_fail, log_file) {
                Err(err) => println!("{err}"),
                _ => (),
            };
        }
    }
}
