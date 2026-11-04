//! The async framing over a pipe that delivers lines in pieces.

#![cfg(feature = "runtime")]
#![allow(
    clippy::disallowed_methods,
    reason = "#[tokio::test] expands to the runtime's block_on"
)]

use ccwire::{
    framing::{AsyncLineReader, FrameReader, FrameWriter, ReadError},
    protocol::{Inbound, UserInput},
};
use serde_json::json;
use tokio::io::{AsyncWriteExt as _, duplex};

#[tokio::test]
async fn lines_split_across_writes_and_eof_mid_line() {
    let (mut tx, rx) = duplex(7);
    let input =
        b"{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\nnot json\n{\"type\":\"result\",\"subt".to_vec();
    let writer = tokio::spawn(async move {
        for chunk in input.chunks(3) {
            tx.write_all(chunk).await.unwrap();
            tokio::task::yield_now().await;
        }
    });
    let mut frames = FrameReader::new(rx);
    let (first, end) = frames.next_at().await.unwrap().unwrap();
    assert!(matches!(first, Inbound::User(_)), "{first:?}");
    let first_line = br#"{"type":"user","message":{"content":"hi"}}"#;
    assert_eq!(end, first_line.len() as u64 + 1);
    assert!(matches!(
        frames.next().await.unwrap(),
        Some(Inbound::Malformed { .. })
    ));
    // Cut off by EOF, still a frame: a result that drifted into garbage.
    let cut = frames.next().await.unwrap().unwrap();
    assert!(cut.ends_turn(), "{cut:?}");
    assert!(frames.next().await.unwrap().is_none());
    writer.await.unwrap();
}

#[tokio::test]
async fn too_long_lines_are_reported_and_skipped() {
    let (mut tx, rx) = duplex(64);
    tokio::spawn(async move {
        tx.write_all(&[b'a'; 1000]).await.unwrap();
        tx.write_all(b"\nok\n").await.unwrap();
    });
    let mut lines = AsyncLineReader::new(rx).max_line_len(100);
    let Err(ReadError::TooLong(e)) = lines.next_line().await else {
        panic!("not too long")
    };
    assert_eq!((e.len, e.end), (1000, 1001));
    let ok = lines.next_line().await.unwrap().unwrap();
    assert_eq!(ok.bytes, b"ok");
    assert!(lines.next_line().await.unwrap().is_none());
}

#[tokio::test]
async fn the_writer_writes_one_line_per_frame() {
    let (tx, rx) = duplex(1024);
    let mut w = FrameWriter::new(tx);
    w.write(&UserInput::text("a\nb")).await.unwrap();
    w.write(&json!({"type": "x"})).await.unwrap();
    w.shutdown().await.unwrap();
    drop(w);
    let mut lines = AsyncLineReader::new(rx);
    let first = lines.next_line().await.unwrap().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&first.bytes).unwrap();
    assert_eq!(v["message"]["content"], "a\nb");
    assert_eq!(
        lines.next_line().await.unwrap().unwrap().bytes,
        br#"{"type":"x"}"#
    );
    assert!(lines.next_line().await.unwrap().is_none());
}
