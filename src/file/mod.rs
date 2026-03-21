use std::fmt::Display;

use crate::SliceGetFixed;

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
}

#[derive(Debug, PartialEq)]
pub enum Chunk<'a> {
    Track(Track<'a>),
    Unknown(&'a [u8]),
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    // TODO: aren't we doing something similar in track?
    ChunkTypeIncomplete { data: Vec<u8>, pos: usize },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::ChunkTypeIncomplete { data, pos } => {
                write!(f, "at: {pos}, chunk type incomplete: {data:?}")
            }
        }
    }
}

impl<'a> Content<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, header::ParseError> {
        let (consumed_header, header) = header::Header::from_bytes(bytes)?;

        //
        // let mut tracks = Vec::new();
        // // TODO: unwrap
        // let mut track_bytes = bytes.get(header_consumed..).unwrap();
        //
        // while track_bytes.starts_with(track::CHUNK_TYPE) {
        //     let (track_consumed, track) =
        //         track::Track::new(track_bytes).map_err(|(_, err)| ParseError::TrackInvalid {
        //             number: tracks.len(),
        //             err,
        //         })?;
        //     tracks.push(track);
        //
        //     // TODO: unwrap
        //     track_bytes = track_bytes.get(track_consumed..).unwrap();
        // }
        //
        // Ok(Self { header, tracks })
        Ok(Self {
            header,
            bytes,
            pos: consumed_header,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn next_chunk(&mut self) -> Result<Option<(usize, Chunk<'_>)>, ParseError> {
        let mut pos = self.pos;

        if let Some(x) = self.bytes.get(pos..)
            && x.is_empty()
        {
            return Ok(None);
        }

        let chunk_type =
            self.bytes
                .get_fixed::<4>(pos)
                .ok_or_else(|| ParseError::ChunkTypeIncomplete {
                    data: (self.bytes.get(pos..).unwrap_or(&[]).to_vec()),
                    pos,
                })?;

        pos += track::CHUNK_TYPE.len();

        let length = u32::from_be_bytes(
            *self
                .bytes
                .get_fixed::<4>(pos)
                // TODO: unwrap, test
                .unwrap(),
        );
        // TODO: assert usize size
        let track_length = length as usize;
        pos += 4;

        let chunk_bytes = self
            .bytes
            .get(pos..pos + track_length)
            // TODO: unwrap, test
            .unwrap();
        pos += track_length;

        let chunk = match chunk_type {
            track::CHUNK_TYPE => Chunk::Track(Track::new(chunk_bytes)),
            // TODO: add chunk type to unknown chunk
            _ => Chunk::Unknown(chunk_bytes),
        };

        let consumed = pos - self.pos;

        self.pos = pos;

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
                data: b"MT".to_vec(),
                pos: 14,
            }
        )
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

    // TODO:
    // #[test]
    // fn after_all_chunks_not_enough_chunks_error() {}
    //
    // TODO:
    // #[test]
    // fn after_all_chunks_too_many_chunks_error() {}

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

        assert_eq!(chunk1, Some((10, Chunk::Unknown(&[0x00, 0xFF]))));
    }
}
