use crate::block::Block;
use crate::translator::{write_message, TranslatorConfig};
use crate::types::ship_types::ShipRequest::{GetBlocksAck, GetStatus};
use crate::types::ship_types::{
    GetBlocksAckRequestV0, GetBlocksRequestV0, GetStatusRequestV0, ShipRequest, ShipResult,
};
use crate::types::types::{BlockOrSkip, RawMessage, WebsocketReceiver, WebsocketTransmitter};
use antelope::chain::Decoder;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info};

pub async fn raw_deserializer(
    thread_id: u8,
    config: TranslatorConfig,
    mut raw_ds_rx: Arc<Mutex<Receiver<RawMessage>>>,
    mut ws_tx: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Message>>>,
    block_deserializer_tx: Sender<Block>,
    orderer_tx: Sender<BlockOrSkip>,
) {
    let mut ship_abi_received = false;
    let mut latest_status = None;
    let mut unackd_blocks = 0;
    let mut last_log = Instant::now();
    let mut unlogged_blocks = 0;

    loop {
        debug!("raw deserializer #{} getting next message...", thread_id);
        let raw = {
            let mut rx = raw_ds_rx.lock().await;
            rx.recv().await
        };
        debug!("raw deserializer #{} got message, decoding...", thread_id);
        if let Some(msg) = raw {
            // ABI is always the first message sent on connect
            if !ship_abi_received {
                // TODO: maybe get this working as an ABI again?
                //   the problem is that the ABI from ship has invalid table names like `account_metadata`
                //   which cause from_string to fail, but if you change AbiTable.name to a String then
                //   when you use the ABI struct to pack for a contract deployment, it causes the table
                //   lookups via v1/chain/get_table_rows to fail because it doesn't like the string when
                //   it's trying to determine the index type of a table
                //let abi_string = msg.to_string();
                //let abi = ABI::from_string(abi_string.as_str()).unwrap();
                //self.ship_abi = Some(abi_string);

                ship_abi_received = true;

                // Send GetStatus request after setting up the ABI
                let request = GetStatus(GetStatusRequestV0);
                write_message(ws_tx.clone(), &request).await;
                orderer_tx.send(BlockOrSkip::Skip(msg.sequence)).await.unwrap();
            } else {
                // Print received messages after ABI is set
                //info!("Received message: {:?}", bytes_to_hex(&msg_data));
                // TODO: Better threading so we don't block reading while deserialize?
                let mut decoder = Decoder::new(msg.bytes.as_slice());
                let ship_result = &mut ShipResult::default();
                decoder.unpack(ship_result);

                match ship_result {
                    ShipResult::GetStatusResultV0(r) => {
                        info!(
                            "GetStatusResultV0 head: {:?} last_irreversible: {:?}",
                            r.head.block_num, r.last_irreversible.block_num
                        );
                        latest_status = Some(Arc::new(ShipResult::GetStatusResultV0(r.clone())));
                        write_message(
                            ws_tx.clone(),
                            &ShipRequest::GetBlocks(GetBlocksRequestV0 {
                                start_block_num: config.start_block,
                                end_block_num: config.stop_block.unwrap_or(u32::MAX),
                                max_messages_in_flight: 10000,
                                have_positions: vec![],
                                irreversible_only: true, // TODO: Fork handling
                                fetch_block: true,
                                fetch_traces: true,
                                fetch_deltas: true,
                            }),
                        )
                        .await;
                        orderer_tx.send(BlockOrSkip::Skip(msg.sequence)).await.unwrap();
                    }
                    ShipResult::GetBlocksResultV0(r) => {
                        unackd_blocks += 1;
                        if let Some(b) = &r.this_block {
                            let block = Block::new(
                                config.chain_id,
                                msg.sequence,
                                b.block_num,
                                b.block_id,
                                r.clone(),
                            );
                            debug!("Block #{} sending to block deserializer...", b.block_num);
                            block_deserializer_tx.send(block).await.unwrap();
                            debug!("Block #{} sent to block deserializer", b.block_num);
                            if last_log.elapsed().as_secs_f64() > 10.0 {
                                info!(
                                    "Raw deserializer thread #{} block #{} - processed {} blocks/sec",
                                    thread_id,
                                    b.block_num,
                                    (unlogged_blocks + unackd_blocks) as f64
                                        / last_log.elapsed().as_secs_f64()
                                );
                                unlogged_blocks = 0;
                                last_log = Instant::now();
                            }

                            // TODO: Better logic here, don't just ack every N blocks, do this based on backpressure
                            if unackd_blocks > 10 {
                                //info!("Acking {} blocks", unackd_blocks);
                                // TODO: Better threading so we don't block reading while we write?
                                write_message(
                                    ws_tx.clone(),
                                    &GetBlocksAck(GetBlocksAckRequestV0 {
                                        num_messages: unackd_blocks,
                                    }),
                                )
                                .await;
                                //info!("Blocks acked");
                                unlogged_blocks += unackd_blocks;
                                unackd_blocks = 0;
                            }
                        } else {
                            // TODO: why would this happen?
                            error!("GetBlocksResultV0 without a block");
                        }
                    }
                }
            }
        }
    }
}
