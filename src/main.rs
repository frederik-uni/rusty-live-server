#[cfg(feature = "binary")]
use std::path::PathBuf;

#[cfg(feature = "binary")]
use clap::{Parser, Subcommand};

#[cfg(feature = "binary")]
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "PORT", default_value_t = 8080)]
    port: u16,
    #[command(subcommand)]
    command: Commands,
}
#[cfg(feature = "binary")]
#[derive(Subcommand)]
enum Commands {
    /// PATH
    Serve { path: PathBuf },
}

#[cfg(feature = "binary")]
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let Commands::Serve { path } = cli.command;
    serve_dir::serve(path, cli.port, true, None).await.unwrap();
}

#[cfg(not(feature = "binary"))]
fn main() {
    println!("Binary Feature required")
}
