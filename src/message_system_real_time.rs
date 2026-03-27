//! MIDI System Real-Time messages (status bytes `0xF8..=0xFF`).
//!
//! These are single-byte messages used for timing and synchronization. They carry
//! no data bytes and can be interleaved anywhere in a MIDI stream, even between
//! the status and data bytes of another message.

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Status {
    ActiveSensing = 0xFE,
    Reset = 0xFF,
    SequenceContinue = 0xFB,
    SequenceStart = 0xFA,
    SequenceStop = 0xFC,
    TimingClock = 0xF8,
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
            v if v == Status::ActiveSensing.into() => Status::ActiveSensing,
            v if v == Status::Reset.into() => Status::Reset,
            v if v == Status::SequenceContinue.into() => Status::SequenceContinue,
            v if v == Status::SequenceStart.into() => Status::SequenceStart,
            v if v == Status::SequenceStop.into() => Status::SequenceStop,
            v if v == Status::TimingClock.into() => Status::TimingClock,
            undefined => return Err(undefined),
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum Message {
    ActiveSensing,
    Reset,
    SequenceContinue,
    SequenceStart,
    SequenceStop,
    TimingClock,
}

impl Message {
    pub fn from_bytes(status: Status) -> (usize, Self) {
        match status {
            Status::ActiveSensing => (0, Self::ActiveSensing),
            Status::Reset => (0, Self::Reset),
            Status::SequenceContinue => (0, Self::SequenceContinue),
            Status::SequenceStart => (0, Self::SequenceStart),
            Status::SequenceStop => (0, Self::SequenceStop),
            Status::TimingClock => (0, Self::TimingClock),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::message_system_real_time::{Message, Status};

    #[test]
    fn status_undefined() {
        assert_eq!(Status::try_from(0xF0), Err(0xF0));
        assert_eq!(Status::try_from(0xF9), Err(0xF9));
        assert_eq!(Status::try_from(0xFD), Err(0xFD));
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
    fn status_timing_clock() {
        assert_eq!(Status::try_from(0xF8), Ok(Status::TimingClock));
    }

    #[test]
    fn active_sensing() {
        assert_eq!(
            Message::from_bytes(Status::ActiveSensing),
            (0, Message::ActiveSensing)
        );
    }

    #[test]
    fn reset() {
        assert_eq!(Message::from_bytes(Status::Reset), (0, Message::Reset));
    }

    #[test]
    fn sequence_continue() {
        assert_eq!(
            Message::from_bytes(Status::SequenceContinue),
            (0, Message::SequenceContinue)
        );
    }

    #[test]
    fn sequence_start() {
        assert_eq!(
            Message::from_bytes(Status::SequenceStart),
            (0, Message::SequenceStart)
        );
    }

    #[test]
    fn sequence_stop() {
        assert_eq!(
            Message::from_bytes(Status::SequenceStop),
            (0, Message::SequenceStop)
        );
    }

    #[test]
    fn timing_clock() {
        assert_eq!(
            Message::from_bytes(Status::TimingClock),
            (0, Message::TimingClock)
        );
    }
}
