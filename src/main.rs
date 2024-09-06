use std::fs;

use clap::Parser;
use telos_translator_rs::translator::{Translator, TranslatorConfig};
use log::{error, LevelFilter};
use env_logger::Builder;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config_contents = fs::read_to_string(args.config).expect("Could not read config file");
    let config: TranslatorConfig =
        toml::from_str(&config_contents).expect("Could not parse config as toml");

    let log_level = parse_log_level(&config.log_level).unwrap();
    let mut builder = Builder::from_default_env();
    builder.filter_level(log_level);
    builder.init();

    if let Err(e) = Translator::new(config).launch(None).await {
        error!("Failed to launch translator: {e:?}");
    }
}

fn parse_log_level(s: &str) -> Result<LevelFilter, String> {
    match s.to_lowercase().as_str() {
        "off" => Ok(LevelFilter::Off),
        "error" => Ok(LevelFilter::Error),
        "warn" => Ok(LevelFilter::Warn),
        "info" => Ok(LevelFilter::Info),
        "debug" => Ok(LevelFilter::Debug),
        "trace" => Ok(LevelFilter::Trace),
        _ => Err(format!("Unknown log level: {}", s)),
    }
}
