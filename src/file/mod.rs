use std::fmt::Display;

use crate::{SliceGetFixed, u32_as_usize};

pub mod event;
pub mod header;
pub mod track;

pub use header::Header;
pub use track::Track;

#[derive(Debug)]
pub struct Content<'a> {
    header: Header,
    bytes: &'a [u8],
    pos: usize,
    chunks_consumed: usize,
}

#[derive(Debug, PartialEq)]
pub enum Chunk<'a> {
    Track(Track<'a>),
    Unknown(&'a [u8; 4], &'a [u8]),
}

#[derive(Debug, PartialEq)]
pub enum ParseError<'a> {
    ChunkDataNotEnoughBytes {
        available: usize,
        length: usize,
        pos: usize,
    },
    ChunkLengthMissing {
        pos: usize,
    },
    ChunkTypeIncomplete {
        data: &'a [u8],
        pos: usize,
    },
    ChunksNotEnough {
        count: usize,
        expected: usize,
    },
    ChunksTooMany {
        expected: usize,
    },
}

impl Display for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::ChunkDataNotEnoughBytes {
                available,
                length,
                pos,
            } => {
                write!(
                    f,
                    "at: {pos}, chunk data not enough bytes: need {length}, have {available}"
                )
            }
            ParseError::ChunkLengthMissing { pos } => {
                write!(f, "at: {pos}, chunk length missing")
            }
            ParseError::ChunkTypeIncomplete { data, pos } => {
                write!(f, "at: {pos}, chunk type incomplete: {data:?}")
            }
            ParseError::ChunksNotEnough { count, expected } => {
                write!(f, "not enough chunks: expected {expected}, got {count}")
            }
            ParseError::ChunksTooMany { expected } => {
                write!(f, "too many chunks: expected {expected}")
            }
        }
    }
}

impl<'a> Content<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, header::ParseError> {
        let (consumed_header, header) = header::Header::from_bytes(bytes)?;

        Ok(Self {
            header,
            bytes,
            pos: consumed_header,
            chunks_consumed: 0,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn next_chunk(&mut self) -> Result<Option<(usize, Chunk<'_>)>, ParseError<'a>> {
        let mut pos = self.pos;
        let expected = self.header.number_of_tracks as usize;

        if let Some(x) = self.bytes.get(pos..)
            && x.is_empty()
        {
            if self.chunks_consumed < expected {
                return Err(ParseError::ChunksNotEnough {
                    count: self.chunks_consumed,
                    expected,
                });
            }
            return Ok(None);
        }

        if self.chunks_consumed >= expected {
            return Err(ParseError::ChunksTooMany { expected });
        }

        let chunk_type =
            self.bytes
                .get_fixed::<4>(pos)
                .ok_or_else(|| ParseError::ChunkTypeIncomplete {
                    data: self.bytes.get(pos..).unwrap_or(&[]),
                    pos,
                })?;

        pos += track::CHUNK_TYPE.len();

        let length = u32::from_be_bytes(
            *self
                .bytes
                .get_fixed::<4>(pos)
                .ok_or(ParseError::ChunkLengthMissing { pos })?,
        );

        let track_length = u32_as_usize(length);
        pos += 4;

        let available = self.bytes.len() - pos;
        let chunk_bytes =
            self.bytes
                .get(pos..pos + track_length)
                .ok_or(ParseError::ChunkDataNotEnoughBytes {
                    available,
                    length: track_length,
                    pos,
                })?;
        pos += track_length;

        let chunk = match chunk_type {
            track::CHUNK_TYPE => Chunk::Track(Track::new(chunk_bytes)),
            _ => Chunk::Unknown(chunk_type, chunk_bytes),
        };

        let consumed = pos - self.pos;

        self.pos = pos;
        self.chunks_consumed += 1;

        Ok(Some((consumed, chunk)))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        file::{
            Chunk, Content, ParseError,
            event::{self, Event},
            header::{self, Header},
            track::{self},
        },
        message_channel_voice::{self, MessageChannelVoice},
        meta_event::{self, MetaEvent},
    };

    #[test]
    pub fn header() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x02, 0x01, 0xE0].as_slice(),
        ]
        .concat();
        let content = Content::new(&bytes).unwrap();

        assert_eq!(
            content.header(),
            &Header {
                division: header::Division::TicksPerQuarterNote(480),
                format: header::Format::MultiTrack,
                number_of_tracks: 2
            }
        );
    }

    #[test]
    fn chunk_type_not_enough_bytes() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x02, 0x01, 0xE0].as_slice(),
            b"MT",
        ]
        .concat();

        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        assert_eq!(
            content.next_chunk().unwrap_err(),
            ParseError::ChunkTypeIncomplete {
                data: b"MT".as_slice(),
                pos: 14,
            }
        )
    }

    #[test]
    fn chunk_length_missing() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x01, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            [0x00, 0x00].as_slice(),
        ]
        .concat();

        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        assert_eq!(
            content.next_chunk().unwrap_err(),
            ParseError::ChunkLengthMissing { pos: 18 }
        );
    }

    #[test]
    fn chunk_data_not_enough_bytes() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x01, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            10_u32.to_be_bytes().as_slice(),
            [0x06, 0xFF, 0x2F, 0x00].as_slice(),
        ]
        .concat();

        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        assert_eq!(
            content.next_chunk().unwrap_err(),
            ParseError::ChunkDataNotEnoughBytes {
                available: 4,
                length: 10,
                pos: 22,
            }
        );
    }

    #[test]
    fn single_track() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x02, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            4_u32.to_be_bytes().as_slice(),
            [
                0x06,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
            ]
            .as_slice(),
        ]
        .concat();
        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        let chunk = match content.next_chunk() {
            Ok(chunk) => chunk,
            Err(err) => panic!("parsing chunk was not successful: {err:?}"),
        };

        let (track_start, mut track) = match chunk {
            Some((chunk_start, Chunk::Track(track))) => (chunk_start, track),
            Some((_, chunk)) => panic!("returned unexpected chunk: {chunk:?}"),
            None => panic!("returned END (None) instead of track"),
        };

        assert_eq!(track_start, 12);
        assert_eq!(
            track.next_event(),
            Ok(Some((
                4,
                Event {
                    tick_absolute: 6,
                    tick_delta: 6,
                    content: MetaEvent::EndOfTrack.into()
                }
            )))
        );
    }

    #[test]
    fn multi_track() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x02, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            7_u32.to_be_bytes().as_slice(),
            [
                0x06,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
                0x07,
                message_channel_voice::Status::Pressure.into(),
                0x08,
            ]
            .as_slice(),
            track::CHUNK_TYPE,
            7_u32.to_be_bytes().as_slice(),
            [
                0x0A,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
                0x0A,
                message_channel_voice::Status::Pressure.into(),
                0x00,
            ]
            .as_slice(),
        ]
        .concat();
        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        let chunk1 = match content.next_chunk() {
            Ok(chunk) => chunk,
            Err(err) => panic!("parsing chunk was not successful: {err:?}"),
        };

        let (track1_consumed, mut track1) = match chunk1 {
            Some((chunk_start, Chunk::Track(track))) => (chunk_start, track),
            Some((_, chunk)) => panic!("returned unexpected chunk: {chunk:?}"),
            None => panic!("returned END (None) instead of track"),
        };

        assert_eq!(track1_consumed, 15);
        assert_eq!(
            track1.next_event(),
            Ok(Some((
                4,
                Event {
                    tick_absolute: 6,
                    tick_delta: 6,
                    content: MetaEvent::EndOfTrack.into()
                },
            )))
        );
        assert_eq!(
            track1.next_event(),
            Ok(Some((
                3,
                Event {
                    tick_absolute: 13,
                    tick_delta: 7,
                    content: event::Content::ChannelVoice {
                        is_running_status: false,
                        message: MessageChannelVoice::ChannelPressure(8)
                    }
                },
            )))
        );

        let chunk2 = match content.next_chunk() {
            Ok(chunk) => chunk,
            Err(err) => panic!("parsing chunk was not successful: {err:?}"),
        };

        let (track2_consumed, mut track2) = match chunk2 {
            Some((chunk_start, Chunk::Track(track))) => (chunk_start, track),
            Some((_, chunk)) => panic!("returned unexpected chunk: {chunk:?}"),
            None => panic!("returned END (None) instead of track"),
        };

        assert_eq!(track2_consumed, 15);
        assert_eq!(
            track2.next_event(),
            Ok(Some((
                4,
                Event {
                    tick_absolute: 10,
                    tick_delta: 10,
                    content: MetaEvent::EndOfTrack.into()
                },
            )))
        );
        assert_eq!(
            track2.next_event(),
            Ok(Some((
                3,
                Event {
                    tick_absolute: 20,
                    tick_delta: 10,
                    content: event::Content::ChannelVoice {
                        is_running_status: false,
                        message: MessageChannelVoice::ChannelPressure(0)
                    }
                },
            )))
        );
    }

    #[test]
    fn after_all_chunks_not_enough_chunks_error() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x02, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            4_u32.to_be_bytes().as_slice(),
            [
                0x06,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
            ]
            .as_slice(),
        ]
        .concat();

        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        assert!(content.next_chunk().unwrap().is_some());
        assert_eq!(
            content.next_chunk().unwrap_err(),
            ParseError::ChunksNotEnough {
                count: 1,
                expected: 2,
            }
        );
    }

    #[test]
    fn after_all_chunks_too_many_chunks_error() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x01, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            4_u32.to_be_bytes().as_slice(),
            [
                0x06,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
            ]
            .as_slice(),
            track::CHUNK_TYPE,
            4_u32.to_be_bytes().as_slice(),
            [
                0x06,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
            ]
            .as_slice(),
        ]
        .concat();

        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        assert!(content.next_chunk().unwrap().is_some());
        assert_eq!(
            content.next_chunk().unwrap_err(),
            ParseError::ChunksTooMany { expected: 1 }
        );
    }

    #[test]
    fn after_all_chunks_none() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x02, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            7_u32.to_be_bytes().as_slice(),
            [
                0x06,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
                0x07,
                message_channel_voice::Status::Pressure.into(),
                0x08,
            ]
            .as_slice(),
            track::CHUNK_TYPE,
            7_u32.to_be_bytes().as_slice(),
            [
                0x0A,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
                0x0A,
                message_channel_voice::Status::Pressure.into(),
                0x00,
            ]
            .as_slice(),
        ]
        .concat();

        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        assert!(content.next_chunk().unwrap().is_some());
        assert!(content.next_chunk().unwrap().is_some());
        assert!(content.next_chunk().unwrap().is_none());
    }

    #[test]
    fn skips_unknown_chunks() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x02, 0x01, 0xE0].as_slice(),
            b"unkn",
            2_u32.to_be_bytes().as_slice(),
            [0x00, 0xFF].as_slice(),
            track::CHUNK_TYPE,
            7_u32.to_be_bytes().as_slice(),
            [
                0x0A,
                meta_event::STATUS,
                meta_event::Type::EndOfTrack.into(),
                0x00,
                0x0A,
                message_channel_voice::Status::Pressure.into(),
                0x00,
            ]
            .as_slice(),
        ]
        .concat();

        let mut content = match Content::new(&bytes) {
            Ok(content) => content,
            Err(err) => panic!("parsing content was not successful: {err:?}"),
        };

        let chunk1 = match content.next_chunk() {
            Ok(chunk) => chunk,
            Err(err) => panic!("parsing chunk was not successful: {err:?}"),
        };

        assert_eq!(chunk1, Some((10, Chunk::Unknown(b"unkn", &[0x00, 0xFF]))));
    }

    #[test]
    fn unknown_chunk_with_other_chunks() {
        let track_data: &[u8] = &[
            0x00,
            meta_event::STATUS,
            meta_event::Type::EndOfTrack.into(),
            0x00,
        ];

        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x04, 0x01, 0xE0].as_slice(),
            track::CHUNK_TYPE,
            4_u32.to_be_bytes().as_slice(),
            track_data,
            b"unkn",
            2_u32.to_be_bytes().as_slice(),
            [0x00, 0xFF].as_slice(),
            b"XTRA",
            3_u32.to_be_bytes().as_slice(),
            [0x01, 0x02, 0x03].as_slice(),
            track::CHUNK_TYPE,
            4_u32.to_be_bytes().as_slice(),
            track_data,
        ]
        .concat();

        let mut content = Content::new(&bytes).unwrap();

        let (_, chunk1) = content.next_chunk().unwrap().unwrap();
        assert!(matches!(chunk1, Chunk::Track(_)));

        let (_, chunk2) = content.next_chunk().unwrap().unwrap();
        assert_eq!(chunk2, Chunk::Unknown(b"unkn", &[0x00, 0xFF]));

        let (_, chunk3) = content.next_chunk().unwrap().unwrap();
        assert_eq!(chunk3, Chunk::Unknown(b"XTRA", &[0x01, 0x02, 0x03]));

        let (_, chunk4) = content.next_chunk().unwrap().unwrap();
        assert!(matches!(chunk4, Chunk::Track(_)));
    }

    #[test]
    fn unknown_chunk_stores_type_and_data() {
        let bytes = [
            header::CHUNK_TYPE,
            6_u32.to_be_bytes().as_slice(),
            [0x00, 0x01, 0x00, 0x01, 0x01, 0xE0].as_slice(),
            b"CuSt",
            4_u32.to_be_bytes().as_slice(),
            [0xDE, 0xAD, 0xBE, 0xEF].as_slice(),
        ]
        .concat();

        let mut content = Content::new(&bytes).unwrap();
        let (consumed, chunk) = content.next_chunk().unwrap().unwrap();

        assert_eq!(consumed, 12);
        let Chunk::Unknown(chunk_type, data) = chunk else {
            panic!("expected Unknown chunk");
        };
        assert_eq!(chunk_type, b"CuSt");
        assert_eq!(data, &[0xDE, 0xAD, 0xBE, 0xEF]);
    }
}
