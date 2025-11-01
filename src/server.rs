// its only sort of a server... but wrapping this into one made sense.

use color_eyre::eyre::private::kind::TraitKind;
use ratatui::prelude::Span;
use crate::config::{Config, ServiceProvider};
use crate::tui::event::{AppEvent, ErrorKind, Event};
use crate::usb::GCodeError;
use crate::websocket::ClientError;
use crate::Command;
use thiserror::Error;
use tokio::io;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};
use tokio::task::{JoinError, JoinHandle};
use tokio_util::sync::CancellationToken;
use crate::tui::bar::Status;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    Websocket(#[from] ClientError),
    #[error(transparent)]
    Tokio(#[from] JoinError),
    #[error(transparent)]
    Gcode(#[from] GCodeError)
}

async fn start_with_configs(channel: (Sender<Command>, Receiver<Command>), token: CancellationToken, config: Config, app_tx: UnboundedSender<Event>) -> Result<(), ServerError> {
    let (tx, rx) = channel;
    let token = token.clone();

    let mut r = tokio::select! {
        result = crate::usb::run_server(config.machine_config, rx, app_tx.clone() ,token.clone()) => result.map_err(ServerError::from),
        result = crate::websocket::intiface(&config.websocket_config, tx.clone(), token.clone()),
            if config.websocket_config.provider == ServiceProvider::INTI => result.map_err(ServerError::from),
        result = crate::websocket::extoys(&config.websocket_config, tx.clone(), token.clone()),
            if config.websocket_config.provider == ServiceProvider::EXTOY => result.map_err(ServerError::from)
    };
    if token.is_cancelled() {
        log::info!("Server closed manually!");
    }
    if let Err(e) = &r {
        token.cancel();
        log::error!("{}", e);

        let kind = match &e {
            ServerError::Websocket(ClientError::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => ErrorKind::Websocket("Invalid URI".to_string()),
            ServerError::Websocket(we) =>  ErrorKind::Websocket("Error".to_string()),
            //ServerError::Gcode(GCodeError::Io(e)) if e.kind() == io::ErrorKind::ConnectionAborted => ErrorKind::GCode("Not connected!".to_string()),
            ServerError::Gcode(GCodeError::Io(e)) if e.kind() == io::ErrorKind::PermissionDenied => ErrorKind::GCode("No permission!".to_string()),
            ServerError::Gcode(GCodeError::Io(e)) if e.kind() == io::ErrorKind::NotFound => ErrorKind::GCode("Not connected!".to_string()),
            ServerError::Gcode(ge) => ErrorKind::GCode("Error".to_string()),
            ServerError::Tokio(_) => ErrorKind::Websocket("Not connected!".to_string()) // unlikely and if we do its a bigger problem
        };
        app_tx.send(Event::App(AppEvent::ServerError(kind))).expect("app error channel went wrong, you're on your own.");
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
