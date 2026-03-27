//! MIDI System Common messages (status bytes `0xF0..=0xF7`).
//!
//! These messages are intended for all receivers in a MIDI system. They include
//! System Exclusive (SysEx), Song Position Pointer, Song Select, and Tune Request.
//!
//! Unlike System Real-Time messages, these may carry data bytes and cannot be
//! interleaved within other messages.

use std::fmt::Display;

use crate::message_channel_voice::mask_data;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Status {
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    SysExEnd = 0xF7,
    SysExStart = 0xF0,
    TuneRequest = 0xF6,
}

impl From<Status> for u8 {
    fn from(value: Status) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for Status {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            v if v == Status::SongPositionPointer.into() => Status::SongPositionPointer,
            v if v == Status::SongSelect.into() => Status::SongSelect,
            v if v == Status::SysExEnd.into() => Status::SysExEnd,
            v if v == Status::SysExStart.into() => Status::SysExStart,
            v if v == Status::TuneRequest.into() => Status::TuneRequest,
            undefined => return Err(undefined),
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum Message {
    SongPositionPointer(u16),
    SongSelect(u8),
    SysEx(Vec<u8>),
    TuneRequest,
}

impl Message {
    pub fn from_bytes(status: Status, bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        match status {
            Status::SongPositionPointer => Self::content_song_position_pointer(bytes),
            Status::SongSelect => Self::content_song_select(bytes),
            Status::SysExEnd => Err(ParseError::SysExEndBeforeStart),
            Status::SysExStart => Self::content_sys_ex(bytes),
            Status::TuneRequest => Ok((0, Self::TuneRequest)),
        }
    }

    fn content_song_position_pointer(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let lsb = *bytes.first().ok_or(ParseError::DataNotEnoughBytes {
            available: 0,
            length: 2,
        })?;
        let msb = *bytes.get(1).ok_or(ParseError::DataNotEnoughBytes {
            available: 1,
            length: 2,
        })?;

        let value = (u16::from(mask_data(msb))) << 7 | u16::from(mask_data(lsb));

        Ok((2, Self::SongPositionPointer(value)))
    }

    fn content_song_select(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let byte = *bytes.first().ok_or(ParseError::DataNotEnoughBytes {
            available: 0,
            length: 1,
        })?;

        Ok((1, Self::SongSelect(mask_data(byte))))
    }

    fn content_sys_ex(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let pos_end = bytes
            .iter()
            .position(|b| b == &Status::SysExEnd.into())
            .ok_or(ParseError::SysExEndMissing)?;

        // SAFETY: `pos_end` is obtained with `iter().position()` above, which guarantees: `0 <=
        // pos_end < len()`
        let content = unsafe { bytes.get_unchecked(..pos_end) };
        let content: Vec<_> = content.iter().map(|b| mask_data(*b)).collect();

        Ok((pos_end + 1, Self::SysEx(content)))
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    DataNotEnoughBytes { available: usize, length: usize },
    SysExEndBeforeStart,
    SysExEndMissing,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::DataNotEnoughBytes { available, length } => write!(
                f,
                "data not enough bytes, length: {length}, available: {available}"
            ),
            ParseError::SysExEndBeforeStart => write!(f, "sys ex end before start"),
            ParseError::SysExEndMissing => write!(f, "sys ex end missing"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        message_channel_voice::mask_data,
        message_system_common::{Message, ParseError, Status},
    };

    #[test]
    fn status_undefined() {
        assert_eq!(Status::try_from(0xF1), Err(0xF1));
        assert_eq!(Status::try_from(0xF4), Err(0xF4));
        assert_eq!(Status::try_from(0xF5), Err(0xF5));
    }

    #[test]
    fn status_song_position_pointer() {
        assert_eq!(Status::try_from(0xF2), Ok(Status::SongPositionPointer));
    }

    #[test]
    fn status_song_select() {
        assert_eq!(Status::try_from(0xF3), Ok(Status::SongSelect));
    }

    #[test]
    fn status_sys_ex_end() {
        assert_eq!(Status::try_from(0xF7), Ok(Status::SysExEnd));
    }

    #[test]
    fn status_sys_ex_start() {
        assert_eq!(Status::try_from(0xF0), Ok(Status::SysExStart));
    }

    #[test]
    fn status_tune_request() {
        assert_eq!(Status::try_from(0xF6), Ok(Status::TuneRequest));
    }

    #[test]
    fn song_position_pointer_missing_lsb() {
        assert_eq!(
            Message::from_bytes(Status::SongPositionPointer, &[]),
            Err(ParseError::DataNotEnoughBytes {
                available: 0,
                length: 2
            })
        );
    }

    #[test]
    fn song_position_pointer_missing_msb() {
        assert_eq!(
            Message::from_bytes(Status::SongPositionPointer, &[0x01]),
            Err(ParseError::DataNotEnoughBytes {
                available: 1,
                length: 2
            })
        );
    }

    #[test]
    fn song_position_pointer() {
        let lsb: u8 = 0x71;
        let msb = 0x02;
        let value = u16::from(msb) << 7 | u16::from(lsb);
        assert_eq!(
            Message::from_bytes(Status::SongPositionPointer, &[lsb, msb]),
            Ok((2, Message::SongPositionPointer(value)))
        );

        let lsb = 0xFF;
        let msb = 0xFF;
        let value = (u16::from(mask_data(msb))) << 7 | u16::from(mask_data(lsb));
        assert_eq!(
            Message::from_bytes(Status::SongPositionPointer, &[lsb, msb]),
            Ok((2, Message::SongPositionPointer(value)))
        );
    }

    #[test]
    fn song_select_missing_byte() {
        assert_eq!(
            Message::from_bytes(Status::SongSelect, &[]),
            Err(ParseError::DataNotEnoughBytes {
                available: 0,
                length: 1
            })
        );
    }

    #[test]
    fn song_select() {
        assert_eq!(
            Message::from_bytes(Status::SongSelect, &[0x01]),
            Ok((1, Message::SongSelect(0x01)))
        );

        assert_eq!(
            Message::from_bytes(Status::SongSelect, &[0xFF]),
            Ok((1, Message::SongSelect(mask_data(0xFF))))
        );
    }

    #[test]
    fn sys_ex_end_before_start() {
        assert_eq!(
            Message::from_bytes(Status::SysExEnd, &[]),
            Err(ParseError::SysExEndBeforeStart)
        );
    }

    #[test]
    fn sys_ex_end_missing() {
        assert_eq!(
            Message::from_bytes(Status::SysExStart, &[]),
            Err(ParseError::SysExEndMissing)
        );
        assert_eq!(
            Message::from_bytes(Status::SysExStart, &[0x01]),
            Err(ParseError::SysExEndMissing)
        );
    }

    #[test]
    fn sys_ex() {
        assert_eq!(
            Message::from_bytes(
                Status::SysExStart,
                &[0x01, Status::SysExEnd.into()]
            ),
            Ok((2, Message::SysEx(vec![0x01]))),
        );
        assert_eq!(
            Message::from_bytes(
                Status::SysExStart,
                &[0xFF, 0xFF, Status::SysExEnd.into()]
            ),
            Ok((
                3,
                Message::SysEx(vec![mask_data(0xFF), mask_data(0xFF),])
            )),
        );
    }

    #[test]
    fn tune_request() {
        assert_eq!(
            Message::from_bytes(Status::TuneRequest, &[]),
            Ok((0, Message::TuneRequest))
        );
    }
}
