use std::sync::Arc;

use crate::block::{ProcessingEVMBlock, TelosEVMBlock};
use crate::data::Database;
use crate::tasks::{evm_block_processor, final_processor, raw_deserializer, ship_reader};
use antelope::api::client::APIClient;
use antelope::api::default_provider::DefaultProvider;
use eyre::{eyre, Context, Result};
use futures_util::future::join_all;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::connect_async;
use tracing::info;

pub const DEFAULT_BLOCK_PROCESS_THREADS: u8 = 4;

pub const DEFAULT_RAW_MESSAGE_CHANNEL_SIZE: usize = 10000;
pub const DEFAULT_BLOCK_PROCESS_CHANNEL_SIZE: usize = 1000;
pub const DEFAULT_MESSAGE_FINALIZER_CHANNEL_SIZE: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatorConfig {
    pub chain_id: u64,
    pub start_block: u32,
    pub stop_block: Option<u32>,
    pub block_delta: u32,
    pub prev_hash: String,
    pub validate_hash: Option<String>,

    pub http_endpoint: String,
    pub ship_endpoint: String,

    pub raw_ds_threads: Option<u8>,
    pub block_process_threads: Option<u8>,

    pub raw_message_channel_size: Option<usize>,
    pub block_message_channel_size: Option<usize>,
    pub final_message_channel_size: Option<usize>,

    pub data_path: String,
}

pub struct Translator {
    config: TranslatorConfig,
}

impl Translator {
    pub fn new(config: TranslatorConfig) -> Self {
        Self { config }
    }

    pub async fn launch(&mut self, output_tx: Option<mpsc::Sender<TelosEVMBlock>>) -> Result<()> {
        let api_client =
            APIClient::<DefaultProvider>::default_provider(self.config.http_endpoint.clone())
                .map_err(|error| eyre!(error))
                .wrap_err("Failed to create API client")?;

        let (ws_stream, _) = connect_async(&self.config.ship_endpoint)
            .await
            .map_err(|_| {
                eyre!(
                    "Failed to connect to ship at endpoint {}",
                    &self.config.ship_endpoint
                )
            })?;

        let (ws_tx, ws_rx) = ws_stream.split();

        let db = Database::open(&self.config.data_path)?;
        let chain = Arc::new(Mutex::new(db.get_chain()?.unwrap_or_default()));
        // Buffer size here should be the readahead buffer size, in blocks.  This could get large if we are reading
        //  a block range with larges blocks/trxs, so this should be tuned based on the largest blocks we hit
        let (raw_ds_tx, raw_ds_rx) = mpsc::channel::<Vec<u8>>(
            self.config
                .raw_message_channel_size
                .unwrap_or(DEFAULT_RAW_MESSAGE_CHANNEL_SIZE),
        );

        let (process_tx, process_rx) = mpsc::channel::<ProcessingEVMBlock>(
            self.config
                .block_message_channel_size
                .unwrap_or(DEFAULT_BLOCK_PROCESS_CHANNEL_SIZE),
        );

        let (finalize_tx, finalize_rx) = mpsc::channel::<ProcessingEVMBlock>(
            self.config
                .final_message_channel_size
                .unwrap_or(DEFAULT_MESSAGE_FINALIZER_CHANNEL_SIZE),
        );

        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        // Start the final processing task
        let final_processor_handle = tokio::spawn(final_processor(
            self.config.clone(),
            api_client,
            finalize_rx,
            output_tx,
            stop_tx,
            chain.clone(),
            db.clone(),
        ));

        let evm_block_processor_handle = tokio::spawn(evm_block_processor(process_rx, finalize_tx));

        let raw_deserializer_handle = tokio::spawn(raw_deserializer(
            self.config.clone(),
            raw_ds_rx,
            ws_tx,
            process_tx,
            chain,
            db,
        ));

        let ship_reader_handle = tokio::spawn(ship_reader(ws_rx, raw_ds_tx, stop_rx));

        info!("Translator launched successfully");
        let result = join_all(vec![
            ship_reader_handle,
            raw_deserializer_handle,
            evm_block_processor_handle,
            final_processor_handle,
        ])
        .await;

        result
            .into_iter()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map(|_| ())
            .wrap_err("Failed to execute tasks")
    }
}
