//! Print frame types from a saved stream-json transcript, without a CLI or runtime.
use std::{fs::File, io};

use ccwire::{framing::LineReader, protocol::Inbound};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: transcript <path.jsonl>",
        )
    })?;
    let mut reader = LineReader::new(File::open(path)?);
    while let Some(line) = reader.next_line()? {
        if line.bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let frame = Inbound::from_line(core::str::from_utf8(&line.bytes)?)?;
        println!(
            "offset={} ends_turn={} {frame:?}",
            line.end,
            frame.ends_turn()
        );
    }
    Ok(())
}
