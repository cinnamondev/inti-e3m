use tokio::{io, task};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tokio_util::sync::CancellationToken;
use crate::Command;
use crate::tui::config::MachineConfig;

pub async fn run_server(config: MachineConfig, mut rx: Receiver<Command>) -> io::Result<()> {
    let mut serial = tokio_serial::new(config.file.clone(), 250000).open()?;
    while let Some(command) = rx.recv().await {
        let command: Command = command;
        log::debug!("{:?}", command);
        match command {
            Command::Movement(movement) => {}
            Command::Home => {}
            Command::Halt => {}
        }
    }
    Ok(())
}