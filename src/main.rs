use std::fs;

use clap::Parser;
use telos_translator_rs::translator::{Translator, TranslatorConfig};
use tracing::error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the translator configuration file
    #[arg(long, default_value = "config.toml")]
    config: String,
    /// Start translator from clean state
    #[arg(long, default_value = "false")]
    clean: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config_contents = fs::read_to_string(args.config).expect("Could not read config file");
    let config: TranslatorConfig =
        toml::from_str(&config_contents).expect("Could not parse config as toml");

    if let Err(e) = Translator::new(config).launch(None, args.clean).await {
        error!("Failed to launch translator: {e:?}");
    }
}
