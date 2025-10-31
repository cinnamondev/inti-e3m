#![feature(if_let_guard)]

use std::cmp::PartialEq;
use std::io;
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::FusedStream;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::sync::CancellationToken;
use crate::tui::app::App;
use crate::tui::config::Config;
use crate::usb::Command;

mod tcode_de;
mod tui;
mod websocket;
mod usb;
mod server;


/**
async fn ws_loop(websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, tx: Sender<Command>, strict: bool) -> Result<(), ClientError> {
    while let Some(packet) = websocket.next().await {
        let packet = packet?;
        if let Message::Binary(bytes) = packet {
            let linear_action = tcode_de::process_linear_token(&bytes[..(bytes.len()-1)]);
            if let Ok(action) = linear_action {
                tx.send(Command::Movement(action)).await?;
            } else if let Err(e) = linear_action && strict { // if strict is not enabled silently ignore.
                return Err(ClientError::LinearAction(e));
            }
        }
    }
    Ok(())
}
*/

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Set max_log_level to Trace
    //tui_logger::init_logger(log::LevelFilter::Debug).unwrap();
    // Set default level for unknown targets to Trace
    //tui_logger::set_default_level(log::LevelFilter::Trace);
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::using_config(Config::default()).run(terminal).await;
    ratatui::restore();
    result
}

/**
#[tokio::main]
async fn main() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Command>(100);
    let token = CancellationToken::new();

    let mut serial_stream = tokio_serial::new("/dev/ttyUSB0", 250000)
        .open_native_async()
        .expect("Failed to open serial port");

    let serial_token = token.clone();
    let ctrl_c_token = token.clone();
    let serial_handle = tokio::spawn(async move {
        // safe initial points for gcode
        serial_stream.write("G1 F1000\n".as_bytes()).await.unwrap();
        serial_stream.write("G28 X\n".as_bytes()).await.unwrap();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if (CONTROL_C_IS_FATAL) {
                    serial_stream.write_all(b"M112\n").await.unwrap();
                    serial_stream.flush().await.unwrap();
                }
                ctrl_c_token.cancel();
            },
            _ = serial_token.cancelled() => {
                println!("Shutting down GCode");
            },
            r = gcode_loop(&mut serial_stream, &mut rx) => { // gcode loop will accept commands now
                println!("Shutting down GCode prematurely! :(");
                if let Err(e) = r {
                    println!("Error while handling GCode: {}", e);
                }
            }
        }
    });

    let ws_tx = tx.clone();
    let ws_token = token.clone();
    let ws_handle = tokio::spawn(async move {
        let (mut websocket, _) = connect_async("ws://localhost:54817").await
            .expect("Failed to connect ! :(");
        websocket.send(Message::Text(Utf8Bytes::from(
            format!("{{\"identifier\":\"{0}\",\"address\":\"{1}\",\"version\":0}}", "UpYourEnder", 2))
        )).await.expect("Failed to send identifying packet... :(");

        tokio::select! {
            _ = ws_token.cancelled() => {
                println!("Shutting down WebSocket!");
            },
            r = ws_loop(&mut websocket, ws_tx, true) => {
                println!("Shutting down WebSocket prematurely! :(");
                if let Err(e) = r {
                    println!("Error while handling GCode: {}", e);
                }
            }
        }
        websocket.close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: Default::default()
        })).await;
        dbg!("Closing!");
    });

    let io_tx = tx.clone();
    let io_token = token.clone();
    tokio::spawn(async move {
        let r = io_listener(io_tx, io_token).await;
        if let Err(e) = r {
            println!("Error while listening! :( {}", e);
        }
    });

    token.cancelled().await;
}
*/
async fn io_listener(tx: Sender<Command>, token: CancellationToken) -> Result<(), SendError<Command>> {
    while let Ok(n) = tokio::io::stdin().read_u8().await && !tx.is_closed() {
        match n as char {
            'S' => tx.send(Command::Halt).await?,
            'X' => { token.cancel(); return Ok(()); },
            'H' | 'h' => tx.send(Command::Home).await?,
            _ => {}
        }
    }
    Ok(())
}

