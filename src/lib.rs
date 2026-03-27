pub mod file;
pub mod message_channel_voice;
pub mod message_system_common;
pub mod message_system_real_time;
pub mod meta_event;
pub mod note;
pub mod variable_length_quantity;

use core::mem;

pub use message_channel_voice::MessageChannelVoice;
pub use note::Note;

// TODO: Add (test) coverage report.

pub trait SliceGetFixed<T> {
    fn get_fixed<const N: usize>(&self, index: usize) -> Option<&[T; N]>;
}

impl<T> SliceGetFixed<T> for [T] {
    fn get_fixed<const N: usize>(&self, index: usize) -> Option<&[T; N]> {
        self.get(index..index + N)
            .and_then(|value| value.try_into().ok())
    }
}

pub fn u32_as_usize(input: u32) -> usize {
    const { assert!(mem::size_of::<u32>() <= mem::size_of::<usize>()) };
    input as usize
}
