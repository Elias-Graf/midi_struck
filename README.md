**_All rights reserved, for educational purposes only!_**

# midi_struck

A MIDI file parser library written in Rust.

## Sample Tests

The `tests/samples/` directory contains `.mid` files used for snapshot-based integration testing via [insta](https://insta.rs). Every `.mid` file in the directory is automatically discovered, parsed, and compared against a stored snapshot.

Run the tests:

```sh
cargo test --test samples
```

When a snapshot doesn't match (or doesn't exist yet), the test fails. Use environment variables to control how insta handles snapshot updates:

```sh
# Accept all new and changed snapshots
INSTA_UPDATE=always cargo test --test samples

# Accept only new snapshots, fail on changed ones
INSTA_UPDATE=new cargo test --test samples
```

To add a new test case, drop a `.mid` file into `tests/samples/` and run `INSTA_UPDATE=new cargo test --test samples`.
