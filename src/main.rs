#![feature(if_let_guard)]
extern crate core;

use crate::tui::app::App;
use crate::usb::Command;
use config::Config;

mod tcode_de;
mod tui;
mod websocket;
mod usb;
mod server;
pub(crate) mod config;
mod extoy_de;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Set max_log_level to Trace
    tui_logger::init_logger(log::LevelFilter::Debug).unwrap();
    // Set default level for unknown targets to Trace
    tui_logger::set_default_level(log::LevelFilter::Trace);
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::using_config(Config::default()).run(terminal).await;
    ratatui::restore();
    result
}


