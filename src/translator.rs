use crate::block::Block;
use crate::tasks::{
    evm_block_processor, final_processor, order_preserving_queue, raw_deserializer, ship_reader,
};
use crate::types::translator_types::{BlockOrSkip, RawMessage};
use alloy::primitives::FixedBytes;
use antelope::api::client::APIClient;
use antelope::api::default_provider::DefaultProvider;
use eyre::{eyre, Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tracing::info;

pub const DEFAULT_BLOCK_PROCESS_THREADS: u8 = 4;

pub const DEFAULT_RAW_MESSAGE_CHANNEL_SIZE: usize = 10000;
pub const DEFAULT_BLOCK_PROCESS_CHANNEL_SIZE: usize = 1000;
pub const DEFAULT_MESSAGE_ORDERER_CHANNEL_SIZE: usize = 1000;
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
    pub order_message_channel_size: Option<usize>,
    pub final_message_channel_size: Option<usize>,
}

pub struct Translator {
    config: TranslatorConfig,
}

impl Translator {
    pub fn new(config: TranslatorConfig) -> Self {
        Self { config }
    }

    pub async fn launch(
        &mut self,
        output_tx: Option<mpsc::Sender<(FixedBytes<32>, Block)>>,
    ) -> Result<()> {
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

        // Buffer size here should be the readahead buffer size, in blocks.  This could get large if we are reading
        //  a block range with larges blocks/trxs, so this should be tuned based on the largest blocks we hit
        let (raw_ds_tx, raw_ds_rx) = mpsc::channel::<RawMessage>(
            self.config
                .raw_message_channel_size
                .unwrap_or(DEFAULT_RAW_MESSAGE_CHANNEL_SIZE),
        );

        let (process_tx, process_rx) = mpsc::channel::<Block>(
            self.config
                .block_message_channel_size
                .unwrap_or(DEFAULT_BLOCK_PROCESS_CHANNEL_SIZE),
        );

        let (order_tx, order_rx) = mpsc::channel::<BlockOrSkip>(
            self.config
                .order_message_channel_size
                .unwrap_or(DEFAULT_MESSAGE_ORDERER_CHANNEL_SIZE),
        );

        let (finalize_tx, finalize_rx) = mpsc::channel::<Block>(
            self.config
                .final_message_channel_size
                .unwrap_or(DEFAULT_MESSAGE_FINALIZER_CHANNEL_SIZE),
        );

        let stop_at = self
            .config
            .stop_block
            .map(|stop| stop - self.config.start_block)
            .map(Into::into);

        let ship_reader_handle = tokio::spawn(ship_reader(ws_rx, raw_ds_tx, stop_at));

        tokio::spawn(raw_deserializer(
            0,
            self.config.clone(),
            raw_ds_rx,
            ws_tx,
            process_tx.clone(),
            order_tx.clone(),
        ));

        tokio::spawn(evm_block_processor(process_rx, order_tx.clone()));

        // Start the order-preserving queue task
        tokio::spawn(order_preserving_queue(order_rx, finalize_tx));

        // Start the final processing task
        tokio::spawn(final_processor(
            self.config.clone(),
            api_client,
            finalize_rx,
            output_tx,
        ));

        info!("Translator launched successfully");
        ship_reader_handle.await?;
        Ok(())
    }
}
