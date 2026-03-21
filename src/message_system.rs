use std::mem;

use crate::message_channel_voice::mask_data;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    ActiveSensing = 0xFE,
    Reset = 0xFF,
    SequenceContinue = 0xFB,
    SequenceStart = 0xFA,
    SequenceStop = 0xFC,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    SysExEnd = 0xF7,
    SysExStart = 0xF0,
    TimingClock = 0xF8,
    TuneRequest = 0xF6,
}

impl From<Status> for u8 {
    fn from(value: Status) -> Self {
        const { assert!(mem::size_of::<Status>() == mem::size_of::<u8>()) };
        // SAFETY: Transmutation is guaranteed to be a valid in the representation of an u8, given
        // the check above.
        unsafe { mem::transmute::<Status, u8>(value) }
    }
}

impl TryFrom<u8> for Status {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            v if v == Status::ActiveSensing.into() => Status::ActiveSensing,
            v if v == Status::Reset.into() => Status::Reset,
            v if v == Status::SequenceContinue.into() => Status::SequenceContinue,
            v if v == Status::SequenceStart.into() => Status::SequenceStart,
            v if v == Status::SequenceStop.into() => Status::SequenceStop,
            v if v == Status::SongPositionPointer.into() => Status::SongPositionPointer,
            v if v == Status::SongSelect.into() => Status::SongSelect,
            v if v == Status::SysExEnd.into() => Status::SysExEnd,
            v if v == Status::SysExStart.into() => Status::SysExStart,
            v if v == Status::TimingClock.into() => Status::TimingClock,
            v if v == Status::TuneRequest.into() => Status::TuneRequest,
            undefined => return Err(undefined),
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum MessageSystem {
    ActiveSensing,
    Reset,
    SequenceContinue,
    SequenceStart,
    SequenceStop,
    SongPositionPointer(u16),
    SongSelect(u8),
    SysEx(Vec<u8>),
    TimingClock,
    TuneRequest,
}

impl MessageSystem {
    pub fn from_bytes(status: Status, bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        match status {
            Status::ActiveSensing => Ok((0, Self::ActiveSensing)),
            Status::Reset => Ok((0, Self::Reset)),
            Status::SequenceContinue => Ok((0, Self::SequenceContinue)),
            Status::SequenceStart => Ok((0, Self::SequenceStart)),
            Status::SequenceStop => Ok((0, Self::SequenceStop)),
            Status::SongPositionPointer => Self::content_song_position_pointer(bytes),
            Status::SongSelect => Self::content_song_select(bytes),
            Status::SysExEnd => Err(ParseError::SysExEndBeforeStart),
            Status::SysExStart => Self::content_sys_ex(bytes),
            Status::TimingClock => Ok((0, Self::TimingClock)),
            Status::TuneRequest => Ok((0, Self::TuneRequest)),
        }
    }

    fn content_song_position_pointer(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let lsb = *bytes.first().ok_or(ParseError::LengthNotEnoughBytes {
            actual: 0,
            expected: 2,
        })?;
        let msb = *bytes.get(1).ok_or(ParseError::LengthNotEnoughBytes {
            actual: 1,
            expected: 2,
        })?;

        let value = (u16::from(mask_data(msb))) << 7 | u16::from(mask_data(lsb));

        Ok((2, Self::SongPositionPointer(value)))
    }

    fn content_song_select(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let byte = *bytes.first().ok_or(ParseError::LengthNotEnoughBytes {
            actual: 0,
            expected: 1,
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
    LengthNotEnoughBytes { actual: usize, expected: usize },
    SysExEndBeforeStart,
    SysExEndMissing,
}

#[cfg(test)]
mod tests {
    use crate::{
        message_channel_voice::mask_data,
        message_system::{MessageSystem, ParseError, Status},
    };

    #[test]
    fn status_undefined() {
        assert_eq!(Status::try_from(0xF1), Err(0xF1));
    }

    #[test]
    fn status_active_sensing() {
        assert_eq!(Status::try_from(0xFE), Ok(Status::ActiveSensing));
    }

    #[test]
    fn status_reset() {
        assert_eq!(Status::try_from(0xFF), Ok(Status::Reset));
    }

    #[test]
    fn status_sequence_continue() {
        assert_eq!(Status::try_from(0xFB), Ok(Status::SequenceContinue));
    }

    #[test]
    fn status_sequence_start() {
        assert_eq!(Status::try_from(0xFA), Ok(Status::SequenceStart));
    }

    #[test]
    fn status_sequence_stop() {
        assert_eq!(Status::try_from(0xFC), Ok(Status::SequenceStop));
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
    fn status_timing_clock() {
        assert_eq!(Status::try_from(0xF8), Ok(Status::TimingClock));
    }

    #[test]
    fn status_tune_request() {
        assert_eq!(Status::try_from(0xF6), Ok(Status::TuneRequest));
    }

    #[test]
    fn active_sensing() {
        assert_eq!(
            MessageSystem::from_bytes(Status::ActiveSensing, &[]),
            Ok((0, MessageSystem::ActiveSensing))
        );
    }

    #[test]
    fn reset() {
        assert_eq!(
            MessageSystem::from_bytes(Status::Reset, &[]),
            Ok((0, MessageSystem::Reset))
        );
    }

    #[test]
    fn sequence_continue() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SequenceContinue, &[]),
            Ok((0, MessageSystem::SequenceContinue))
        );
    }

    #[test]
    fn sequence_start() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SequenceStart, &[]),
            Ok((0, MessageSystem::SequenceStart))
        );
    }

    #[test]
    fn sequence_stop() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SequenceStop, &[]),
            Ok((0, MessageSystem::SequenceStop))
        );
    }

    #[test]
    fn song_position_pointer_missing_lsb() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SongPositionPointer, &[]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 0,
                expected: 2
            })
        );
    }

    #[test]
    fn song_position_pointer_missing_msb() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SongPositionPointer, &[0x01]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 1,
                expected: 2
            })
        );
    }

    #[test]
    fn song_position_pointer() {
        let lsb: u8 = 0x71;
        let msb = 0x02;
        let value = u16::from(msb) << 7 | u16::from(lsb);
        assert_eq!(
            MessageSystem::from_bytes(Status::SongPositionPointer, &[lsb, msb]),
            Ok((2, MessageSystem::SongPositionPointer(value)))
        );

        let lsb = 0xFF;
        let msb = 0xFF;
        let value = (u16::from(mask_data(msb))) << 7 | u16::from(mask_data(lsb));
        assert_eq!(
            MessageSystem::from_bytes(Status::SongPositionPointer, &[lsb, msb]),
            Ok((2, MessageSystem::SongPositionPointer(value)))
        );
    }

    #[test]
    fn song_select_missing_byte() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SongSelect, &[]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 0,
                expected: 1
            })
        );
    }

    #[test]
    fn song_select() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SongSelect, &[0x01]),
            Ok((1, MessageSystem::SongSelect(0x01)))
        );

        assert_eq!(
            MessageSystem::from_bytes(Status::SongSelect, &[0xFF]),
            Ok((1, MessageSystem::SongSelect(mask_data(0xFF))))
        );
    }

    #[test]
    fn sys_ex_end_before_start() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SysExEnd, &[]),
            Err(ParseError::SysExEndBeforeStart)
        );
    }

    #[test]
    fn sys_ex_end_missing() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SysExStart, &[]),
            Err(ParseError::SysExEndMissing)
        );
        assert_eq!(
            MessageSystem::from_bytes(Status::SysExStart, &[0x01]),
            Err(ParseError::SysExEndMissing)
        );
    }

    #[test]
    fn sys_ex() {
        assert_eq!(
            MessageSystem::from_bytes(Status::SysExStart, &[0x01, Status::SysExEnd.into()]),
            Ok((2, MessageSystem::SysEx(vec![0x01]))),
        );
        assert_eq!(
            MessageSystem::from_bytes(Status::SysExStart, &[0xFF, 0xFF, Status::SysExEnd.into()]),
            Ok((
                3,
                MessageSystem::SysEx(vec![mask_data(0xFF), mask_data(0xFF),])
            )),
        );
    }

    #[test]
    fn timing_clock() {
        assert_eq!(
            MessageSystem::from_bytes(Status::TimingClock, &[]),
            Ok((0, MessageSystem::TimingClock))
        );
    }

    #[test]
    fn tune_request() {
        assert_eq!(
            MessageSystem::from_bytes(Status::TuneRequest, &[]),
            Ok((0, MessageSystem::TuneRequest))
        );
    }
}
