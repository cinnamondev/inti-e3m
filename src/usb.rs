use ratatui::text::ToLine;
use thiserror::Error;
use tokio::{io, task};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tokio_util::sync::CancellationToken;
use crate::tui::config::MachineConfig;
use crate::tui::event::{AppEvent, Event, EventHandler};
use crate::usb::Action::MOVE;
use crate::usb::GCodeError::UnsupportedMovement;

#[derive(Debug, Clone)]
pub enum Command {
    Movement(LinearAction),
    Home,
    Halt,
}

#[derive(Debug, Clone)]
pub struct LinearAction {
    pub action: Action,
    pub id: u32,
    pub magnitude: u32,
    pub modifier: Option<LinearModifier>
}
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Action {
    MOVE,
    ROTATE,
    VIBRATE,
    AUXILLARY
}

#[derive(Debug, Clone)]
pub enum LinearModifier {
    TIME(u32),
    SPEED(u32)
}
#[derive(Debug, Error)]
pub enum GCodeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serial(#[from] tokio_serial::Error),
    #[error("Unsupported movement!")]
    UnsupportedMovement(Action),
}
pub async fn run_server(config: MachineConfig, mut rx: Receiver<Command>, event_handler: UnboundedSender<Event>) -> Result<(), GCodeError> {
    let mut serial = tokio_serial::new(config.file.clone(), 250000).open_native_async()?;
    while let Some(command) = rx.recv().await {
        let command: Command = command;
        log::debug!("{:?}", command);
        let mut last_linear_action: Option<LinearAction> = None;
        match command {
            Command::Movement(action) => {
                let gcode = &*create_gcode(&config, &action, last_linear_action)?;
                let ret =serial.write(gcode).await?;
                last_linear_action = Some(action);
                event_handler.send(Event::App(AppEvent::GCode(format!("{:?}", gcode))));
            }
            Command::Halt => {
                serial.write_all(b"M112\n").await?;
                serial.flush().await?;
                event_handler.send(Event::App(AppEvent::GCode("M112".to_string())));
            },
            Command::Home => {
                serial.write_all(b"G28 X\n").await?;
                serial.flush().await?; // ensure
                event_handler.send(Event::App(AppEvent::GCode("G28 X".to_string())));
            },
        }
    }
    Ok(())
}

fn create_gcode(config: &MachineConfig, action: &LinearAction, last_action: Option<LinearAction>) -> Result<Vec<u8>, GCodeError> {
    let last_action = last_action.unwrap_or_else(|| LinearAction {
        action: Action::MOVE,
        id: 0,
        magnitude: 0,
        modifier: None,
    });

    if action.action != MOVE { return Err(UnsupportedMovement(action.action.clone())); }
    // now lets make a gcode for it
    let mut output = String::from("G1 X");
    let distance = action.magnitude_to_distance(config.max_movement);

    output.push_str(&format!("{:.2} ", (config.throw - config.max_movement) as f32+ distance));
    // distance is in MM so speed is MM/h.ms -> MM/min
    let feedrate = match action.modifier {
        Some(LinearModifier::SPEED(mmPerHundredMs)) => {
            if let Some(LinearModifier::SPEED(last)) = last_action.modifier && last == mmPerHundredMs{
                String::new() // same as previous we dont need to redo
            } else { format!("F{}\n", mmPerHundredMs*600) }
        },
        //Some(LinearModifier::SPEED(_)) => { String::from("F2000") } // limited speed ver.
        Some(LinearModifier::TIME(ms)) => {
            if let Some(LinearModifier::TIME(last)) = last_action.modifier && last == ms {
                String::new() // same as previous we dont need to redo.
            } else {
                let previous_distance = last_action.magnitude_to_distance(config.max_movement);
                let speed = ((distance-previous_distance).abs())/ (ms as f32 / 60_000.00);
                //if speed > 7000f32 { speed = 2000f32; } // if the speed goes haywire we start forcing it to slow.
                format!("F{:.2}\n", speed)
            }
        }
        None => "".to_string()
    };
    output.push_str(&feedrate);
    Ok(output.as_bytes().to_vec())
}

impl LinearAction {
    // max distance (mm) -> distance on scale in mm
    pub fn magnitude_to_distance(&self, max_distance: u32) -> f32 {
        if self.magnitude <= 0 { return 0f32; }
        let digits = self.magnitude.ilog10() + 1;
        let distance = max_distance as f32 * (self.magnitude as f32 / 10f32.powi(digits as i32));

        distance.max(max_distance as f32)
    }
}