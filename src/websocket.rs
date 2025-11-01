use crate::config::{WebsocketConfig};
use crate::extoy_de::ExtoyPacket;
use crate::tcode_de::LinearActionError;
use crate::usb::LinearModifier::TIME;
use crate::usb::{Action, LinearAction};
use crate::websocket::ClientError::{InvalidListener};
use crate::{tcode_de, Command};
use futures_util::SinkExt;
use futures_util::{StreamExt, TryStreamExt};
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::{accept_async, connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Tungstenite(#[from] tungstenite::error::Error),
    #[error(transparent)]
    LinearAction(#[from] LinearActionError),
    #[error(transparent)]
    Mspc(#[from] SendError<Command>),
    #[error("not valid uri")]
    InvalidListener,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("unsupported")]
    UnsupportedAction,
}

pub(crate) async fn extoys(config: &WebsocketConfig, tx: Sender<Command>, token: CancellationToken) -> Result<(), ClientError> {
    let listener = TcpListener::bind(config.ws.strip_prefix("ws://").ok_or(InvalidListener)?).await?;
    let (stream,_) = listener.accept().await?;
    let mut websocket = accept_async(stream).await?;

    // position logic:
    // move to position at some speed
    while !token.is_cancelled() && let Some(msg) = websocket.try_next().await? {
        if let Message::Text(b) = &msg {
            let str = b.as_str();
            let packet: ExtoyPacket = serde_json::from_str(str)?;
            if let ExtoyPacket::Position { position, mut duration} = packet {
                let BASE_F = 10;
                let ACTUAL_F = 50;
                if BASE_F != ACTUAL_F {
                    duration = (duration as f32 * (BASE_F as f32 / ACTUAL_F as f32)) as u32;
                }
                log::debug!("Received extoy packet: {:?}", packet);
                let action = LinearAction {
                    action: Action::MOVE,
                    id: 0,
                    magnitude: position.min(99) as u32,
                    modifier: Some(TIME(duration as u32))
                };
                log::debug!("Processed action: {:?}", action);
                tx.send(Command::Movement(action)).await?;
            } else {
                log::error!("unsupported speed action will be ignored !");
            }
        }
        //log::debug!("websocket received message: {:?}", msg);
    }

    // speed logic:
    // Move to nearest bound (from last known position) at speed
    // I'M JUST NOT GOING TO IMPLEMENT SPEED IT'S TOO ANNOYING GRR
    /** // im leaving this here incase ever want to try implementing speed in future. in short
    // we need to keep checking time elapsed and checking if it is longer than the expected time for
    // old positon -> new position then send the gcode to move to the other side once it is.
    // OR maybe it would be better to send a stream of commands splitting up the movement so we can
    // turn around if extoys asks us to
    while !token.is_cancelled() {
        let current_command: ExtoyPacket;
        if let Some(Some(msg)) = websocket.next().now_or_never() {
            let msg = msg?;
            log::debug!("websocket received message: {:?}", msg);
        }
    }
*/
    Ok(())
}

pub async fn intiface(config:&WebsocketConfig, tx: Sender<Command>, token: CancellationToken) -> Result<(), ClientError> {
    let (mut websocket, _) = connect_async(config.ws.as_str()).await?;

    websocket.send(Message::Text(Utf8Bytes::from(
        format!("{{\"identifier\":\"{0}\",\"address\":\"{1}\",\"version\":0}}", "UpYourEnder", 2))
    )).await?;

    while !token.is_cancelled() && let Some(msg) = &websocket.try_next().await? {
        let msg = msg;
        log::debug!("websocket received message: {:?}", msg);
        if let Message::Binary(bytes) = msg {
            let linear_action = tcode_de::process_linear_token(&bytes[..(bytes.len()-1)]);
            if let Ok(action) = linear_action {
                tx.send(Command::Movement(action)).await?;
            } else if let Err(e) = linear_action { // if strict is not enabled silently ignore.
                return Err(ClientError::LinearAction(e));
            }
        }
    }

    Ok(websocket.close(Some(CloseFrame {
        code: CloseCode::Normal,
        reason: Default::default(),
    })).await?)
}
