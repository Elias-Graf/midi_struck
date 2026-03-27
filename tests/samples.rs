use std::fs;

use midi_struck::file::{self, Chunk, Content};

// TODO: Consider adding a try_all_chunks method to Content, similar to Track::try_all_events
fn parse_file(bytes: &[u8]) -> (file::Header, Vec<Vec<file::event::Event>>) {
    let mut content = Content::new(bytes).expect("failed to parse header");
    let header = content.header().clone();
    let mut tracks = Vec::new();

    while let Some((_, chunk)) = content.next_chunk().expect("failed to parse chunk") {
        match chunk {
            Chunk::Track(mut track) => {
                tracks.push(
                    track
                        .try_all_events()
                        .expect("failed to parse track events"),
                );
            }
            Chunk::Unknown(chunk_type, data) => {
                todo!("handle unknown chunk: type={chunk_type:?}, data={data:?}");
            }
        }
    }

    (header, tracks)
}

#[test]
fn all() {
    let mut entries: Vec<_> = fs::read_dir("tests/samples")
        .expect("failed to read tests/samples directory")
        .map(|entry| entry.expect("failed to read directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mid"))
        .collect();

    entries.sort();

    assert!(!entries.is_empty(), "no .mid files found in tests/samples");

    for path in entries {
        let bytes = fs::read(&path).expect("failed to read file");
        let parsed = parse_file(&bytes);
        let name = path.file_stem().unwrap().to_str().unwrap();
        insta::assert_debug_snapshot!(name.to_owned(), parsed);
    }
}
