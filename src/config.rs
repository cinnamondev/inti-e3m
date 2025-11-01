use std::fmt::Display;

#[derive(Debug)]
#[derive(Default, Clone)]
pub struct Config {
    pub machine_config: MachineConfig,
    pub websocket_config: WebsocketConfig,
}
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum ServiceProvider {
    EXTOY,
    INTI,
}

impl Display for ServiceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[derive(Debug, Clone)]
pub struct WebsocketConfig {
    pub provider: ServiceProvider,
    pub ws: String,
}

impl Default for WebsocketConfig {
    fn default() -> WebsocketConfig {
        WebsocketConfig {
            provider: ServiceProvider::INTI,
            ws: "ws://localhost:54817".to_string(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct MachineConfig {
    pub file: String,
    pub throw: u32,
    pub max_movement: u32,
    pub max_acceleration: u32
}

impl Default for MachineConfig {
    fn default() -> Self {
        MachineConfig {
            file: "/dev/ttyUSB0".to_string(),
            throw: 240,
            max_movement: 100,
            max_acceleration: 100000
        }
    }
}
