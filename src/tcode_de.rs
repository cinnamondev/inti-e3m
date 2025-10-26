use std::str::Utf8Error;
use thiserror::Error;
use tokio_tungstenite::tungstenite::handshake::machine;
#[derive(Debug, Eq, PartialEq,)]
pub enum Action {
    MOVE,
    ROTATE,
    VIBRATE,
    AUXILLARY
}
#[derive(Debug)]
pub enum LinearModifier {
    TIME(u32),
    SPEED(u32)
}

#[derive(Debug)]
pub struct LinearAction {
    pub action: Action,
    pub id: u32,
    pub magnitude: u32,
    pub modifier: Option<LinearModifier>
}
#[derive(Error, Debug)]
pub enum LinearActionError {
    #[error("Invalid ID `{0}` given!")]
    InvalidID(char),
    #[error("Invalid linear movement. Expected <l/r/v/a>, got {0}")]
    InvalidOpcode(char),
    #[error("i unno, uhhh. have {0:?}")]
    CommandInvalid(Vec<u8>),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}
/// assumes format of L<id><magnitude><I/T><magnitude>
pub fn process_linear_token(bytes: &[u8]) -> Result<LinearAction, LinearActionError> {
    if (bytes.len() < 3) { return Err(LinearActionError::CommandInvalid(bytes.to_vec())) }

    let action = match bytes[0] as char {
        'l' | 'L' => Action::MOVE,
        'R' | 'r' => Action::ROTATE,
        'V' | 'v' => Action::VIBRATE,
        'A' | 'a' => Action::AUXILLARY,
        c => return Err(LinearActionError::InvalidOpcode(c))
    };
    let id = (bytes[1] as char).to_digit(10)
        .ok_or_else(|| LinearActionError::InvalidID(bytes[1] as char))?;

    let position = bytes.iter().position(|&c| { let c = c as char; c == 'I' || c == 'i' || c == 'S' || c == 's' });
    let magnitude: u32;
    let modifier: Option<LinearModifier>;
    if let Some(i) = position { // we have a time modifier, only consume up to it.
        magnitude = str::from_utf8(&bytes[2..i])?.parse()?; // get magnitude
        let n: u32;
        n = str::from_utf8(&bytes[(i+1)..])?.parse()?; // get time/speed thing

        match bytes[i] as char {
            'I' | 'i' => modifier = Some(LinearModifier::TIME(n)),
            'S' | 's' => modifier = Some(LinearModifier::SPEED(n)),
            c => return Err(LinearActionError::InvalidOpcode(c))
        }
    } else {
        modifier = None;
        magnitude = str::from_utf8(&bytes[2..])?.parse()?;
    };

    Ok(LinearAction {
        action,
        id,
        magnitude,
        modifier
    })
}