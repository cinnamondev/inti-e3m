use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag="mode",rename_all="snake_case")]
pub enum ExtoyPacket {
    Speed {
        speed: u32,
        upper: u32,
        lower: u32,
    },
    Position {
        duration: u32,
        position: u32,
    },
}