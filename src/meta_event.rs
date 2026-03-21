use std::{
    fmt::{Debug, Display},
    str::Utf8Error,
};

use crate::SliceGetFixed;

pub const STATUS: u8 = 0xFF;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Type {
    ChannelPrefix,
    Copyright,
    CuePoint,
    EndOfTrack,
    InstrumentName,
    KeySignature,
    Lyric,
    Marker,
    SequenceNumber,
    SequenceTrackName,
    SequencerSpecific,
    SetTempo,
    SmpteOffset,
    Text,
    TimeSignature,
    Unknown(u8),
}
impl From<u8> for Type {
    fn from(value: u8) -> Self {
        match value {
            v if v == Self::ChannelPrefix.into() => Self::ChannelPrefix,
            v if v == Self::CuePoint.into() => Self::CuePoint,
            v if v == Self::Copyright.into() => Self::Copyright,
            v if v == Self::EndOfTrack.into() => Self::EndOfTrack,
            v if v == Self::InstrumentName.into() => Self::InstrumentName,
            v if v == Self::KeySignature.into() => Self::KeySignature,
            v if v == Self::Lyric.into() => Self::Lyric,
            v if v == Self::Marker.into() => Self::Marker,
            v if v == Self::SequenceNumber.into() => Self::SequenceNumber,
            v if v == Self::SequenceTrackName.into() => Self::SequenceTrackName,
            v if v == Self::SequencerSpecific.into() => Self::SequencerSpecific,
            v if v == Self::SetTempo.into() => Self::SetTempo,
            v if v == Self::SmpteOffset.into() => Self::SmpteOffset,
            v if v == Self::Text.into() => Self::Text,
            v if v == Self::TimeSignature.into() => Self::TimeSignature,
            unknown => Self::Unknown(unknown),
        }
    }
}

impl From<Type> for u8 {
    fn from(value: Type) -> Self {
        match value {
            Type::ChannelPrefix => 0x20,
            Type::Copyright => 0x02,
            Type::CuePoint => 0x07,
            Type::EndOfTrack => 0x2F,
            Type::InstrumentName => 0x04,
            Type::KeySignature => 0x59,
            Type::Lyric => 0x05,
            Type::Marker => 0x06,
            Type::SequenceNumber => 0x00,
            Type::SequenceTrackName => 0x03,
            Type::SequencerSpecific => 0x7F,
            Type::SetTempo => 0x51,
            Type::SmpteOffset => 0x54,
            Type::Text => 0x01,
            Type::TimeSignature => 0x58,
            Type::Unknown(v) => v,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MetaEvent {
    ChannelPrefix(u8),
    Copyright(String),
    CuePoint(String),
    EndOfTrack,
    InstrumentName(String),
    KeySignature {
        key: i8,
        is_minor: bool,
    },
    Lyric(String),
    Marker(String),
    SequenceNumber(u16),
    SequenceTrackName(String),
    SequencerSpecific(Vec<u8>),
    SetTempo(u32),
    SmpteOffset {
        hours: u8,
        minute: u8,
        second: u8,
        frame_rate: u8,
        fractional_frames: u8,
    },
    Text(String),
    TimeSignature {
        numerator: u8,
        negative_denominator_power: u8,
        clock_per_metronome_click: u8,
        thirtyseconds_per_quarter_note: u8,
    },
    Unknown(Vec<u8>),
}

impl MetaEvent {
    pub fn from_bytes(typ: Type, length: usize, bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let variant = match typ {
            Type::ChannelPrefix => Self::channel_prefix_content(bytes, length)?,
            Type::Copyright => Self::Copyright(Self::string_content(bytes, length)?),
            Type::CuePoint => Self::CuePoint(Self::string_content(bytes, length)?),
            Type::EndOfTrack => Self::EndOfTrack,
            Type::InstrumentName => Self::InstrumentName(Self::string_content(bytes, length)?),
            Type::KeySignature => Self::key_signature_content(bytes, length)?,
            Type::Lyric => Self::Lyric(Self::string_content(bytes, length)?),
            Type::Marker => Self::Marker(Self::string_content(bytes, length)?),
            Type::SequenceNumber => Self::sequence_number_content(bytes, length)?,
            Type::SequenceTrackName => {
                Self::SequenceTrackName(Self::string_content(bytes, length)?)
            }
            Type::SequencerSpecific => Self::sequencer_specific_content(bytes, length)?,
            Type::SetTempo => Self::set_tempo_content(bytes, length)?,
            Type::SmpteOffset => Self::smpte_offset_content(bytes, length)?,
            Type::Text => Self::Text(Self::string_content(bytes, length)?),
            Type::TimeSignature => Self::time_signature_content(bytes, length)?,
            Type::Unknown(_) => Self::unknown_content(bytes, length)?,
        };

        Ok((length, variant))
    }

    fn channel_prefix_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let channel_prefix = Self::fixed_bytes_content::<1>(bytes, length)?;

        Ok(Self::ChannelPrefix(channel_prefix[0]))
    }

    fn key_signature_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let key_signature_bytes = Self::fixed_bytes_content::<2>(bytes, length)?;

        Ok(Self::KeySignature {
            key: key_signature_bytes[0].cast_signed(),
            is_minor: key_signature_bytes[1] > 0,
        })
    }

    fn sequence_number_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let sequence_number_bytes = Self::fixed_bytes_content::<2>(bytes, length)?;

        Ok(Self::SequenceNumber(u16::from_be_bytes(
            *sequence_number_bytes,
        )))
    }

    fn sequencer_specific_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let sequencer_specific_bytes =
            bytes
                .get(..length)
                .ok_or(ParseError::LengthNotEnoughBytes {
                    actual: bytes.len(),
                    expected: length,
                })?;

        Ok(Self::SequencerSpecific(sequencer_specific_bytes.to_owned()))
    }

    fn set_tempo_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let tempo_bytes = Self::fixed_bytes_content::<3>(bytes, length)?;

        Ok(Self::SetTempo(u32::from_be_bytes([
            0x00,
            tempo_bytes[0],
            tempo_bytes[1],
            tempo_bytes[2],
        ])))
    }

    fn smpte_offset_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let offset_bytes = Self::fixed_bytes_content::<5>(bytes, length)?;

        Ok(Self::SmpteOffset {
            hours: offset_bytes[0],
            minute: offset_bytes[1],
            second: offset_bytes[2],
            frame_rate: offset_bytes[3],
            fractional_frames: offset_bytes[4],
        })
    }

    fn time_signature_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let time_signature_bytes = Self::fixed_bytes_content::<4>(bytes, length)?;

        Ok(Self::TimeSignature {
            numerator: time_signature_bytes[0],
            negative_denominator_power: time_signature_bytes[1],
            clock_per_metronome_click: time_signature_bytes[2],
            thirtyseconds_per_quarter_note: time_signature_bytes[3],
        })
    }

    fn unknown_content(bytes: &[u8], length: usize) -> Result<Self, ParseError> {
        let unknown_bytes = bytes
            .get(..length)
            .ok_or(ParseError::LengthNotEnoughBytes {
                actual: bytes.len(),
                expected: length,
            })?;

        Ok(Self::Unknown(unknown_bytes.to_vec()))
    }

    /// `N` is the expected length.
    fn fixed_bytes_content<const N: usize>(
        bytes: &[u8],
        actual_length: usize,
    ) -> Result<&[u8; N], ParseError> {
        if actual_length != N {
            return Err(ParseError::LengthNotExpected {
                actual: actual_length,
                expected: N,
            });
        }

        bytes
            .get_fixed::<N>(0)
            .ok_or(ParseError::LengthNotEnoughBytes {
                actual: bytes.len(),
                expected: N,
            })
    }

    fn string_content(bytes: &[u8], length: usize) -> Result<String, ParseError> {
        let instrument_name_bytes =
            bytes
                .get(..length)
                .ok_or(ParseError::LengthNotEnoughBytes {
                    actual: bytes.len(),
                    expected: length,
                })?;

        Ok(str::from_utf8(instrument_name_bytes)?.to_owned())
    }
}

impl Display for MetaEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ChannelPrefix(value) => format!("channel prefix: {}", value),
                Self::Copyright(value) => format!("copyright: {}", value),
                Self::CuePoint(value) => format!("cue point: {}", value),
                Self::EndOfTrack => "end of track".to_owned(),
                Self::InstrumentName(value) => format!("instrument name: {}", value),
                Self::KeySignature { key, is_minor } =>
                    format!("key signature: {:?}, is minor: {}", key, is_minor),
                Self::Lyric(value) => format!("lyric: {}", value),
                Self::Marker(marker) => format!("marker: {}", marker),
                Self::SequenceNumber(value) => format!("sequence number: {}", value),
                Self::SequenceTrackName(value) => format!("sequence/track name: {}", value),
                Self::SequencerSpecific(bytes) =>
                    format!("sequencer specific: {} bytes", bytes.len()),
                Self::SetTempo(value) => format!("set tempo: {}", value),
                Self::SmpteOffset {
                    hours,
                    minute,
                    second,
                    frame_rate,
                    fractional_frames,
                } => format!(
                    "smpte offset: {}h {}m {}s, frame rate: {}, fractional frames: {}",
                    hours, minute, second, frame_rate, fractional_frames
                ),
                Self::Text(value) => format!("text: {}", value),
                Self::TimeSignature {
                    numerator,
                    negative_denominator_power,
                    clock_per_metronome_click,
                    thirtyseconds_per_quarter_note,
                } => format!(
                    "time signature: {}/{}, clocks/tick: {}, thirtyseconds per quarter: {}",
                    numerator,
                    2_u8.pow((*negative_denominator_power).into()),
                    clock_per_metronome_click,
                    thirtyseconds_per_quarter_note
                ),
                Self::Unknown(items) => format!(
                    "unknown: {}",
                    items
                        .iter()
                        .map(|i| format!("{}", i))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    LengthNotEnoughBytes { actual: usize, expected: usize },
    LengthNotExpected { actual: usize, expected: usize },
    TextInvalid(Utf8Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // TODO: evaluate below comment for all variants.
            // TODO: evaluate if more context needs to be added to the errors, or if they are
            // incorrect entirely. E.g. LengthNotEnoughBytes vs ContentLengthUnexpected, or
            // TextInvalid vs SomeEventTextInvalid.
            ParseError::LengthNotEnoughBytes { actual, expected } => write!(
                f,
                "length not enough bytes, expeced: {expected}, actual: {actual}"
            ),
            ParseError::LengthNotExpected { actual, expected } => {
                write!(
                    f,
                    "unexpected length, expected: {expected}, actual: {actual}"
                )
            }
            ParseError::TextInvalid(utf8_error) => write!(f, "received invalid text: {utf8_error}"),
        }
    }
}

impl From<Utf8Error> for ParseError {
    fn from(value: Utf8Error) -> Self {
        Self::TextInvalid(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::meta_event::{MetaEvent, ParseError, Type};

    #[test]
    fn type_channel_prefix() {
        assert_eq!(Type::from(0x20), Type::ChannelPrefix);
    }

    #[test]
    fn type_copyright() {
        assert_eq!(Type::from(0x02), Type::Copyright);
    }

    #[test]
    fn type_cue_point() {
        assert_eq!(Type::from(0x07), Type::CuePoint);
    }

    #[test]
    fn type_end_of_track() {
        assert_eq!(Type::from(0x2F), Type::EndOfTrack);
    }

    #[test]
    fn type_instrument_name() {
        assert_eq!(Type::from(0x04), Type::InstrumentName);
    }

    #[test]
    fn type_key_signature() {
        assert_eq!(Type::from(0x59), Type::KeySignature);
    }

    #[test]
    fn type_lyric() {
        assert_eq!(Type::from(0x05), Type::Lyric);
    }

    #[test]
    fn type_marker() {
        assert_eq!(Type::from(0x06), Type::Marker);
    }

    #[test]
    fn type_sequence_number() {
        assert_eq!(Type::from(0x00), Type::SequenceNumber);
    }

    #[test]
    fn type_sequence_track_name() {
        assert_eq!(Type::from(0x03), Type::SequenceTrackName);
    }

    #[test]
    fn type_sequencer_specific() {
        assert_eq!(Type::from(0x7F), Type::SequencerSpecific);
    }

    #[test]
    fn type_set_tempo() {
        assert_eq!(Type::from(0x51), Type::SetTempo);
    }

    #[test]
    fn type_smpte_offset() {
        assert_eq!(Type::from(0x54), Type::SmpteOffset);
    }

    #[test]
    fn type_text() {
        assert_eq!(Type::from(0x01), Type::Text);
    }

    #[test]
    fn type_time_signature() {
        assert_eq!(Type::from(0x58), Type::TimeSignature);
    }

    #[test]
    fn type_unknown() {
        assert_eq!(Type::from(0xFF), Type::Unknown(0xFF));
        assert_eq!(Type::from(0xFE), Type::Unknown(0xFE));
    }

    #[test]
    #[ignore = "should be done in the parent"]
    fn length_missing() {
        todo!()
    }

    #[test]
    fn channel_prefix() {
        assert_eq!(
            MetaEvent::from_bytes(Type::ChannelPrefix, 1, &[0x05]),
            Ok((1, MetaEvent::ChannelPrefix(0x05)))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::ChannelPrefix, 1, &[0x06]),
            Ok((1, MetaEvent::ChannelPrefix(0x06)))
        );
    }

    #[test]
    fn copyright() {
        test_string_content(Type::Copyright, MetaEvent::Copyright);
    }

    #[test]
    fn cue_point() {
        test_string_content(Type::CuePoint, MetaEvent::CuePoint);
    }

    #[test]
    fn end_of_track() {
        assert_eq!(
            MetaEvent::from_bytes(Type::EndOfTrack, 0, &[]),
            Ok((0, MetaEvent::EndOfTrack))
        )
    }

    #[test]
    fn instrument_name() {
        test_string_content(Type::InstrumentName, MetaEvent::InstrumentName);
    }

    #[test]
    fn key_signature_length_invalid() {
        assert_eq!(
            MetaEvent::from_bytes(Type::KeySignature, 0, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x00,
                expected: 2
            })
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::KeySignature, 3, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x03,
                expected: 2,
            })
        );
    }

    #[test]
    fn key_signature_length_not_enough_bytes() {
        assert_eq!(
            MetaEvent::from_bytes(Type::KeySignature, 2, &[0x00]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 1,
                expected: 0x02
            })
        );
    }

    #[test]
    fn key_signature() {
        assert_eq!(
            MetaEvent::from_bytes(Type::KeySignature, 2, &[7_i8.cast_unsigned(), 0x00]),
            Ok((
                2,
                MetaEvent::KeySignature {
                    key: 7,
                    is_minor: false
                }
            ))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::KeySignature, 2, &[(-7_i8).cast_unsigned(), 0xFF]),
            Ok((
                2,
                MetaEvent::KeySignature {
                    key: -7,
                    is_minor: true
                }
            ))
        );
    }

    #[test]
    fn lyric() {
        test_string_content(Type::Lyric, MetaEvent::Lyric);
    }

    #[test]
    fn marker() {
        test_string_content(Type::Marker, MetaEvent::Marker);
    }

    #[test]
    fn sequence_number_length_invalid() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SequenceNumber, 0, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x00,
                expected: 2
            })
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SequenceNumber, 3, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x03,
                expected: 2,
            })
        );
    }

    #[test]
    fn sequence_number_length_not_enough_bytes() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SequenceNumber, 2, &[0x00]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 1,
                expected: 0x02
            })
        );
    }

    #[test]
    fn sequence_number() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SequenceNumber, 2, &[0x00, 0x05]),
            Ok((
                2,
                MetaEvent::SequenceNumber(u16::from_be_bytes([0x00, 0x05]))
            ))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SequenceNumber, 2, &[0x01, 0x06]),
            Ok((
                2,
                MetaEvent::SequenceNumber(u16::from_be_bytes([0x01, 0x06]))
            ))
        );
    }

    #[test]
    fn sequence_track_name() {
        test_string_content(Type::SequenceTrackName, MetaEvent::SequenceTrackName);
    }

    #[test]
    fn sequencer_specific_not_enough_bytes() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SequencerSpecific, 1, &[]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 0,
                expected: 0x01
            })
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SequencerSpecific, 2, &[0x00]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 1,
                expected: 0x02
            })
        );
    }

    #[test]
    fn sequencer_specific() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SequencerSpecific, 1, &[0x01]),
            Ok((1, MetaEvent::SequencerSpecific(vec![0x01])))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SequencerSpecific, 2, &[0x01, 0x02]),
            Ok((2, MetaEvent::SequencerSpecific(vec![0x01, 0x02])))
        );
    }

    #[test]
    fn set_tempo_length_invalid() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SetTempo, 0, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x00,
                expected: 3
            })
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SetTempo, 4, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x04,
                expected: 3,
            })
        );
    }

    #[test]
    fn set_tempo_length_not_enough_bytes() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SetTempo, 3, &[0x00, 0x00]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 2,
                expected: 0x03
            })
        );
    }

    #[test]
    fn set_tempo() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SetTempo, 3, &[0x00, 0x00, 0x06]),
            Ok((
                3,
                MetaEvent::SetTempo(u32::from_be_bytes([0x00, 0x00, 0x00, 0x06]))
            ))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SetTempo, 3, &[0x01, 0x02, 0x03]),
            Ok((
                3,
                MetaEvent::SetTempo(u32::from_be_bytes([0x00, 0x01, 0x02, 0x03]))
            ))
        );
    }

    #[test]
    fn smpte_offset_length_invalid() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SmpteOffset, 0, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x00,
                expected: 5
            })
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SmpteOffset, 6, &[0x00]),
            Err(ParseError::LengthNotExpected {
                actual: 0x06,
                expected: 5,
            })
        );
    }

    #[test]
    fn smpte_offset_length_not_enough_bytes() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SmpteOffset, 5, &[0x00, 0x00, 0x00, 0x00]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 4,
                expected: 0x05
            })
        );
    }

    #[test]
    fn smpte_offset() {
        assert_eq!(
            MetaEvent::from_bytes(Type::SmpteOffset, 5, &[0x01, 0x02, 0x03, 0x04, 0x05]),
            Ok((
                5,
                MetaEvent::SmpteOffset {
                    hours: 0x01,
                    minute: 0x02,
                    second: 0x03,
                    frame_rate: 0x04,
                    fractional_frames: 0x05
                }
            ))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::SmpteOffset, 5, &[0x11, 0x12, 0x13, 0x14, 0x15]),
            Ok((
                5,
                MetaEvent::SmpteOffset {
                    hours: 0x11,
                    minute: 0x12,
                    second: 0x13,
                    frame_rate: 0x14,
                    fractional_frames: 0x15
                }
            ))
        );
    }

    #[test]
    fn text() {
        test_string_content(Type::Text, MetaEvent::Text);
    }

    #[test]
    fn time_signature_length_invalid() {
        assert_eq!(
            MetaEvent::from_bytes(Type::TimeSignature, 0, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x00,
                expected: 4
            })
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::TimeSignature, 5, &[]),
            Err(ParseError::LengthNotExpected {
                actual: 0x05,
                expected: 4,
            })
        );
    }

    #[test]
    fn time_signature_length_not_enough_bytes() {
        assert_eq!(
            MetaEvent::from_bytes(Type::TimeSignature, 4, &[0x00, 0x00, 0x00]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 3,
                expected: 0x04
            })
        );
    }

    #[test]
    fn time_signature() {
        assert_eq!(
            MetaEvent::from_bytes(Type::TimeSignature, 4, &[0x01, 0x02, 0x03, 0x04]),
            Ok((
                4,
                MetaEvent::TimeSignature {
                    numerator: 0x01,
                    negative_denominator_power: 0x02,
                    clock_per_metronome_click: 0x03,
                    thirtyseconds_per_quarter_note: 0x04
                }
            ))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::TimeSignature, 4, &[0x06, 0x03, 0x13, 0x14]),
            Ok((
                4,
                MetaEvent::TimeSignature {
                    numerator: 0x06,
                    negative_denominator_power: 0x03,
                    clock_per_metronome_click: 0x13,
                    thirtyseconds_per_quarter_note: 0x14
                }
            ))
        );
    }

    fn test_string_content(typ: Type, create_variant: impl Fn(String) -> MetaEvent) {
        // Text invalid
        let text_invalid = [0xFFu8];
        assert_eq!(
            MetaEvent::from_bytes(typ, text_invalid.len(), text_invalid.as_slice()),
            Err(ParseError::TextInvalid(
                #[allow(invalid_from_utf8)]
                str::from_utf8(&text_invalid).unwrap_err()
            ))
        );

        // Text valid
        let text = "abc";
        assert_eq!(
            MetaEvent::from_bytes(typ, text.len(), text.as_bytes()),
            Ok((text.len(), create_variant(text.to_owned())))
        );
        let text = "xyz";
        assert_eq!(
            MetaEvent::from_bytes(typ, text.len(), text.as_bytes()),
            Ok((text.len(), create_variant(text.to_owned())))
        );
    }

    #[test]
    fn unknown_length_not_enough_bytes() {
        assert_eq!(
            MetaEvent::from_bytes(Type::Unknown(0xFF), 4, &[0x00, 0x00, 0x00]),
            Err(ParseError::LengthNotEnoughBytes {
                actual: 3,
                expected: 0x04
            })
        );
    }

    #[test]
    fn unknown() {
        assert_eq!(
            MetaEvent::from_bytes(Type::Unknown(0xFF), 4, &[0x00, 0x00, 0x00, 0x00]),
            Ok((4, MetaEvent::Unknown(vec![0x00, 0x00, 0x00, 0x00])))
        );
        assert_eq!(
            MetaEvent::from_bytes(Type::Unknown(0xFF), 4, &[0x00, 0x00, 0x01, 0x00, 0x00]),
            Ok((4, MetaEvent::Unknown(vec![0x00, 0x00, 0x01, 0x00])))
        );
    }
}
