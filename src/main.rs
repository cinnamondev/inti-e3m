#![feature(if_let_guard)]

use std::cmp::PartialEq;
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use futures_util::{future, pin_mut, SinkExt, StreamExt};
use futures_util::stream::FusedStream;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::{SendError, TryRecvError};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_util::sync::CancellationToken;
use log::log;
use crate::tcode_de::Action::MOVE;
use crate::tcode_de::{process_linear_token, Action, LinearAction, LinearActionError, LinearModifier};
use crate::tui::app::App;

mod tcode_de;
mod tui;
mod websocket_client;

const SHAFT_LENGTH: u32 = 240; // 240 mm max distance!
const KEEPALIVE_WS: bool = false;
const CONTROL_C_IS_FATAL: bool = true; // HALT on control c!

enum Command {
    Movement(LinearAction),
    Home,
    Halt,
}

async fn gcode_loop(serial: &mut SerialStream, rx: &mut Receiver<Command>) -> io::Result<()> {
    while let Some(command) = rx.recv().await {
        let mut last_linear_action: Option<LinearAction> = None;
        match command {
            Command::Movement(action) => {
                let ret =serial.write(
                    &*create_gcode(&action, last_linear_action).unwrap()
                ).await?;
                last_linear_action = Some(action);
            }
            Command::Halt => {
                serial.write_all(b"M112\n").await?;
                serial.flush().await?;
            },
            Command::Home => {
                serial.write_all(b"G28 X\n").await?;
                serial.flush().await?; // ensure
            },
        }
    }
    Ok(())
}

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

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Set max_log_level to Trace
    tui_logger::init_logger(log::LevelFilter::Trace).unwrap();
    // Set default level for unknown targets to Trace
    tui_logger::set_default_level(log::LevelFilter::Trace);
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal).await;
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
#[derive(Debug,Error)]
enum ClientError {
    #[error(transparent)]
    Tungstenite(#[from] tungstenite::error::Error),
    #[error(transparent)]
    LinearAction(#[from] LinearActionError),
    #[error(transparent)]
    Mspc(#[from] SendError<Command>)
}

fn create_gcode(action: &LinearAction, last_action: Option<LinearAction>) -> Result<Vec<u8>, ()> {
    let last_action = last_action.unwrap_or_else(|| LinearAction {
        action: Action::MOVE,
        id: 0,
        magnitude: 0,
        modifier: None,
    });

    if action.action != MOVE { return Err(()); }
    // now lets make a gcode for it
    let mut output = String::from("G1 X");
    let mut distance;
    if action.magnitude == 0 {
        distance = 0f32;
    } else {
        let digits = action.magnitude.checked_ilog10().unwrap_or(0) + 1;
        distance = SHAFT_LENGTH as f32 * (action.magnitude as f32 / 10f32.powi(digits as i32));
    }

    if distance > SHAFT_LENGTH as f32 { distance = SHAFT_LENGTH as f32; } // just.. double check
    if distance < 0f32 { distance = 0f32; }
    output.push_str(&format!("{:.2} ", distance));
    // distance is in MM so speed is MM/h.ms -> MM/min
    let feedrate = match action.modifier {
        Some(LinearModifier::SPEED(mmPerHundredMs)) => {
            if let Some(LinearModifier::SPEED(last)) = last_action.modifier && last == mmPerHundredMs{
                String::new()
            } else { format!("F{}\n", mmPerHundredMs*600) }
        },
        //Some(LinearModifier::SPEED(_)) => { String::from("F2000") } // limited speed ver.
        Some(LinearModifier::TIME(ms)) => {
            if let Some(LinearModifier::TIME(last)) = last_action.modifier && last == ms {
                String::new()
            } else {
                let magnitude_diff = (action.magnitude.abs_diff(last_action.magnitude)); //
                let digits = magnitude_diff.checked_ilog10().unwrap_or(0) + 1;
                let speed = (SHAFT_LENGTH as f32 * (magnitude_diff as f32 / 10f32.powi(digits as i32))) / (ms as f32 / 60_000.00);
                //if speed > 7000f32 { speed = 2000f32; } // if the speed goes haywire we start forcing it to slow.
                format!("F{:.2}\n", speed)
            }
        }
        None => "".to_string()
    };
    output.push_str(&feedrate);
    Ok(output.as_bytes().to_vec())
}

