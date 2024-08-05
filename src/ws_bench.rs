use antelope::chain::Decoder;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::fs;
use telos_translator_rs::{
    translator::TranslatorConfig,
    types::ship_types::{
        GetBlocksAckRequestV0, GetBlocksRequestV0, GetStatusRequestV0, ShipRequest::*, ShipResult,
    },
};
use tokio::{sync::mpsc, time::Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() {
    // NOTE: cargo run --release --bin ws-bench
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config: TranslatorConfig = fs::read_to_string(args.config)
        .map(|value| toml::from_str(&value).unwrap())
        .unwrap();

    let connect_result = connect_async(&config.ship_endpoint).await;
    let (mut ws_tx, mut ws_rx) = connect_result.unwrap().0.split();
    let (ds_tx, mut ds_rx) =
        mpsc::channel::<(u64, Message)>(config.raw_message_channel_size.unwrap_or(100000));

    tokio::spawn(async move {
        let mut sequence: u64 = 0;
        let mut now = Instant::now();

        while let Some(Ok(message)) = ws_rx.next().await {
            if sequence > 0 && sequence % 10000 == 0 {
                let elapsed = now.elapsed().as_millis() as f64;
                let per_second = 1000.0 * 10000.0 / elapsed;
                info!("Processing ws messages, {per_second:.2}/s",);
                now = Instant::now();
            }
            sequence += 1;
            ds_tx.send((sequence, message)).await.unwrap();
        }
    });

    let mut num_messages = 0;

    ds_rx.recv().await.unwrap();

    let message = &GetStatus(GetStatusRequestV0);
    ws_tx.send(message.into()).await.unwrap();

    while let Some((_, message)) = ds_rx.recv().await {
        let ship_result = &mut ShipResult::default();
        Decoder::new(message.into_data().as_slice()).unpack(ship_result);

        match ship_result {
            ShipResult::GetStatusResultV0(_) => {
                let request = &GetBlocks(GetBlocksRequestV0 {
                    start_block_num: config.start_block,
                    end_block_num: config.stop_block.unwrap_or(u32::MAX),
                    max_messages_in_flight: 10000,
                    have_positions: vec![],
                    irreversible_only: true,
                    fetch_block: true,
                    fetch_traces: true,
                    fetch_deltas: true,
                });
                ws_tx.send(request.into()).await.unwrap();
            }
            ShipResult::GetBlocksResultV0(result) => {
                result.this_block.as_ref().unwrap();
                if num_messages == 10 {
                    let request = &GetBlocksAck(GetBlocksAckRequestV0 { num_messages });
                    ws_tx.send(request.into()).await.unwrap();
                    num_messages = 0;
                } else {
                    num_messages += 1;
                }
            }
        }
    }
}
