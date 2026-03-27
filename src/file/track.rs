use crate::{
    file::event::{self, Event},
    message_channel_voice,
};

pub const CHUNK_TYPE: &[u8; 4] = b"MTrk";

#[derive(Debug, PartialEq)]
pub struct Track<'a> {
    bytes: &'a [u8],
    pos: usize,
    running_status: Option<message_channel_voice::Status>,
    tick: u32,
}

impl<'a> Track<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            running_status: None,
            tick: 0,
        }
    }

    pub fn next_event(&mut self) -> Result<Option<(usize, Event)>, ParseError> {
        let mut pos = self.pos;

        let Some(event_bytes) = self.bytes.get(pos..) else {
            return Ok(None);
        };

        if event_bytes.is_empty() {
            return Ok(None);
        }

        let (consumed_event, event, new_running_status) =
            Event::from_bytes(event_bytes, self.tick, self.running_status)
                // TODO: remove clone (to_vec)
                .map_err(|(pos_event, err)| ParseError::EventInvalid {
                    data: (self.bytes.get(pos + pos_event..)).unwrap_or(&[]).to_vec(),
                    pos: pos + pos_event,
                    err,
                })?;

        self.running_status = new_running_status;
        pos += consumed_event;

        self.tick = event.tick_absolute;
        self.pos = pos;

        Ok(Some((consumed_event, event)))
    }

    pub fn try_all_events(mut self) -> Result<Vec<Event>, ParseError> {
        let mut events = Vec::new();

        while let Some((_, event)) = self.next_event()? {
            events.push(event);
        }

        Ok(events)
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    EventInvalid {
        data: Vec<u8>,
        pos: usize,
        err: event::ParseError,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EventInvalid { pos, err, .. } => write!(f, "at {pos}: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        file::{
            event::{self, Event},
            track::Track,
        },
        message_channel_voice::{self, MessageChannelVoice},
        meta_event::{self, MetaEvent},
    };

    #[test]
    fn single_event() {
        let bytes = &[
            0x05,
            meta_event::STATUS,
            meta_event::Type::EndOfTrack.into(),
            0x00,
        ];
        let mut track = Track::new(bytes);

        assert_eq!(
            track.next_event(),
            Ok(Some((
                4,
                Event {
                    tick_absolute: 5,
                    tick_delta: 5,
                    content: MetaEvent::EndOfTrack.into()
                }
            )))
        );
    }

    #[test]
    fn multiple_events() {
        let bytes = [
            0x05,
            meta_event::STATUS,
            meta_event::Type::EndOfTrack.into(),
            0x00,
            0x07,
            message_channel_voice::Status::Pressure.into(),
            0x08,
        ];
        let mut track = Track::new(&bytes);

        assert_eq!(
            track.next_event(),
            Ok(Some((
                4,
                Event {
                    tick_absolute: 5,
                    tick_delta: 5,
                    content: MetaEvent::EndOfTrack.into()
                }
            )))
        );
        assert_eq!(
            track.next_event(),
            Ok(Some((
                3,
                Event {
                    tick_absolute: 12,
                    tick_delta: 7,
                    content: event::Content::ChannelVoice {
                        is_running_status: false,
                        message: MessageChannelVoice::ChannelPressure(8),
                    }
                }
            )))
        );
    }

    #[test]
    fn running_status() {
        let bytes = [
            0x00,
            message_channel_voice::Status::Pressure.into(),
            0x08,
            0x01,
            0x07,
        ];
        let mut track = Track::new(&bytes);

        assert_eq!(
            track.next_event(),
            Ok(Some((
                3,
                Event {
                    tick_absolute: 0,
                    tick_delta: 0,
                    content: event::Content::ChannelVoice {
                        is_running_status: false,
                        message: MessageChannelVoice::ChannelPressure(8)
                    }
                },
            )))
        );
        assert_eq!(
            track.next_event(),
            Ok(Some((
                2,
                Event {
                    tick_absolute: 1,
                    tick_delta: 1,
                    content: event::Content::ChannelVoice {
                        is_running_status: true,
                        message: MessageChannelVoice::ChannelPressure(7)
                    }
                }
            )))
        );
    }

    #[test]
    fn end_returns_none() {
        let bytes = &[
            0x05,
            meta_event::STATUS,
            meta_event::Type::EndOfTrack.into(),
            0x00,
        ];
        let mut track = Track::new(bytes);

        assert!(track.next_event().unwrap().is_some());
        assert!(track.next_event().unwrap().is_none());
    }

    #[test]
    fn tick_accumulates_across_events() {
        let bytes = [
            0x0A,
            message_channel_voice::Status::Pressure.into(),
            0x08,
            0x14,
            0x07,
            0x05,
            meta_event::STATUS,
            meta_event::Type::EndOfTrack.into(),
            0x00,
        ];
        let mut track = Track::new(&bytes);

        let (_, event1) = track.next_event().unwrap().unwrap();
        assert_eq!(event1.tick_delta, 10);
        assert_eq!(event1.tick_absolute, 10);

        let (_, event2) = track.next_event().unwrap().unwrap();
        assert_eq!(event2.tick_delta, 20);
        assert_eq!(event2.tick_absolute, 30);

        let (_, event3) = track.next_event().unwrap().unwrap();
        assert_eq!(event3.tick_delta, 5);
        assert_eq!(event3.tick_absolute, 35);
    }

    #[test]
    fn try_all_events_collects_all_events() {
        let bytes = [
            0x05,
            message_channel_voice::Status::Pressure.into(),
            0x08,
            0x03,
            0x07,
            0x02,
            meta_event::STATUS,
            meta_event::Type::EndOfTrack.into(),
            0x00,
        ];
        let track = Track::new(&bytes);

        let events = track.try_all_events().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(
            events[0],
            Event {
                tick_absolute: 5,
                tick_delta: 5,
                content: event::Content::ChannelVoice {
                    is_running_status: false,
                    message: MessageChannelVoice::ChannelPressure(8),
                }
            }
        );
        assert_eq!(
            events[1],
            Event {
                tick_absolute: 8,
                tick_delta: 3,
                content: event::Content::ChannelVoice {
                    is_running_status: true,
                    message: MessageChannelVoice::ChannelPressure(7),
                }
            }
        );
        assert_eq!(
            events[2],
            Event {
                tick_absolute: 10,
                tick_delta: 2,
                content: MetaEvent::EndOfTrack.into()
            }
        );
    }

    #[test]
    fn try_all_events_empty_track() {
        let track = Track::new(&[]);
        let events = track.try_all_events().unwrap();
        assert!(events.is_empty());
    }
}
