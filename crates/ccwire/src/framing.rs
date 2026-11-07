//! Line-delimited JSON framing: stream-json is one JSON object per line.
//!
//! - [`LineReader`] (sync, any [`std::io::Read`]) and [`AsyncLineReader`](https://docs.rs/ccwire/latest/ccwire/framing/struct.AsyncLineReader.html)
//!   (feature `runtime`, any [`tokio::io::AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html)) split a byte stream
//!   into lines, bounded by a maximum length. A line over it is skipped and
//!   reported as a [`LineTooLong`], and reading goes on after it.
//! - [`FrameReader`](https://docs.rs/ccwire/latest/ccwire/framing/struct.FrameReader.html) (feature `runtime`) turns the lines into [`Inbound`]
//!   frames. It never fails on content: a line that isn't JSON, or is too
//!   long, becomes [`Inbound::Malformed`]; only an I/O error ends it.
//! - [`FrameWriter`](https://docs.rs/ccwire/latest/ccwire/framing/struct.FrameWriter.html) (feature `runtime`) writes one frame per line.
//!
//! Every line carries the byte offset just past it ([`Line::end`]), counted
//! from the reader's start offset, so a caller can persist how far it got
//! and resume a file from there.
//!
//! EOF: a last line without a newline is still returned, with
//! [`Line::terminated`] false (a writer killed mid-line leaves one); a
//! clean EOF after a newline is just the end.

use std::io;

use serde_json::Value;

use crate::protocol::Inbound;

/// The default maximum line length: 64 MiB. A tool result can be big, but
/// an unbounded line is an out-of-memory.
pub const DEFAULT_MAX_LINE: usize = 64 * 1024 * 1024;

/// How many bytes of a skipped line [`LineTooLong::head`] keeps.
const HEAD_LEN: usize = 1024;

/// One line, without its newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Its bytes (a trailing `\r` is kept: JSON parsing ignores it).
    pub bytes: Vec<u8>,
    /// The offset just past it (and its newline).
    pub end: u64,
    /// Whether it ended with a newline (false only for a last line at EOF).
    pub terminated: bool,
}

/// A line over the maximum length. It was skipped: the reader is past it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line too long: {len} bytes, over the limit of {limit}")]
pub struct LineTooLong {
    /// Its length, without the newline.
    pub len: u64,
    /// The limit it went over.
    pub limit: usize,
    /// The offset just past it.
    pub end: u64,
    /// Its first bytes (up to 1 KiB), to tell what it was.
    pub head: Vec<u8>,
    /// Whether it ended with a newline (false only at EOF).
    pub terminated: bool,
}

impl LineTooLong {
    /// The [`Inbound::Malformed`] that reports it. Its `type_` is read off
    /// the head (its top-level `"type"`, when the head has it), so a giant
    /// `result` still ends the turn ([`Inbound::ends_turn`]); `raw` is the
    /// head as a string.
    pub fn to_inbound(&self) -> Inbound {
        let type_ = sniff_type(&self.head);
        let error = self.to_string();
        crate::protocol::stats::malformed(&type_, &error);
        Inbound::Malformed {
            type_,
            error,
            raw: Value::String(String::from_utf8_lossy(&self.head).into_owned()),
        }
    }
}

/// A [`LineReader`] or [`AsyncLineReader`](https://docs.rs/ccwire/latest/ccwire/framing/struct.AsyncLineReader.html) error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReadError {
    /// The underlying read failed: the stream is done.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A line was over the limit and skipped: reading can go on.
    #[error(transparent)]
    TooLong(#[from] LineTooLong),
}

/// Parse one line as a frame. `None` for a blank line; a line that isn't a
/// JSON object is [`Inbound::Malformed`] (see [`LineTooLong::to_inbound`]
/// for its `type_`), with the line as a string in `raw`.
pub fn parse_line(bytes: &[u8]) -> Option<Inbound> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    let error = match serde_json::from_slice::<Value>(bytes) {
        Ok(Value::Object(obj)) => return Some(Inbound::from_object(obj)),
        Ok(_) => "not a JSON object".to_owned(),
        Err(e) => format!("invalid JSON: {e}"),
    };
    let type_ = sniff_type(bytes);
    crate::protocol::stats::malformed(&type_, &error);
    Some(Inbound::Malformed {
        type_,
        error,
        raw: Value::String(String::from_utf8_lossy(bytes).into_owned()),
    })
}

/// The top-level `"type"` of a line that may be cut off (a JSON object's
/// first bytes), else empty: enough of a scan to skip strings and nested
/// values, no more.
fn sniff_type(bytes: &[u8]) -> String {
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let Some((s, next)) = scan_string(bytes, i) else {
                    return String::new();
                };
                i = next;
                if depth == 1 && s == b"type" {
                    let rest = skip_ws(bytes, i);
                    if bytes.get(rest) == Some(&b':') {
                        let v = skip_ws(bytes, rest + 1);
                        if bytes.get(v) == Some(&b'"') {
                            return scan_string(bytes, v)
                                .filter(|(s, _)| !s.contains(&b'\\'))
                                .map(|(s, _)| String::from_utf8_lossy(s).into_owned())
                                .unwrap_or_default();
                        }
                    }
                }
                continue;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    String::new()
}

/// The contents of the string whose opening quote is at `start`, and the
/// index past its closing quote; `None` if it is cut off.
fn scan_string(bytes: &[u8], start: usize) -> Option<(&[u8], usize)> {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some((&bytes[start + 1..i], i + 1)),
            _ => i += 1,
        }
    }
    None
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

/// Encode a frame as one line: compact JSON and a newline (compact JSON has
/// no raw newline in it).
///
/// # Errors
/// The frame doesn't serialize.
pub fn encode_line<T: serde::Serialize + ?Sized>(frame: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut out = serde_json::to_vec(frame)?;
    out.push(b'\n');
    Ok(out)
}

/// The splitting state shared by the sync and async readers: what it holds
/// of the current line survives between reads, so a read can stop anywhere
/// (and an async one can be cancelled) without losing bytes.
#[derive(Debug)]
struct Splitter {
    buf: Vec<u8>,
    max: usize,
    pos: u64,
    /// The length so far of a line over the limit, being skipped.
    skipping: Option<u64>,
}

type Split = Result<Line, LineTooLong>;

impl Splitter {
    fn new(max: usize, start: u64) -> Self {
        Self {
            buf: Vec::new(),
            max,
            pos: start,
            skipping: None,
        }
    }

    /// Take what it can of `avail`: how many bytes it used, and a line if
    /// one ended in them.
    fn feed(&mut self, avail: &[u8]) -> (usize, Option<Split>) {
        if let Some(i) = avail.iter().position(|&b| b == b'\n') {
            self.take(&avail[..i]);
            self.pos += i as u64 + 1;
            (i + 1, Some(self.finish(true)))
        } else {
            self.take(avail);
            self.pos += avail.len() as u64;
            (avail.len(), None)
        }
    }

    /// At EOF: the unterminated last line, if any.
    fn eof(&mut self) -> Option<Split> {
        (!self.buf.is_empty() || self.skipping.is_some()).then(|| self.finish(false))
    }

    fn take(&mut self, chunk: &[u8]) {
        if let Some(len) = &mut self.skipping {
            *len += chunk.len() as u64;
        } else if self.buf.len() + chunk.len() > self.max {
            self.skipping = Some((self.buf.len() + chunk.len()) as u64);
            let keep = HEAD_LEN.saturating_sub(self.buf.len()).min(chunk.len());
            self.buf.extend_from_slice(&chunk[..keep]);
            self.buf.truncate(HEAD_LEN);
            self.buf.shrink_to_fit();
        } else {
            self.buf.extend_from_slice(chunk);
        }
    }

    fn finish(&mut self, terminated: bool) -> Split {
        let bytes = core::mem::take(&mut self.buf);
        match self.skipping.take() {
            Some(len) => Err(LineTooLong {
                len,
                limit: self.max,
                end: self.pos,
                head: bytes,
                terminated,
            }),
            None => Ok(Line {
                bytes,
                end: self.pos,
                terminated,
            }),
        }
    }
}

fn split_result(s: Option<Split>) -> Result<Option<Line>, ReadError> {
    match s {
        None => Ok(None),
        Some(Ok(line)) => Ok(Some(line)),
        Some(Err(e)) => Err(ReadError::TooLong(e)),
    }
}

/// Bounded lines from a [`std::io::Read`] (blocking).
#[derive(Debug)]
pub struct LineReader<R> {
    r: io::BufReader<R>,
    s: Splitter,
}

impl<R: io::Read> LineReader<R> {
    /// Lines of `r`, at most [`DEFAULT_MAX_LINE`] long, offsets from 0.
    pub fn new(r: R) -> Self {
        Self {
            r: io::BufReader::new(r),
            s: Splitter::new(DEFAULT_MAX_LINE, 0),
        }
    }

    /// Lines at most `max` bytes long (without the newline).
    #[must_use]
    pub fn max_line_len(mut self, max: usize) -> Self {
        self.s.max = max;
        self
    }

    /// Count offsets from `start` (where `r` begins in a file read from
    /// that offset).
    #[must_use]
    pub fn start_offset(mut self, start: u64) -> Self {
        self.s.pos = start;
        self
    }

    /// The offset just past the last byte read.
    pub fn position(&self) -> u64 {
        self.s.pos
    }

    /// The next line; `Ok(None)` at EOF.
    ///
    /// # Errors
    /// [`ReadError::TooLong`] for a line over the limit (call again for the
    /// next one), [`ReadError::Io`] when the read fails.
    pub fn next_line(&mut self) -> Result<Option<Line>, ReadError> {
        loop {
            let avail = match io::BufRead::fill_buf(&mut self.r) {
                Ok(a) => a,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };
            if avail.is_empty() {
                return split_result(self.s.eof());
            }
            let (n, line) = self.s.feed(avail);
            io::BufRead::consume(&mut self.r, n);
            if line.is_some() {
                return split_result(line);
            }
        }
    }
}

#[cfg(feature = "runtime")]
pub use self::runtime::{AsyncLineReader, FrameReader, FrameWriter};

#[cfg(feature = "runtime")]
mod runtime {
    use std::io;

    use tokio::io::{AsyncBufReadExt as _, AsyncRead, AsyncWrite, AsyncWriteExt as _, BufReader};

    use super::{
        DEFAULT_MAX_LINE, Line, ReadError, Splitter, encode_line, parse_line, split_result,
    };
    use crate::protocol::Inbound;

    /// Bounded lines from a [`tokio::io::AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html).
    ///
    /// [`Self::next_line`] is cancel safe: dropping it mid-read loses
    /// nothing, the next call carries on.
    #[derive(Debug)]
    pub struct AsyncLineReader<R> {
        r: BufReader<R>,
        s: Splitter,
    }

    impl<R: AsyncRead + Unpin> AsyncLineReader<R> {
        /// Lines of `r`, at most [`DEFAULT_MAX_LINE`] long, offsets from 0.
        pub fn new(r: R) -> Self {
            Self {
                r: BufReader::new(r),
                s: Splitter::new(DEFAULT_MAX_LINE, 0),
            }
        }

        /// Lines at most `max` bytes long (without the newline).
        #[must_use]
        pub fn max_line_len(mut self, max: usize) -> Self {
            self.s.max = max;
            self
        }

        /// Count offsets from `start` (where `r` begins in a file read from
        /// that offset).
        #[must_use]
        pub fn start_offset(mut self, start: u64) -> Self {
            self.s.pos = start;
            self
        }

        /// The offset just past the last byte read.
        pub fn position(&self) -> u64 {
            self.s.pos
        }

        /// The next line; `Ok(None)` at EOF.
        ///
        /// # Errors
        /// [`ReadError::TooLong`] for a line over the limit (call again for
        /// the next one), [`ReadError::Io`] when the read fails.
        pub async fn next_line(&mut self) -> Result<Option<Line>, ReadError> {
            loop {
                let avail = match self.r.fill_buf().await {
                    Ok(a) => a,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e.into()),
                };
                if avail.is_empty() {
                    return split_result(self.s.eof());
                }
                let (n, line) = self.s.feed(avail);
                self.r.consume(n);
                if line.is_some() {
                    return split_result(line);
                }
            }
        }
    }

    /// [`Inbound`] frames from a [`tokio::io::AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html), one per line.
    ///
    /// Blank lines are skipped; a line that isn't a JSON object, or is too
    /// long, is an [`Inbound::Malformed`] ([`super::parse_line`],
    /// [`super::LineTooLong::to_inbound`]). Cancel safe, like
    /// [`AsyncLineReader::next_line`].
    #[derive(Debug)]
    pub struct FrameReader<R> {
        lines: AsyncLineReader<R>,
    }

    impl<R: AsyncRead + Unpin> FrameReader<R> {
        /// Frames of `r`, lines at most [`DEFAULT_MAX_LINE`] long.
        pub fn new(r: R) -> Self {
            Self::from_lines(AsyncLineReader::new(r))
        }

        /// Frames of an [`AsyncLineReader`](https://docs.rs/ccwire/latest/ccwire/framing/struct.AsyncLineReader.html) (to set its limit and offset).
        pub fn from_lines(lines: AsyncLineReader<R>) -> Self {
            Self { lines }
        }

        /// The offset just past the last byte read.
        pub fn position(&self) -> u64 {
            self.lines.position()
        }

        /// The next frame; `Ok(None)` at EOF.
        ///
        /// # Errors
        /// The read failed (the stream is done).
        pub async fn next(&mut self) -> io::Result<Option<Inbound>> {
            Ok(self.next_at().await?.map(|(frame, _)| frame))
        }

        /// The next frame and the offset just past its line; `Ok(None)` at
        /// EOF.
        ///
        /// # Errors
        /// The read failed (the stream is done).
        pub async fn next_at(&mut self) -> io::Result<Option<(Inbound, u64)>> {
            loop {
                match self.lines.next_line().await {
                    Ok(None) => return Ok(None),
                    Ok(Some(line)) => {
                        if let Some(frame) = parse_line(&line.bytes) {
                            return Ok(Some((frame, line.end)));
                        }
                    }
                    Err(ReadError::TooLong(e)) => return Ok(Some((e.to_inbound(), e.end))),
                    Err(ReadError::Io(e)) => return Err(e),
                }
            }
        }
    }

    /// Frames to a [`tokio::io::AsyncWrite`], one JSON line each, flushed.
    #[derive(Debug)]
    pub struct FrameWriter<W> {
        w: W,
    }

    impl<W: AsyncWrite + Unpin> FrameWriter<W> {
        pub fn new(w: W) -> Self {
            Self { w }
        }

        /// Write `frame` as one line and flush.
        ///
        /// Not cancel safe: dropped mid-write, it can leave half a line.
        /// (A [`crate::session::Session`] writes from one task, so its
        /// callers can drop their futures.)
        ///
        /// # Errors
        /// It doesn't serialize ([`io::ErrorKind::InvalidData`]), or the
        /// write fails.
        pub async fn write<T: serde::Serialize + ?Sized>(&mut self, frame: &T) -> io::Result<()> {
            let line =
                encode_line(frame).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            self.write_line(&line).await
        }

        /// Write an encoded line ([`super::encode_line`]) and flush.
        ///
        /// # Errors
        /// The write fails.
        pub async fn write_line(&mut self, line: &[u8]) -> io::Result<()> {
            self.w.write_all(line).await?;
            self.w.flush().await
        }

        /// Shut the writer down (for a child's stdin: EOF to the CLI).
        ///
        /// # Errors
        /// The shutdown fails.
        pub async fn shutdown(&mut self) -> io::Result<()> {
            self.w.shutdown().await
        }

        /// The writer.
        pub fn into_inner(self) -> W {
            self.w
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads one byte at a time: every line arrives split.
    struct Trickle<'a>(&'a [u8]);

    impl io::Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let Some((&b, rest)) = self.0.split_first() else {
                return Ok(0);
            };
            if buf.is_empty() {
                return Ok(0);
            }
            buf[0] = b;
            self.0 = rest;
            Ok(1)
        }
    }

    fn all<R: io::Read>(mut r: LineReader<R>) -> Vec<Result<Line, String>> {
        let mut out = Vec::new();
        loop {
            match r.next_line() {
                Ok(None) => return out,
                Ok(Some(l)) => out.push(Ok(l)),
                Err(e) => out.push(Err(e.to_string())),
            }
        }
    }

    fn line(s: &str, end: u64, terminated: bool) -> Result<Line, String> {
        Ok(Line {
            bytes: s.as_bytes().to_vec(),
            end,
            terminated,
        })
    }

    #[test]
    fn splits_whatever_the_read_size() {
        let input = b"{\"a\":1}\n\n{\"b\":2}\r\nlast";
        let want = vec![
            line("{\"a\":1}", 8, true),
            line("", 9, true),
            line("{\"b\":2}\r", 18, true),
            line("last", 22, false),
        ];
        assert_eq!(all(LineReader::new(&input[..])), want);
        assert_eq!(all(LineReader::new(Trickle(input))), want);
    }

    #[test]
    fn clean_eof_after_a_newline() {
        assert_eq!(all(LineReader::new(&b"x\n"[..])), vec![line("x", 2, true)]);
        assert_eq!(all(LineReader::new(&b""[..])), []);
    }

    #[test]
    fn a_long_line_is_skipped_and_reading_goes_on() {
        let long = format!("{{\"type\":\"result\",\"pad\":\"{}\"}}", "x".repeat(3000));
        let input = format!("ok\n{long}\nnext\n{long}");
        for r in [
            all(LineReader::new(input.as_bytes()).max_line_len(100)),
            all(LineReader::new(Trickle(input.as_bytes())).max_line_len(100)),
        ] {
            assert_eq!(r.len(), 4);
            assert_eq!(r[0], line("ok", 3, true));
            assert!(r[1].as_ref().is_err_and(|e| e.contains("too long")));
            assert_eq!(r[2], line("next", 3 + long.len() as u64 + 1 + 5, true));
            assert!(r[3].is_err());
        }
        let mut r = LineReader::new(input.as_bytes()).max_line_len(100);
        r.next_line().unwrap();
        let Err(ReadError::TooLong(e)) = r.next_line() else {
            panic!("not too long")
        };
        assert_eq!(e.len, long.len() as u64);
        assert_eq!(e.limit, 100);
        assert_eq!(e.head.len(), HEAD_LEN);
        assert!(e.terminated);
        assert!(
            e.to_inbound().ends_turn(),
            "a giant result still ends the turn"
        );
    }

    #[test]
    fn a_line_at_the_limit_is_kept() {
        let r = all(LineReader::new(&b"abcd\nabcde\n"[..]).max_line_len(4));
        assert_eq!(r[0], line("abcd", 5, true));
        assert!(r[1].is_err());
    }

    #[test]
    fn offsets_count_from_the_start() {
        let mut r = LineReader::new(&b"a\nb\n"[..]).start_offset(100);
        assert_eq!(r.next_line().unwrap().unwrap().end, 102);
        assert_eq!(r.next_line().unwrap().unwrap().end, 104);
        assert_eq!(r.position(), 104);
    }

    #[test]
    fn parse_line_never_fails() {
        assert!(parse_line(b"  \r").is_none());
        let Some(Inbound::Malformed { type_, error, raw }) = parse_line(b"garbage") else {
            panic!()
        };
        assert_eq!(type_, "");
        assert!(error.starts_with("invalid JSON"));
        assert_eq!(raw, Value::String("garbage".into()));
        let Some(Inbound::Malformed { error, .. }) = parse_line(b"[1]") else {
            panic!()
        };
        assert_eq!(error, "not a JSON object");
        let cut = parse_line(br#"{"type":"result","subtype":"succ"#).unwrap();
        assert!(cut.ends_turn(), "a cut-off result still ends the turn");
        assert!(matches!(
            parse_line(br#"{"type":"brand_new"}"#),
            Some(Inbound::Other { .. })
        ));
    }

    #[test]
    fn sniffs_the_top_level_type() {
        assert_eq!(sniff_type(br#" {"type":"user","x":1"#), "user");
        assert_eq!(
            sniff_type(br#"{"a":{"type":"no"},"b":"\"type\"","type" : "result","#),
            "result"
        );
        assert_eq!(sniff_type(br#"{"x":{"type":"user"}"#), "");
        assert_eq!(sniff_type(br#"{"x":"cut off"#), "");
        assert_eq!(sniff_type(br#"{"type":"us"#), "");
        assert_eq!(sniff_type(br#"{"type":"a\"b"}"#), "");
        assert_eq!(sniff_type(b"garbage"), "");
    }

    #[test]
    fn encodes_one_line() {
        let l = encode_line(&serde_json::json!({"t": "a\nb"})).unwrap();
        assert_eq!(l, b"{\"t\":\"a\\nb\"}\n");
    }
}
