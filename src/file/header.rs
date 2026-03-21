use std::{
    fmt::{Debug, Display},
    mem,
};

use crate::SliceGetFixed;

pub const CHUNK_TYPE: &[u8; 4] = b"MThd";

#[derive(Debug, PartialEq, Clone)]
pub struct Header {
    pub division: Division,
    pub format: Format,
    pub number_of_tracks: u16,
}

impl Header {
    pub fn from_bytes(bytes: &[u8]) -> Result<(usize, Self), ParseError> {
        let chunk_type = bytes.get(..4).unwrap_or(bytes);
        if chunk_type != CHUNK_TYPE {
            return Err(ParseError::ChunkTypeInvalid(chunk_type.to_vec()));
        }

        const { assert!(mem::size_of::<u32>() <= mem::size_of::<usize>()) };
        let length =
            u32::from_be_bytes(*bytes.get_fixed::<4>(4).ok_or(ParseError::LengthMissing)?) as usize;

        if length != 6 {
            return Err(ParseError::LengthNotExpected {
                actual: length,
                expected: 6,
            });
        }

        let format = u16::from_be_bytes(*bytes.get_fixed::<2>(8).ok_or(ParseError::FormatMissing)?)
            .try_into()
            .map_err(ParseError::FormatInvalid)?;

        let number_of_tracks = u16::from_be_bytes(
            *bytes
                .get_fixed::<2>(10)
                .ok_or(ParseError::NumberOfTracksMissing)?,
        );

        if format == Format::SingleTrack && number_of_tracks != 1 {
            return Err(ParseError::NumberOfTracksNotOne(number_of_tracks));
        }

        let division = u16::from_be_bytes(
            *bytes
                .get_fixed::<2>(12)
                .ok_or(ParseError::DivisionMissing)?,
        )
        .into();

        Ok((
            14,
            Self {
                division,
                format,
                number_of_tracks,
            },
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Format {
    SingleTrack,
    MultiTrack,
    MultiTrackIndependent,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::SingleTrack => write!(f, "single track"),
            Format::MultiTrack => write!(f, "multi track"),
            Format::MultiTrackIndependent => write!(f, "multi track independent"),
        }
    }
}

impl TryFrom<u16> for Format {
    type Error = u16;

    /// Returns the `value` if the variant could not be matched.
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::SingleTrack,
            1 => Self::MultiTrack,
            2 => Self::MultiTrackIndependent,
            n => return Err(n),
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Division {
    TicksPerQuarterNote(u16),
    DeltaTime {
        frames_per_second: u8,
        ticks_per_frame: u8,
    },
}

impl Display for Division {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Division::TicksPerQuarterNote(ticks) => write!(f, "ticks per quarter note: {ticks}"),
            Division::DeltaTime {
                frames_per_second,
                ticks_per_frame,
            } => write!(
                f,
                "delta time - fps: {frames_per_second}, tpf: {ticks_per_frame}"
            ),
        }
    }
}

impl From<u16> for Division {
    fn from(value: u16) -> Self {
        if value & 0x80_00 == 0 {
            Self::TicksPerQuarterNote(value)
        } else {
            let [frames_per_second, ticks_per_frame] = value.to_be_bytes();
            Self::DeltaTime {
                frames_per_second: frames_per_second.cast_signed().unsigned_abs(),
                ticks_per_frame,
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    ChunkTypeInvalid(Vec<u8>),
    DivisionMissing,
    FormatInvalid(u16),
    FormatMissing,
    LengthMissing,
    LengthNotExpected { actual: usize, expected: usize },
    NumberOfTracksMissing,
    NumberOfTracksNotOne(u16),
}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::ChunkTypeInvalid(items) => write!(
                f,
                "chunk type is invalid, str repr: {:?}, bytes: {:?}",
                String::from_utf8(items.clone()),
                items,
            ),
            ParseError::DivisionMissing => write!(f, "division is missing"),
            ParseError::FormatInvalid(format) => write!(f, "format invalid: {format}"),
            ParseError::FormatMissing => write!(f, "format is missing"),
            // TODO: can this be more explicit?
            ParseError::LengthMissing => write!(f, "length is missing"),
            // TODO: can this be more explicit?
            ParseError::LengthNotExpected { actual, expected } => write!(
                f,
                "length not expected, expected: {expected}, actual: {actual}"
            ),
            ParseError::NumberOfTracksMissing => write!(f, "number of tracks missing"),
            ParseError::NumberOfTracksNotOne(n) => write!(
                f,
                "expected single track, as declared in format, but received {n} tracks"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::file::header::{Division, Format, Header, ParseError};

    #[test]
    fn chunk_type_incomplete() {
        assert_eq!(
            Header::from_bytes(&[b'M', b'T', 0x00]),
            Err(ParseError::ChunkTypeInvalid(vec![b'M', b'T', 0x00]))
        );
    }

    #[test]
    fn chunk_type_not_header() {
        assert_eq!(
            Header::from_bytes(&[b'M', b'T', b'r', b'k', 0x00]),
            Err(ParseError::ChunkTypeInvalid(b"MTrk".to_vec()))
        );
    }

    #[test]
    fn length_missing() {
        assert_eq!(Header::from_bytes(b"MThd"), Err(ParseError::LengthMissing));
    }

    #[test]
    fn length_not_expected() {
        assert_eq!(
            Header::from_bytes(&[b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x05]),
            Err(ParseError::LengthNotExpected {
                actual: 0x05,
                expected: 6
            })
        );
        assert_eq!(
            Header::from_bytes(&[b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x07]),
            Err(ParseError::LengthNotExpected {
                actual: 0x07,
                expected: 6
            })
        );
    }

    #[test]
    fn format_missing() {
        assert_eq!(
            Header::from_bytes(&[b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06]),
            Err(ParseError::FormatMissing)
        );
    }

    #[test]
    fn format_invalid() {
        assert_eq!(
            Header::from_bytes(&[b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x03]),
            Err(ParseError::FormatInvalid(3))
        );
    }

    #[test]
    fn format_single_track() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00
            ])
            .map(|r| r.1.format),
            Ok(Format::SingleTrack)
        );
    }

    #[test]
    fn format_multi_track() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00
            ])
            .map(|r| r.1.format),
            Ok(Format::MultiTrack)
        );
    }

    #[test]
    fn format_multi_track_independent() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00
            ])
            .map(|r| r.1.format),
            Ok(Format::MultiTrackIndependent)
        );
    }

    #[test]
    fn number_of_tracks_missing() {
        assert_eq!(
            Header::from_bytes(&[b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x01]),
            Err(ParseError::NumberOfTracksMissing)
        );
    }

    #[test]
    fn number_of_tracks_incorrect_for_single_track() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00
            ]),
            Err(ParseError::NumberOfTracksNotOne(0))
        );
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x02
            ]),
            Err(ParseError::NumberOfTracksNotOne(2))
        );
    }

    #[test]
    fn number_of_tracks_single_track() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00
            ])
            .map(|r| r.1.number_of_tracks),
            Ok(1)
        );
    }

    #[test]
    fn number_of_tracks_multi_track() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00
            ])
            .map(|r| r.1.number_of_tracks),
            Ok(2)
        );
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00
            ])
            .map(|r| r.1.number_of_tracks),
            Ok(3)
        );
    }

    #[test]
    fn division_missing() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01
            ]),
            Err(ParseError::DivisionMissing)
        );
    }

    #[test]
    fn division_ticks_per_quarter_note() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01
            ])
            .map(|r| r.1.division),
            Ok(Division::TicksPerQuarterNote(1))
        );
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x0F, 0xFF
            ])
            .map(|r| r.1.division),
            Ok(Division::TicksPerQuarterNote(0x0F_FF))
        );
    }

    #[test]
    fn division_smpte_frames_per_second_invalid() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE8, 0x64
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 24,
                ticks_per_frame: 100
            })
        );
    }

    #[test]
    fn division_smpte_24() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE8, 0x64
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 24,
                ticks_per_frame: 100
            })
        );
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE8, 0xC8
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 24,
                ticks_per_frame: 200
            })
        );
    }

    #[test]
    fn division_smpte_25() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE7, 0x64
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 25,
                ticks_per_frame: 100
            })
        );
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE7, 0xC8
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 25,
                ticks_per_frame: 200
            })
        );
    }

    #[test]
    fn division_smpte_29() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE3, 0x64
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 29,
                ticks_per_frame: 100
            })
        );
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE3, 0xC8
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 29,
                ticks_per_frame: 200
            })
        );
    }

    #[test]
    fn division_smpte_30() {
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE2, 0x64
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 30,
                ticks_per_frame: 100
            })
        );
        assert_eq!(
            Header::from_bytes(&[
                b'M', b'T', b'h', b'd', 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xE2, 0xC8
            ])
            .map(|r| r.1.division),
            Ok(Division::DeltaTime {
                frames_per_second: 30,
                ticks_per_frame: 200
            })
        );
    }

    #[test]
    fn example_header() {
        assert_eq!(
            Header::from_bytes(&[
                0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x01, 0x00, 0x02, 0x01, 0xE0,
            ]),
            Ok((
                14,
                Header {
                    division: Division::TicksPerQuarterNote(480),
                    format: Format::MultiTrack,
                    number_of_tracks: 2
                }
            ))
        )
    }
}
