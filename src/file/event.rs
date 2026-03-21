use std::fmt::{self, Display};

use crate::{
    message_channel_voice::{self, MessageChannelVoice},
    meta_event::{self, MetaEvent},
    variable_length_quantity::{self, variable_length_quantity, variable_length_quantity_usize},
};

#[derive(Debug, PartialEq)]
pub struct Event {
    pub tick_delta: u32,
    pub tick_absolute: u32,
    pub content: Content,
}

impl Event {
    pub fn from_bytes(
        bytes: &[u8],
        tick: u32,
        running_status: Option<message_channel_voice::Status>,
    ) -> Result<(usize, Self, Option<message_channel_voice::Status>), (usize, ParseError)> {
        let mut pos = 0;

        let (delta_time_consumed, tick_delta) = variable_length_quantity(bytes)
            .map_err(|err| (pos, ParseError::DeltaTimeInvalid(err)))?;
        pos += delta_time_consumed;

        let content_bytes = bytes.get(pos..).unwrap_or(&[]);
        // TODO: inline event content?
        let (content_consumed, content, running_status_new) =
            Content::from_bytes(content_bytes, running_status)?;
        pos += content_consumed;

        Ok((
            pos,
            Self {
                tick_delta,
                tick_absolute: tick + tick_delta,
                content,
            },
            running_status_new,
        ))
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    DeltaTimeInvalid(variable_length_quantity::ParseError),
    InputEmpty,
    MessageChannelVoiceInvalid(message_channel_voice::ParseError),
    MessageChannelVoiceStatusInvalid(u8),
    MetaEventInvalid(meta_event::ParseError),
    MetaEventTypeMissing,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::DeltaTimeInvalid(parse_error) => {
                write!(f, "delta time invalid: {parse_error}")
            }
            ParseError::InputEmpty => write!(f, "empty input"),
            ParseError::MessageChannelVoiceInvalid(parse_error) => {
                write!(f, "channel voice message is invalid: {parse_error}")
            }
            ParseError::MessageChannelVoiceStatusInvalid(status) => {
                write!(
                    f,
                    "status '{status}' is not valid for channel voice message"
                )
            }
            ParseError::MetaEventInvalid(parse_error) => {
                write!(f, "meta event is invalid: {parse_error}")
            }
            ParseError::MetaEventTypeMissing => write!(f, "meta event event type is missing"),
        }
    }
}

impl From<message_channel_voice::ParseError> for ParseError {
    fn from(value: message_channel_voice::ParseError) -> Self {
        Self::MessageChannelVoiceInvalid(value)
    }
}

impl From<meta_event::ParseError> for ParseError {
    fn from(value: meta_event::ParseError) -> Self {
        Self::MetaEventInvalid(value)
    }
}

#[derive(Debug, PartialEq)]
pub enum Content {
    ChannelVoice {
        is_running_status: bool,
        message: MessageChannelVoice,
    },
    MetaEvent(MetaEvent),
    // TODO: Sysex
}

impl From<MetaEvent> for Content {
    fn from(value: MetaEvent) -> Self {
        Self::MetaEvent(value)
    }
}

impl Content {
    pub fn from_bytes(
        bytes: &[u8],
        running_status: Option<message_channel_voice::Status>,
    ) -> Result<(usize, Self, Option<message_channel_voice::Status>), (usize, ParseError)> {
        let mut pos = 0;
        let status_byte = bytes.get(pos).ok_or((pos, ParseError::InputEmpty))?;

        if status_byte == &meta_event::STATUS {
            pos += 1;

            return Self::from_bytes_meta_event(bytes, pos);
        }

        Self::from_bytes_message_channel_voice(bytes, pos, status_byte, running_status)
    }

    fn from_bytes_meta_event(
        bytes: &[u8],
        mut pos: usize,
    ) -> Result<(usize, Self, Option<message_channel_voice::Status>), (usize, ParseError)> {
        let typ = meta_event::Type::from(
            *bytes
                .get(pos)
                .ok_or((pos, ParseError::MetaEventTypeMissing))?,
        );
        pos += 1;

        let (consumed_length, length) =
        // TODO unwrap
        variable_length_quantity_usize(bytes.get(pos..).unwrap_or(&[])).unwrap();
        pos += consumed_length;

        let meta_event_bytes = bytes.get(pos..pos + length).unwrap_or(&[]);
        let (consumed_meta_event, meta_event) =
            MetaEvent::from_bytes(typ, length, meta_event_bytes)
                .map_err(|err| (pos, err.into()))?;
        pos += consumed_meta_event;

        Ok((pos, meta_event.into(), None))
    }

    fn from_bytes_message_channel_voice(
        bytes: &[u8],
        mut pos: usize,
        status_byte: &u8,
        running_status: Option<message_channel_voice::Status>,
    ) -> Result<(usize, Self, Option<message_channel_voice::Status>), (usize, ParseError)> {
        let (status, is_running_status) =
            match message_channel_voice::Status::try_from(*status_byte) {
                Ok(status) => (status, false),
                Err(err) => match running_status {
                    Some(status) => (status, true),
                    None => {
                        return Err((pos, ParseError::MessageChannelVoiceStatusInvalid(err)));
                    }
                },
            };

        if !is_running_status {
            pos += 1;
        }

        let content_bytes = bytes.get(pos..).unwrap_or(&[]);

        let (consumed_message, message) = MessageChannelVoice::from_bytes(status, content_bytes)
            .map_err(|err| (pos, err.into()))?;
        pos += consumed_message;

        Ok((
            pos,
            Content::ChannelVoice {
                is_running_status,
                message,
            },
            Some(status),
        ))
    }
}

impl Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Content::ChannelVoice {
                    is_running_status,
                    message,
                } => format!(
                    "{}{}",
                    message,
                    match is_running_status {
                        true => " - r",
                        false => "",
                    }
                ),
                Content::MetaEvent(meta_event) => format!("meta event {}", meta_event),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        file::event::{Content, Event, ParseError},
        message_channel_voice::{self, MessageChannelVoice},
        meta_event::{self, MetaEvent},
        note::{HalfTone, Note, PitchClass},
        variable_length_quantity,
    };

    #[test]
    fn content_channel_note_off() {
        let result = Content::from_bytes(
            &[message_channel_voice::Status::NoteOff.into(), 0x3C, 0x01],
            None,
        );

        assert_eq!(
            result,
            Ok((
                3,
                Content::ChannelVoice {
                    is_running_status: false,
                    message: MessageChannelVoice::Off {
                        note: Note::from_parts(PitchClass::C, 4, None),
                        velocity: 1,
                    }
                },
                Some(message_channel_voice::Status::NoteOff)
            ))
        );
    }

    #[test]
    fn content_channel_note_on() {
        let result = Content::from_bytes(
            &[message_channel_voice::Status::NoteOn.into(), 0x4E, 0x02],
            None,
        );

        assert_eq!(
            result,
            Ok((
                3,
                Content::ChannelVoice {
                    is_running_status: false,
                    message: MessageChannelVoice::On {
                        note: Note::from_parts(PitchClass::F, 5, Some(HalfTone::Sharp)),
                        velocity: 2,
                    }
                },
                Some(message_channel_voice::Status::NoteOn),
            ))
        );
    }

    #[test]
    fn content_channel_pitch_wheel_change() {
        let result = Content::from_bytes(
            &[
                message_channel_voice::Status::PitchWheelChange.into(),
                0x12,
                0x11,
            ],
            None,
        );

        assert_eq!(
            result,
            Ok((
                3,
                Content::ChannelVoice {
                    is_running_status: false,
                    message: MessageChannelVoice::PitchWheelChange(2194),
                },
                Some(message_channel_voice::Status::PitchWheelChange)
            ))
        );
    }

    #[test]
    fn content_channel_polyphonic_key_pressure() {
        let result = Content::from_bytes(
            &[message_channel_voice::Status::Polyphonic.into(), 0x7F, 0x7C],
            None,
        );

        assert_eq!(
            result,
            Ok((
                3,
                Content::ChannelVoice {
                    is_running_status: false,
                    message: MessageChannelVoice::PolyphonicKeyPressure {
                        note: Note::from_parts(PitchClass::G, 9, None),
                        value: 124,
                    }
                },
                Some(message_channel_voice::Status::Polyphonic),
            ))
        );
    }

    #[test]
    fn content_channel_program_change() {
        let result = Content::from_bytes(
            &[message_channel_voice::Status::ProgramChange.into(), 0x0C],
            None,
        );

        assert_eq!(
            result,
            Ok((
                2,
                Content::ChannelVoice {
                    is_running_status: false,
                    message: MessageChannelVoice::ProgramChange(12),
                },
                Some(message_channel_voice::Status::ProgramChange)
            ))
        );
    }

    #[test]
    fn content_event_meta_type_missing() {
        let result = Content::from_bytes(&[0xFF], None);
        assert_eq!(result, Err((1, ParseError::MetaEventTypeMissing)));
    }

    #[test]
    fn content_event_meta() {
        assert_eq!(
            Content::from_bytes(&[0xFF, meta_event::Type::EndOfTrack.into(), 0x00], None),
            Ok((3, Content::MetaEvent(MetaEvent::EndOfTrack), None))
        );
        assert_eq!(
            Content::from_bytes(
                &[0xFF, meta_event::Type::ChannelPrefix.into(), 0x01, 0x08],
                None
            ),
            Ok((4, Content::MetaEvent(MetaEvent::ChannelPrefix(8)), None))
        );
    }

    #[test]
    fn running_status() {
        assert_eq!(
            Content::from_bytes(
                &[0x0a, 0x40, 00],
                Some(message_channel_voice::Status::ControlChange)
            ),
            Ok((
                2,
                Content::ChannelVoice {
                    is_running_status: true,
                    message: MessageChannelVoice::ControlChange {
                        controller: 10,
                        value: 64
                    }
                },
                Some(message_channel_voice::Status::ControlChange)
            ))
        );
    }

    #[test]
    fn content_missing() {
        assert_eq!(
            Content::from_bytes(&[], None),
            Err((0, ParseError::InputEmpty))
        );
    }

    #[test]
    fn content_channel_pressure() {
        let result =
            Content::from_bytes(&[message_channel_voice::Status::Pressure.into(), 0x3], None);

        assert_eq!(
            result,
            Ok((
                2,
                Content::ChannelVoice {
                    is_running_status: false,
                    message: MessageChannelVoice::ChannelPressure(3)
                },
                Some(message_channel_voice::Status::Pressure)
            ))
        );
    }

    #[test]
    fn input_empty() {
        assert_eq!(
            Event::from_bytes(&[], 0, None),
            Err((
                0,
                ParseError::DeltaTimeInvalid(variable_length_quantity::ParseError::InputEmpty)
            ))
        )
    }

    #[test]
    fn delta_time_missing() {
        assert_eq!(
            Event::from_bytes(&[], 0, None),
            Err((
                0,
                ParseError::DeltaTimeInvalid(variable_length_quantity::ParseError::InputEmpty)
            ))
        );
    }

    #[test]
    fn event_meta_end_of_track() {
        assert_eq!(
            Event::from_bytes(
                &[
                    0x00,
                    meta_event::STATUS,
                    meta_event::Type::EndOfTrack.into(),
                    0x00,
                ],
                0,
                None
            ),
            Ok((
                4,
                Event {
                    tick_absolute: 0,
                    tick_delta: 0,
                    content: MetaEvent::EndOfTrack.into()
                },
                None
            ))
        );
        assert_eq!(
            Event::from_bytes(
                &[
                    0x05,
                    meta_event::STATUS,
                    meta_event::Type::EndOfTrack.into(),
                    0x00,
                ],
                0,
                None
            ),
            Ok((
                4,
                Event {
                    tick_absolute: 5,
                    tick_delta: 5,
                    content: MetaEvent::EndOfTrack.into()
                },
                None
            ))
        );
    }
}
