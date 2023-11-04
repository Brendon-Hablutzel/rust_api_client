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
    Interactive,
    /// Send API requests from JSON file
    FromFile {
        /// The JSON file with the requests to send
        file: String,
        #[arg(short, long)]
        /// Whether the program should stop sending requests early if one errors
        stop_early_on_fail: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Interactive => interactive(),
        Commands::FromFile {
            file,
            stop_early_on_fail,
        } => {
            match from_file(file, *stop_early_on_fail) {
                Err(err) => println!("{err}"),
                _ => (),
            };
        }
    }
}
