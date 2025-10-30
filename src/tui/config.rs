use std::fmt::Display;

#[derive(Debug)]
#[derive(Default)]
pub struct Config<'a> {
    pub machine_config: MachineConfig<'a>,
    pub websocket_config: WebsocketConfig<'a>,
}
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum ServiceProvider {
    EXTOY,
    INTI,
}

impl Display for ServiceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[derive(Debug)]
pub struct WebsocketConfig<'a> {
    pub provider: ServiceProvider,
    pub ws: &'a str,
}

impl<'a> Default for WebsocketConfig<'a> {
    fn default() -> WebsocketConfig<'a> {
        WebsocketConfig {
            provider: ServiceProvider::INTI,
            ws: "wss://localhost:8080",
        }
    }
}
#[derive(Debug)]
pub struct MachineConfig<'a> {
    pub file: &'a str,
    pub throw: u32,
    pub max_movement: u32,
    pub max_acceleration: u32
}

impl<'a> Default for MachineConfig<'a> {
    fn default() -> Self {
        MachineConfig {
            file: "/dev/ttyUSB0",
            throw: 240,
            max_movement: 100,
            max_acceleration: 100000
        }
    }
}
