use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag="mode",rename_all="snake_case")]
pub enum ExtoyPacket {
    Speed {
        speed: u8,
        upper: u8,
        lower: u8,
    },
    Position {
        duration: u8,
        position: u8,
    },
}