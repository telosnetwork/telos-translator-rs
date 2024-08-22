use eyre::Result;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use log::debug;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{error, info};

pub async fn ship_reader(
    mut ws_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    raw_ds_tx: mpsc::Sender<Vec<u8>>,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<()> {
    let mut counter: u64 = 0;

    loop {
        let message = tokio::select! {
            Some(message) = ws_rx.next() => message,
            _ = &mut stop_rx => break,
            else => break
        };

        counter += 1;

        let message = match message {
            Ok(message) => message,
            Err(error) => {
                error!("Error receiving message: {error}");
                break;
            }
        };

        debug!("Received message {counter}, sending to raw ds pool...",);

        if let Err(error) = raw_ds_tx.send(message.into_data()).await {
            error!("Receiver dropped: {error}");
            break;
        }

        debug!("Sent message {counter} to raw ds pool...");
    }

    info!("Exiting ship reader...");

    Ok(())
}
