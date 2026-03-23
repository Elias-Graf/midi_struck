use std::fmt::Display;

use crate::note::Note;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    Pressure,
    ControlChange,
    NoteOff,
    NoteOn,
    PitchWheelChange,
    Polyphonic,
    ProgramChange,
}

impl TryFrom<u8> for Status {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value & 0xF0 {
            0b1101_0000 => Self::Pressure,
            0b1000_0000 => Self::NoteOff,
            0b1001_0000 => Self::NoteOn,
            0b1011_0000 => Self::ControlChange,
            0b1110_0000 => Self::PitchWheelChange,
            0b1100_0000 => Self::ProgramChange,
            0b1010_0000 => Self::Polyphonic,
            unknown_status => return Err(unknown_status),
        })
    }
}

impl From<Status> for u8 {
    fn from(value: Status) -> Self {
        match value {
            Status::Pressure => 0b1101_0000,
            Status::NoteOff => 0b1000_0000,
            Status::NoteOn => 0b1001_0000,
            Status::ControlChange => 0b1011_0000,
            Status::PitchWheelChange => 0b1110_0000,
            Status::ProgramChange => 0b1100_0000,
            Status::Polyphonic => 0b1010_0000,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MessageChannelVoice {
    ChannelPressure(u8),
    ControlChange { controller: u8, value: u8 },
    Off { note: Note, velocity: u8 },
    On { note: Note, velocity: u8 },
    PitchWheelChange(u16),
    PolyphonicKeyPressure { note: Note, value: u8 },
    ProgramChange(u8),
}

impl MessageChannelVoice {
    pub fn from_bytes(status: Status, bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        match status {
            Status::Pressure => Self::channel_pressure_content(bytes),
            Status::ControlChange => Self::control_change_content(bytes),
            Status::NoteOff => {
                let (consumed, note, velocity) = Self::note_content(status, bytes)?;
                Ok((consumed, MessageChannelVoice::Off { note, velocity }))
            }
            Status::NoteOn => {
                let (consumed, note, velocity) = Self::note_content(status, bytes)?;
                Ok((consumed, MessageChannelVoice::On { note, velocity }))
            }
            Status::PitchWheelChange => Self::pitch_wheel_change_content(bytes),
            Status::Polyphonic => Self::polyphonic_key_pressure(bytes),
            Status::ProgramChange => Self::program_change_content(bytes),
        }
    }

    fn channel_pressure_content(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        Ok((
            1,
            Self::ChannelPressure(mask_data(
                *bytes
                    .first()
                    .ok_or(ParseError::ValueMissing(Status::Pressure))?,
            )),
        ))
    }

    fn control_change_content(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let controller = bytes
            .first()
            .map(|byte| mask_data(*byte))
            .ok_or(ParseError::ControlChangeControllerMissing)?;
        let value = bytes
            .get(1)
            .map(|byte| mask_data(*byte))
            .ok_or(ParseError::ValueMissing(Status::ControlChange))?;

        Ok((2, Self::ControlChange { controller, value }))
    }

    fn note_content(status: Status, bytes: &[u8]) -> Result<(usize, Note, u8), ParseError> {
        let note = match bytes.first() {
            Some(n) => (*n).into(),
            None => return Err(ParseError::NoteMissing(status)),
        };
        let velocity = match bytes.get(1) {
            Some(v) => mask_data(*v),
            None => {
                return Err(ParseError::VelocityMissing(status, note));
            }
        };

        Ok((2, note, velocity))
    }

    fn pitch_wheel_change_content(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        // Per specification the the first bit is the least significant bit for this message.
        let lsb = *bytes.first().ok_or(ParseError::DataNotEnoughBytes {
            available: 0,
            length: 2,
        })?;
        let msb = *bytes.get(1).ok_or(ParseError::DataNotEnoughBytes {
            available: 1,
            length: 2,
        })?;
        let value = (u16::from(mask_data(msb))) << 7 | u16::from(mask_data(lsb));

        Ok((2, Self::PitchWheelChange(value)))
    }

    fn polyphonic_key_pressure(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let note = bytes
            .first()
            .map(|byte| Note::from(*byte))
            .ok_or(ParseError::NoteMissing(Status::Polyphonic))?;
        let value = bytes
            .get(1)
            .map(|byte| mask_data(*byte))
            .ok_or(ParseError::ValueMissing(Status::Polyphonic))?;

        Ok((2, Self::PolyphonicKeyPressure { note, value }))
    }

    fn program_change_content(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        Ok((
            1,
            Self::ProgramChange(mask_data(*bytes.first().ok_or(ParseError::ProgramMissing)?)),
        ))
    }
}

impl Display for MessageChannelVoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MessageChannelVoice::ChannelPressure(pressure) =>
                    format!("channel pressure {}", pressure),
                MessageChannelVoice::ControlChange { controller, value } =>
                    format!("control change for controller {}: {}", controller, value),
                MessageChannelVoice::Off { note, velocity } =>
                    format!("note off: {}, velocity: {}", note, velocity),
                MessageChannelVoice::On { note, velocity } =>
                    format!("note on: {}, velocity: {}", note, velocity),
                MessageChannelVoice::PitchWheelChange(pitch_wheel) =>
                    format!("pitch wheel: {}", pitch_wheel),
                MessageChannelVoice::PolyphonicKeyPressure { note, value } =>
                    format!("polyphonic key pressure {}: {}", note, value),
                MessageChannelVoice::ProgramChange(program) =>
                    format!("program change: {}", program),
            }
        )
    }
}

// TODO: move out of this file and ensure is used in any message.
pub fn mask_data(byte: u8) -> u8 {
    // Specification demands the first bit be always zero, so we simply mask it off.
    byte & 0x7F
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    ControlChangeControllerMissing,
    DataNotEnoughBytes { available: usize, length: usize },
    NoteMissing(Status),
    ValueMissing(Status),
    ProgramMissing,
    VelocityMissing(Status, Note),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::ControlChangeControllerMissing => {
                write!(f, "'control change' is missing target controller")
            }
            ParseError::DataNotEnoughBytes { available, length } => write!(
                f,
                "length not enough bytes: available: {available}, expected length: {length}"
            ),
            ParseError::NoteMissing(status) => write!(f, "note for status '{status:?}' is missing"),
            ParseError::ValueMissing(status) => {
                write!(f, "value for status: '{status:?}' is missing")
            }
            ParseError::ProgramMissing => write!(f, "program is missing"),
            ParseError::VelocityMissing(status, note) => {
                write!(
                    f,
                    "velocity is missing for status: '{status:?}', note: {note}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        message_channel_voice::{MessageChannelVoice, ParseError, Status, mask_data},
        note::{Note, PitchClass},
    };

    #[test]
    fn status_pressure() {
        assert_eq!(Status::try_from(0b1101_0000), Ok(Status::Pressure));
        assert_eq!(Status::try_from(0b1101_0001), Ok(Status::Pressure));
    }

    #[test]
    fn status_note_off() {
        assert_eq!(Status::try_from(0b1000_0000), Ok(Status::NoteOff));
        assert_eq!(Status::try_from(0b1000_0001), Ok(Status::NoteOff));
    }

    #[test]
    fn status_note_on() {
        assert_eq!(Status::try_from(0b1001_0000), Ok(Status::NoteOn));
        assert_eq!(Status::try_from(0b1001_0001), Ok(Status::NoteOn));
    }

    #[test]
    fn status_control_change() {
        assert_eq!(Status::try_from(0b1011_0000), Ok(Status::ControlChange));
        assert_eq!(Status::try_from(0b1011_0000), Ok(Status::ControlChange));
    }

    #[test]
    fn status_pitch_wheel_change() {
        assert_eq!(Status::try_from(0b1110_0000), Ok(Status::PitchWheelChange));
        assert_eq!(Status::try_from(0b1110_0000), Ok(Status::PitchWheelChange));
    }

    #[test]
    fn status_program_change() {
        assert_eq!(Status::try_from(0b1100_0000), Ok(Status::ProgramChange));
        assert_eq!(Status::try_from(0b1100_0001), Ok(Status::ProgramChange));
    }

    #[test]
    fn status_polyphonic() {
        assert_eq!(Status::try_from(0b1010_0000), Ok(Status::Polyphonic));
    }

    #[test]
    fn status_unknown() {
        assert_eq!(Status::try_from(0b1111_1111), Err(0xF0));
    }

    #[test]
    fn channel_pressure_value_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::Pressure, &[]),
            Err(ParseError::ValueMissing(Status::Pressure))
        );
    }

    #[test]
    fn channel_pressure() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::Pressure, &[0x01]),
            Ok((1, MessageChannelVoice::ChannelPressure(0x01)))
        );
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::Pressure, &[0xFF]),
            Ok((1, MessageChannelVoice::ChannelPressure(mask_data(0xFF))))
        );
    }

    #[test]
    fn control_change_controller_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::ControlChange, &[]),
            Err(ParseError::ControlChangeControllerMissing)
        );
    }

    #[test]
    fn control_change_value_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::ControlChange, &[0x01]),
            Err(ParseError::ValueMissing(Status::ControlChange))
        );
    }

    #[test]
    fn control_change() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::ControlChange, &[0x01, 0x02]),
            Ok((
                2,
                MessageChannelVoice::ControlChange {
                    controller: 0x01,
                    value: 0x02
                }
            ))
        );
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::ControlChange, &[0xFF, 0xFF]),
            Ok((
                2,
                MessageChannelVoice::ControlChange {
                    controller: mask_data(0xFF),
                    value: mask_data(0xFF)
                }
            ))
        );
    }

    #[test]
    fn off_note_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOff, &[]),
            Err(ParseError::NoteMissing(Status::NoteOff))
        );
    }

    #[test]
    fn off_velocity_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOff, &[0x3C]),
            Err(ParseError::VelocityMissing(
                Status::NoteOff,
                Note::from_parts(PitchClass::C, 4)
            ))
        );
    }

    #[test]
    fn off() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOff, &[0x3E, 0x01]),
            Ok((
                2,
                MessageChannelVoice::Off {
                    note: Note::from_parts(PitchClass::D, 4),
                    velocity: 0x01
                },
            ))
        );
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOff, &[0x42, 0xFF]),
            Ok((
                2,
                MessageChannelVoice::Off {
                    note: Note::from_parts(PitchClass::FSharp, 4),
                    velocity: mask_data(0xFF)
                }
            ))
        );
    }

    #[test]
    fn on_note_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOn, &[]),
            Err(ParseError::NoteMissing(Status::NoteOn))
        );
    }

    #[test]
    fn on_velocity_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOff, &[0x5C]),
            Err(ParseError::VelocityMissing(
                Status::NoteOff,
                Note::from_parts(PitchClass::GSharp, 6)
            ))
        );
    }

    #[test]
    fn on() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOn, &[0x40, 0x01]),
            Ok((
                2,
                MessageChannelVoice::On {
                    note: Note::from_parts(PitchClass::E, 4),
                    velocity: 0x01
                }
            ))
        );
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::NoteOn, &[0x12, 0xFF]),
            Ok((
                2,
                MessageChannelVoice::On {
                    note: Note::from_parts(PitchClass::FSharp, 0),
                    velocity: mask_data(0xFF)
                }
            ))
        );
    }

    #[test]
    fn pitch_wheel_change_missing_lsb() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::PitchWheelChange, &[]),
            Err(ParseError::DataNotEnoughBytes {
                available: 0,
                length: 2
            })
        );
    }

    #[test]
    fn pitch_wheel_change_missing_msb() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::PitchWheelChange, &[0x01]),
            Err(ParseError::DataNotEnoughBytes {
                available: 1,
                length: 2
            })
        );
    }

    #[test]
    fn pitch_wheel_change() {
        let lsb: u8 = 0x71;
        let msb = 0x02;
        let value = u16::from(msb) << 7 | u16::from(lsb);
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::PitchWheelChange, &[lsb, msb]),
            Ok((2, MessageChannelVoice::PitchWheelChange(value)))
        );

        let lsb = 0xFF;
        let msb = 0xFF;
        let value = (u16::from(mask_data(msb))) << 7 | u16::from(mask_data(lsb));
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::PitchWheelChange, &[lsb, msb]),
            Ok((2, MessageChannelVoice::PitchWheelChange(value)))
        );
    }

    #[test]
    fn polyphonic_key_pressure_note_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::Polyphonic, &[]),
            Err(ParseError::NoteMissing(Status::Polyphonic)),
        );
    }

    #[test]
    fn polyphonic_key_pressure_value_missing() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::Polyphonic, &[0xAB]),
            Err(ParseError::ValueMissing(Status::Polyphonic)),
        );
    }

    #[test]
    fn polyphonic_key_pressure() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::Polyphonic, &[0x7F, 0x64]),
            Ok((
                2,
                MessageChannelVoice::PolyphonicKeyPressure {
                    note: Note::from_parts(PitchClass::G, 9),
                    value: 0x64
                }
            )),
        );
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::Polyphonic, &[0x7F, 0xFF]),
            Ok((
                2,
                MessageChannelVoice::PolyphonicKeyPressure {
                    note: Note::from_parts(PitchClass::G, 9),
                    value: mask_data(0xFF)
                }
            )),
        );
    }

    #[test]
    fn program_change_missing_program() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::ProgramChange, &[]),
            Err(ParseError::ProgramMissing)
        );
    }

    #[test]
    fn program_change() {
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::ProgramChange, &[0x01]),
            Ok((1, MessageChannelVoice::ProgramChange(0x01)))
        );
        assert_eq!(
            MessageChannelVoice::from_bytes(Status::ProgramChange, &[0xFF]),
            Ok((1, MessageChannelVoice::ProgramChange(mask_data(0xFF))))
        );
    }
}
