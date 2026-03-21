use std::{
    env::{self},
    fs,
};

// TODO: All controllers off: 00 b0 79 00?
// TODO: Volume?
// TODO: pan?
// TODO: reverb?
// TODO: chorus?
// TODO: port prefix?

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();

    let file_path = args.get(1).ok_or("Usage: file_dump <file_path>")?;
    let file_bytes = fs::read(file_path)?;

    let mut ble = midi_struck::file::Content::new(&file_bytes)
        .map_err(|err| format!("failed to initialized file content {err}"))?;

    println!("{:=^40}", " Header ");
    println!("Format: {}", ble.header().format);
    println!("Number of tracks: {}", ble.header().number_of_tracks);
    println!("Division: {}", ble.header().division);

    let mut chunk_count = 0;
    // TODO: I don't like this tracking outside of the actual logic.
    // Maybe we should returned "spanned" "tokens"?
    let mut pos = ble.pos();

    while let Some((consumed_chunk, chunk)) = ble
        .next_chunk()
        .map_err(|err| format!("failed to get next chunk: {err}"))?
    {
        println!("{:=^40}", format!(" Chunk {} ", chunk_count + 1));
        chunk_count += 1;
        println!("{:08x}..{:08x}", pos, pos + consumed_chunk);
        println!("{:=^40}", "");

        let mut track = match chunk {
            midi_struck::file::Chunk::Track(track) => track,
            // TODO: add information what th chunk type is
            midi_struck::file::Chunk::Unknown(_) => {
                println!("unknown chunk");
                break;
            }
        };

        // TODO: add position information to more errors.
        while let Some((consumed_event, event)) = track
            .next_event()
            .map_err(|(index, err)| format!("at byte: {index}, got error: {err}"))?
        {
            let consumed_bytes = file_bytes.get(pos..pos + consumed_event).unwrap_or(&[]);

            println!(
                "{:08x}  {:<30} {:<4} {:<8} 𝚫 {:<4} {}",
                pos,
                consumed_bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                consumed_event,
                event.tick_absolute,
                event.tick_delta,
                event.content,
            );

            pos += consumed_event;
        }
    }

    // TODO: check more tracks

    Ok(())
}
