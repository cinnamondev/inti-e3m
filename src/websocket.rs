use futures_util::StreamExt;
use futures_util::SinkExt;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes, WebSocket};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_util::sync::CancellationToken;
use crate::{tcode_de, Command};
use crate::tcode_de::LinearActionError;
use crate::tui::config::{ServiceProvider, WebsocketConfig};

#[derive(Debug,Error)]
pub enum ClientError {
    #[error(transparent)]
    Tungstenite(#[from] tungstenite::error::Error),
    #[error(transparent)]
    LinearAction(#[from] LinearActionError),
    #[error(transparent)]
    Mspc(#[from] SendError<Command>)
}

async fn extoys(websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, tx: Sender<Command>) -> Result<(), ClientError>{
    while let Some(msg) = websocket.next().await {
        let msg = msg?;
        log::debug!("websocket received message: {:?}", msg);
    }
    Ok(())
}

async fn intiface(websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, tx: Sender<Command>) -> Result<(), ClientError> {
    websocket.send(Message::Text(Utf8Bytes::from(
        format!("{{\"identifier\":\"{0}\",\"address\":\"{1}\",\"version\":0}}", "UpYourEnder", 2))
    )).await?;

    while let Some(msg) = websocket.next().await {
        let msg = msg?;
        log::debug!("websocket received message: {:?}", msg);
    }
    Ok(())
}

pub(crate) async fn run_client(config: WebsocketConfig, websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, tx: Sender<Command>) -> Result<(), ClientError> {
    match config.provider {
        ServiceProvider::EXTOY => extoys(websocket, tx).await?,
        ServiceProvider::INTI => intiface(websocket, tx).await?,
    }
    Ok(())
}

    async fn ws_loop(websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, strict: bool) -> Result<(), ClientError> {
        while let Some(packet) = websocket.next().await {
            let packet = packet?;
            if let Message::Binary(bytes) = packet {
                let linear_action = tcode_de::process_linear_token(&bytes[..(bytes.len()-1)]);
                if let Ok(action) = linear_action {
                    //self.gcode_channel.send(Command::Movement(action)).await?;
                } else if let Err(e) = linear_action && strict { // if strict is not enabled silently ignore.
                    return Err(ClientError::LinearAction(e));
                }
            }
        }
        Ok(())
    }