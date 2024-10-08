use eyre::Result;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::debug;
use tracing::info;

pub async fn ship_reader(
    mut ws_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    raw_ds_tx: mpsc::Sender<Vec<u8>>,
    mut stop_rx: mpsc::Receiver<()>,
) -> Result<()> {
    let mut counter: u64 = 0;

    loop {
        // Read the websocket
        let message = tokio::select! {
            message = ws_rx.next() => message,
            _ = stop_rx.recv() => break
        };

        counter += 1;
        match message {
            Some(Ok(msg)) => {
                debug!("Received message {counter}, sending to raw ds pool...",);
                // write to the channel
                if raw_ds_tx.send(msg.into_data()).await.is_err() {
                    println!("Receiver dropped");
                    break;
                }
                debug!("Sent message {counter} to raw ds pool...");
            }
            Some(Err(e)) => {
                println!("Error receiving message: {}", e);
                break;
            }
            None => {
                break;
            }
        }
    }
    info!("Exiting ship reader...");
    Ok(())
}
