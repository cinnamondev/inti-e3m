// its only sort of a server... but wrapping this into one made sense.

use futures::channel;
use thiserror::Error;
use tokio::io;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};
use tokio::task::{JoinError, JoinHandle};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_util::sync::CancellationToken;
use crate::Command;
use crate::tui::app::App;
use crate::tui::config::Config;
use crate::tui::event::{AppEvent, Event};
use crate::usb::GCodeError;
use crate::websocket::ClientError;

#[derive(Debug,Error)]
pub enum ServerError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Websocket(#[from] ClientError),
    #[error(transparent)]
    Tokio(#[from] JoinError),
    #[error(transparent)]
    Gcode(#[from] GCodeError)
}

async fn start_with_configs(channel: (Sender<Command>, Receiver<Command>), token: CancellationToken, config: Config, app_tx: UnboundedSender<Event>) -> Result<(), ServerError> {
    let (mut websocket, _) =
        connect_async(config.websocket_config.ws.as_str()).await
            .map_err(|e| ClientError::from(e))?;

    let (tx, rx) = channel;
    let token = token.clone();

    let mut r = tokio::select! {
        _ = token.cancelled() => {
            log::info!("Server has been stopped (manually!)");
            Ok(())
        },
        result = crate::usb::run_server(config.machine_config, rx,app_tx) => result.map_err(ServerError::from),
        result = crate::websocket::run_client(config.websocket_config, &mut websocket, tx) => result.map_err(ServerError::from),
    };
    if let Err(e) = &r {
        token.cancel();
        log::error!("{}", e);
    }
    // do cleanup
    let close_response = websocket.close(Some(CloseFrame {
        code: CloseCode::Normal,
        reason: Default::default()
    })).await;

    if let Err(e) = &close_response {
        token.cancel();
        log::error!("Couldn't send close packet to websocket! :(");
        log::error!("{}", e);
        if r.is_ok() { r = close_response.map_err(ClientError::from).map_err(ServerError::from); }
    }

    r
}

#[derive(Debug)]
pub struct Server {
    pub(crate) token: CancellationToken,
    pub(crate) tx: Sender<Command>,
    pub(crate) handle: JoinHandle<Result<(), ServerError>>,
}

impl Server {
    pub fn start(config: Config, app_tx: UnboundedSender<Event>) -> Self {
        let token = CancellationToken::new();
        let channel = tokio::sync::mpsc::channel::<Command>(100);
        Self {
            token: token.clone(),
            tx: channel.0.clone(),
            handle: tokio::spawn(start_with_configs(channel, token, config, app_tx))
        }
    }

}
