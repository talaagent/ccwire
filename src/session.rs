//! A session with the CLI over any reader and writer (feature `runtime`).
//!
//! [`Session`] speaks stream-json over an [`AsyncRead`] (the CLI's output)
//! and an [`AsyncWrite`] (its input): a child's stdout and stdin, a file
//! read from an offset, a socket. It isn't tied to a process: whatever
//! starts and reaps the CLI sits on top.
//!
//! - [`SessionHandle::send`] writes a user message; [`Session::next`] reads
//!   the frames the CLI writes, as [`Inbound`].
//! - [`SessionHandle::request`] is a control round trip: the request gets a
//!   fresh `request_id`, and the `control_response` with that id resolves
//!   it (and isn't passed on to the stream).
//! - The CLI's own control requests (`can_use_tool`, `hook_callback`,
//!   `mcp_message`) go to a [`ControlHandler`] when one is set, each on a
//!   task of its own; else they come out of the stream, to answer with
//!   [`SessionHandle::respond`]. A `control_cancel_request` aborts the
//!   handler's task (or drops the request), and no answer is written.
//! - Turns: a user message or a `system/init` starts one, a `result` (even
//!   a [`Malformed`](Inbound::Malformed) one) ends it. A user message
//!   written mid-turn is either absorbed into that turn or starts the next
//!   one; the CLI's `command_lifecycle` events (for messages with a uuid,
//!   which [`SessionHandle::send`] adds when missing) say which, so the
//!   session counts every message in flight until it is done. A CLI that
//!   sends no lifecycle events gets the plain rule: a `result` ends
//!   everything sent before it. [`SessionState`] says where the session is.
//! - Reattaching ([`SessionBuilder::replay_until`]): the frames up to an
//!   offset, which an earlier reader handled already, only update the
//!   bookkeeping (turn, lifecycle) and come out of the stream; nothing is
//!   answered, and no control request among them stays open.
//! - The run-end rule ([`SessionHandle::finish`]): without callbacks, the
//!   input is closed at once; with them (a handler, or
//!   [`SessionBuilder::answers_control_requests`]) it stays open until the
//!   session is idle (no turn, no control request open either way), since
//!   the CLI may still ask something before the result.
//!
//! All writes go through one task, so a caller can drop a `send` or
//! `request` future at any point without leaving half a line. Dropping the
//! [`Session`] aborts its tasks (reader, writer, handlers) and so drops the
//! reader and the writer; [`Session::close`] ends it gracefully.

use alloc::sync::Arc;
use core::{
    fmt,
    future::Future,
    hash::{BuildHasher as _, Hash as _, Hasher as _},
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, RandomState},
    io,
    sync::{Mutex, MutexGuard, PoisonError},
    time::SystemTime,
};

use serde_json::{Map, Value};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, oneshot, watch},
    task::{AbortHandle, JoinHandle},
};

use crate::{
    framing::{AsyncLineReader, DEFAULT_MAX_LINE, FrameReader, FrameWriter, encode_line},
    protocol::{
        CommandLifecycle, ControlCancelRequest, ControlRequest, ControlRequestBody,
        ControlResponse, ControlResponseBody, Inbound, InterruptRequest, SetModelRequest,
        SetPermissionModeRequest, StopTaskRequest, System, UserInput,
    },
};

/// What a [`ControlHandler`] answers: `Ok(payload)` is a `success`
/// response (a [`PermissionResult`](crate::protocol::PermissionResult) for
/// `can_use_tool`, a hook's output, a JSON-RPC reply), `Err(message)` an
/// `error` one.
pub type ControlReply = Result<Option<Value>, String>;

/// The future a [`ControlHandler`] returns.
pub type ControlFuture = Pin<Box<dyn Future<Output = ControlReply> + Send>>;

/// Answers the CLI's control requests (`can_use_tool`, `hook_callback`,
/// `mcp_message`, ...). Each request runs on a task of its own, so a slow
/// answer never holds up reading; a `control_cancel_request` aborts it.
///
/// Implemented for closures `Fn(ControlRequest) -> ControlFuture`.
pub trait ControlHandler: Send + Sync + 'static {
    /// The answer to `request`.
    ///
    /// Called on the session's reader task: it must return at once, without
    /// blocking or doing the work itself. The work goes in the future it
    /// returns, which runs on a task of its own.
    fn handle(&self, request: ControlRequest) -> ControlFuture;
}

impl<F> ControlHandler for F
where
    F: Fn(ControlRequest) -> ControlFuture + Send + Sync + 'static,
{
    fn handle(&self, request: ControlRequest) -> ControlFuture {
        self(request)
    }
}

/// Where a session is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionState {
    /// Between turns.
    Idle,
    /// A turn is running: a user message was sent or a `system/init` came,
    /// and no `result` yet.
    InTurn,
    /// The CLI's output ended (or the session was dropped).
    Closed,
}

/// A session error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Writing to the CLI failed; its input is closed from then on.
    #[error("writing to the CLI: {0}")]
    Write(#[source] io::Error),
    /// Reading the CLI's output failed; the stream is over.
    #[error("reading from the CLI: {0}")]
    Read(#[source] io::Error),
    /// The CLI's input is closed ([`SessionHandle::finish`],
    /// [`SessionHandle::end_input`], or a failed write).
    #[error("the CLI's input is closed")]
    InputClosed,
    /// The session ended before the answer came.
    #[error("the session is closed")]
    Closed,
    /// The CLI's output ended in the middle of a turn (it exited or was
    /// killed before its `result`).
    #[error("the CLI's output ended mid-turn")]
    EndedMidTurn,
    /// The CLI answered a control request with an error (or a response of
    /// a subtype this version doesn't know).
    #[error("control request {request_id} failed: {message}")]
    Control { request_id: String, message: String },
    /// A frame didn't serialize.
    #[error("serializing a frame: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Options for a [`Session`]: `Session::builder()...start(reader, writer)`.
#[derive(Clone)]
pub struct SessionBuilder {
    max_line: usize,
    start_offset: u64,
    replay_until: u64,
    handler: Option<Arc<dyn ControlHandler>>,
    answers_control: bool,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            max_line: DEFAULT_MAX_LINE,
            start_offset: 0,
            replay_until: 0,
            handler: None,
            answers_control: false,
        }
    }
}

impl fmt::Debug for SessionBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionBuilder")
            .field("max_line", &self.max_line)
            .field("start_offset", &self.start_offset)
            .field("replay_until", &self.replay_until)
            .field("handler", &self.handler.is_some())
            .field("answers_control", &self.answers_control)
            .finish()
    }
}

impl SessionBuilder {
    /// The longest line read (default [`DEFAULT_MAX_LINE`]); a longer one
    /// comes out as an [`Inbound::Malformed`].
    #[must_use]
    pub fn max_line_len(mut self, max: usize) -> Self {
        self.max_line = max;
        self
    }

    /// The offset the reader starts at, for [`Session::next_at`] (a file
    /// read from where an earlier reader stopped).
    #[must_use]
    pub fn start_offset(mut self, offset: u64) -> Self {
        self.start_offset = offset;
        self
    }

    /// Reattach to a CLI an earlier reader followed up to `offset` (counted
    /// like [`Session::next_at`]'s): replay the frames that end at or
    /// before it with no side effects, then go on live.
    ///
    /// A replayed frame only updates the bookkeeping: a `system/init` or a
    /// `result` (the turn), `command_lifecycle` (the messages in flight),
    /// the session id. Its control requests go to no handler and don't stay
    /// open (the earlier reader answered them), its cancels abort nothing,
    /// and its control responses resolve nothing. Every replayed frame
    /// still comes out of the stream, to rebuild the caller's own state:
    /// [`Session::next_at`]'s offset tells them apart (`<= offset`).
    ///
    /// Read from the start (`start_offset` 0, the default) so the turn a
    /// detached CLI is in is known: an EOF then still says
    /// [`Error::EndedMidTurn`].
    ///
    /// The caller's contract: `offset` must only cover what was fully
    /// handled. A control request the earlier reader read but never
    /// answered (it died in between) is replayed like any other, so
    /// nobody answers it, and the CLI, which only sees answers on its
    /// input, waits on it forever. A caller that answers control requests
    /// must therefore advance the offset it persists past a request only
    /// once its answer is written ([`SessionHandle::respond`] returned, or
    /// the handler's answer went out), never when the request is read.
    #[must_use]
    pub fn replay_until(mut self, offset: u64) -> Self {
        self.replay_until = offset;
        self
    }

    /// Answer the CLI's control requests with `handler` (they don't come
    /// out of the stream then). Counts as callbacks for the run-end rule.
    #[must_use]
    pub fn control_handler(mut self, handler: impl ControlHandler) -> Self {
        self.handler = Some(Arc::new(handler));
        self
    }

    /// The caller answers the CLI's control requests itself, from the
    /// stream ([`SessionHandle::respond`]): counts as callbacks for the
    /// run-end rule, so [`SessionHandle::finish`] waits until idle.
    #[must_use]
    pub fn answers_control_requests(mut self, yes: bool) -> Self {
        self.answers_control = yes;
        self
    }

    /// Start the session over `reader` (the CLI's output) and `writer` (its
    /// input). Spawns its reader and writer tasks.
    ///
    /// # Panics
    /// Outside a tokio runtime.
    pub fn start<R, W>(self, reader: R, writer: W) -> Session
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (write_tx, write_rx) = mpsc::unbounded_channel();
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        let (state, _) = watch::channel(SessionState::Idle);
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner::default()),
            state,
            writer: write_tx,
            errors: out_tx.downgrade(),
            next_id: AtomicU64::new(1),
            nonce: format!("{:08x}", random_u64() & 0xffff_ffff),
            replay_until: self.replay_until,
            callbacks: self.handler.is_some() || self.answers_control,
            handler: self.handler,
        });
        let frames = FrameReader::from_lines(
            AsyncLineReader::new(reader)
                .max_line_len(self.max_line)
                .start_offset(self.start_offset),
        );
        let reader = tokio::spawn(read_loop(Arc::clone(&shared), frames, out_tx));
        let writer = tokio::spawn(write_loop(
            Arc::clone(&shared),
            FrameWriter::new(writer),
            write_rx,
        ));
        Session {
            handle: SessionHandle { shared },
            rx: out_rx,
            reader,
            writer,
        }
    }
}

/// A session with the CLI: the frames it writes ([`Self::next`]), and,
/// through [`SessionHandle`] (which it derefs to), what to write to it.
///
/// Dropping it aborts its tasks; see the [module docs](self).
pub struct Session {
    handle: SessionHandle,
    rx: mpsc::UnboundedReceiver<Item>,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}

/// What the reader task passes on: a frame and the offset past it, or the
/// error that ended the stream (last).
type Item = Result<(Inbound, u64), Error>;

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("state", &self.state())
            .finish_non_exhaustive()
    }
}

impl Session {
    /// A session with the default options over `reader` and `writer`.
    ///
    /// # Panics
    /// Outside a tokio runtime.
    pub fn new<R, W>(reader: R, writer: W) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        SessionBuilder::default().start(reader, writer)
    }

    /// Options for a session.
    pub fn builder() -> SessionBuilder {
        SessionBuilder::default()
    }

    /// A handle to write to the session from elsewhere (another task).
    pub fn handle(&self) -> SessionHandle {
        self.handle.clone()
    }

    /// The next frame the CLI wrote; `None` once its output has ended.
    ///
    /// Control responses to this session's requests, and the requests a
    /// [`ControlHandler`] answers (and their cancels), aren't passed on;
    /// everything else is, `Other` and `Malformed` included.
    ///
    /// The last item before `None` is an `Err` when the output ended badly:
    /// [`Error::Read`], or [`Error::EndedMidTurn`] at EOF in a turn. An
    /// [`Error::Write`] nobody awaited (a [`ControlHandler`]'s answer, or
    /// closing the CLI's input) comes out mid-stream, and the stream goes
    /// on after it; so does an [`Error::InputClosed`] for a handler's answer
    /// the closed input couldn't take.
    ///
    /// Frames wait here, unbounded, until read: read them, or a control
    /// round trip awaited while nothing reads could never be answered.
    /// Cancel safe.
    pub async fn next(&mut self) -> Option<Result<Inbound, Error>> {
        Some(self.rx.recv().await?.map(|(frame, _)| frame))
    }

    /// [`Self::next`], with the offset just past the frame's line (counted
    /// from [`SessionBuilder::start_offset`]).
    pub async fn next_at(&mut self) -> Option<Result<(Inbound, u64), Error>> {
        self.rx.recv().await
    }

    /// [`Self::next`] as a poll, to wrap the session in a `Stream`.
    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Inbound, Error>>> {
        self.rx
            .poll_recv(cx)
            .map(|item| item.map(|r| r.map(|(frame, _)| frame)))
    }

    /// End the session gracefully: close the CLI's input, read its output
    /// to the end (frames not read yet are dropped), and wait for the
    /// tasks.
    ///
    /// # Errors
    /// The error the output ended with, if any (an [`Error::EndedMidTurn`]
    /// when the CLI stopped mid-turn).
    pub async fn close(mut self) -> Result<(), Error> {
        self.handle.end_input();
        let mut err = None;
        while let Some(item) = self.rx.recv().await {
            if let Err(e) = item {
                err.get_or_insert(e);
            }
        }
        // Both end on their own once the output and the input are closed.
        let _ = (&mut self.reader).await;
        let _ = (&mut self.writer).await;
        err.map_or(Ok(()), Err)
    }
}

impl core::ops::Deref for Session {
    type Target = SessionHandle;

    fn deref(&self) -> &SessionHandle {
        &self.handle
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.reader.abort();
        self.writer.abort();
        let shared = &self.handle.shared;
        let mut inner = shared.lock();
        inner.closed = true;
        inner.input_closed = true;
        inner.pending.clear();
        for (_, task) in inner.incoming.drain() {
            if let Some(task) = task {
                task.abort();
            }
        }
        shared.settle(&mut inner);
    }
}

/// Writes to a [`Session`]: user messages, control requests and answers.
/// Cheap to clone; every clone talks to the same session.
#[derive(Clone)]
pub struct SessionHandle {
    shared: Arc<Shared>,
}

impl fmt::Debug for SessionHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandle")
            .field("state", &self.state())
            .finish_non_exhaustive()
    }
}

impl SessionHandle {
    /// Send a user message. Between turns it starts one; mid-turn the CLI
    /// takes it in at its next step. Returns once it is written.
    ///
    /// A message without a uuid gets a fresh one, so the CLI's
    /// `command_lifecycle` events can say when it is done. It counts as in
    /// flight from when it is written (not if the write fails).
    ///
    /// # Errors
    /// [`Error::InputClosed`], [`Error::Write`], [`Error::Closed`], or
    /// [`Error::Serialize`] for content that doesn't serialize.
    pub async fn send(&self, input: impl Into<UserInput>) -> Result<(), Error> {
        let mut input = input.into();
        let uuid = input.uuid.get_or_insert_with(new_uuid).clone();
        let line = encode_line(&input)?;
        let (ack, done) = oneshot::channel();
        self.shared
            .queue(&self.shared.lock(), line, Some(ack), Some(uuid))?;
        wait_written(done).await
    }

    /// Write any frame as it is, with no bookkeeping (no turn, no request
    /// id): the escape hatch for what this version doesn't model.
    ///
    /// # Errors
    /// As [`Self::send`].
    pub async fn send_raw<T: serde::Serialize + ?Sized>(&self, frame: &T) -> Result<(), Error> {
        let line = encode_line(frame)?;
        let (ack, done) = oneshot::channel();
        self.shared
            .queue(&self.shared.lock(), line, Some(ack), None)?;
        wait_written(done).await
    }

    /// Send a control request and wait for its answer: the `response`
    /// payload of a `success`, if it had one.
    ///
    /// There is no timeout: wrap it in one. Dropping the future before the
    /// answer forgets the request and sends the CLI a
    /// `control_cancel_request` for it (a late answer comes out of the
    /// stream instead).
    ///
    /// Its `request_id` is `req_{n}_{nonce}`: `n` counts this session's
    /// requests, and the nonce is the session's own, so an answer meant for
    /// another session's request never resolves one of this one's.
    ///
    /// # Errors
    /// [`Error::Control`] when the CLI answers with an error; [`Error::Closed`]
    /// when the session ends first; as [`Self::send`] for the write.
    pub async fn request(
        &self,
        body: impl Into<ControlRequestBody>,
    ) -> Result<Option<Value>, Error> {
        let n = self.shared.next_id.fetch_add(1, Ordering::Relaxed);
        let request_id = format!("req_{n}_{}", self.shared.nonce);
        let frame = Inbound::ControlRequest(ControlRequest::new(request_id.clone(), body));
        let line = encode_line(&frame)?;
        let (ack, written) = oneshot::channel();
        let (answer_tx, answer) = oneshot::channel();
        let _pending = {
            let mut inner = self.shared.lock();
            self.shared.queue(&inner, line, Some(ack), None)?;
            inner.pending.insert(request_id.clone(), answer_tx);
            self.shared.settle(&mut inner);
            Pending {
                shared: &self.shared,
                id: &request_id,
            }
        };
        wait_written(written).await?;
        let failed = |message| Error::Control {
            request_id: request_id.clone(),
            message,
        };
        match answer.await.map_err(|_| Error::Closed)? {
            ControlResponseBody::Success(s) => Ok(s.response.into_option()),
            ControlResponseBody::Error(e) => Err(failed(e.error)),
            other => Err(failed(format!(
                "unexpected response subtype {:?}",
                other.tag()
            ))),
        }
    }

    /// [`Self::request`] of a `subtype` this version doesn't model, with
    /// `fields` beside the `subtype`.
    ///
    /// # Errors
    /// As [`Self::request`].
    pub async fn request_raw(
        &self,
        subtype: &str,
        mut fields: Map<String, Value>,
    ) -> Result<Option<Value>, Error> {
        fields.insert("subtype".to_owned(), Value::String(subtype.to_owned()));
        self.request(ControlRequestBody::Other {
            subtype: subtype.to_owned(),
            raw: Value::Object(fields),
        })
        .await
    }

    /// Stop the current turn: an `interrupt` control request, answered once
    /// the CLI has taken it. The turn then ends with a `result` (an
    /// `error_during_execution` one) in the stream.
    ///
    /// # Errors
    /// As [`Self::request`].
    pub async fn interrupt(&self) -> Result<(), Error> {
        self.request(InterruptRequest::default()).await.map(drop)
    }

    /// Switch the model in place (`None`: back to the default).
    ///
    /// # Errors
    /// As [`Self::request`].
    pub async fn set_model(&self, model: Option<&str>) -> Result<(), Error> {
        let req = SetModelRequest {
            model: model.map(str::to_owned).into(),
            ..SetModelRequest::default()
        };
        self.request(req).await.map(drop)
    }

    /// Switch the permission mode (`default`, `acceptEdits`, `plan`, ...).
    ///
    /// # Errors
    /// As [`Self::request`].
    pub async fn set_permission_mode(&self, mode: &str) -> Result<(), Error> {
        let req = SetPermissionModeRequest {
            mode: mode.to_owned(),
            ..SetPermissionModeRequest::default()
        };
        self.request(req).await.map(drop)
    }

    /// Stop a background task.
    ///
    /// # Errors
    /// As [`Self::request`].
    pub async fn stop_task(&self, task_id: &str) -> Result<(), Error> {
        let req = StopTaskRequest {
            task_id: task_id.to_owned(),
            ..StopTaskRequest::default()
        };
        self.request(req).await.map(drop)
    }

    /// Answer a control request the CLI sent (one that came out of the
    /// stream). `Ok(false)`, with nothing written, when it isn't open: it
    /// was answered or cancelled already, or never came.
    ///
    /// # Errors
    /// As [`Self::send`].
    pub async fn respond(&self, response: ControlResponse) -> Result<bool, Error> {
        let Some(id) = response.request_id().map(str::to_owned) else {
            return Err(Error::Control {
                request_id: String::new(),
                message: "a control response without a request_id".to_owned(),
            });
        };
        let line = encode_line(&Inbound::ControlResponse(response))?;
        let (ack, done) = oneshot::channel();
        {
            let mut inner = self.shared.lock();
            if !inner.incoming.contains_key(&id) {
                return Ok(false);
            }
            self.shared.queue(&inner, line, Some(ack), None)?;
            inner.incoming.remove(&id);
            self.shared.settle(&mut inner);
        }
        wait_written(done).await.map(|()| true)
    }

    /// End the run, by the run-end rule: without callbacks (no
    /// [`ControlHandler`], no [`SessionBuilder::answers_control_requests`],
    /// no control request open), close the CLI's input now; with them,
    /// once the session is idle: no turn running, no message in flight and
    /// no control request open either way. The CLI exits after that, and
    /// the stream ends.
    pub fn finish(&self) {
        let mut inner = self.shared.lock();
        inner.finish = true;
        if !self.shared.callbacks && inner.incoming.is_empty() {
            self.shared.close_input(&mut inner);
        }
        self.shared.settle(&mut inner);
    }

    /// Close the CLI's input now (after what is queued), turn or not.
    pub fn end_input(&self) {
        let mut inner = self.shared.lock();
        self.shared.close_input(&mut inner);
        self.shared.settle(&mut inner);
    }

    /// Where the session is.
    pub fn state(&self) -> SessionState {
        *self.shared.state.borrow()
    }

    /// Watch [`Self::state`] change.
    pub fn watch_state(&self) -> watch::Receiver<SessionState> {
        self.shared.state.subscribe()
    }

    /// Idle: no turn running, no message in flight, and no control request
    /// open either way.
    pub fn is_idle(&self) -> bool {
        self.shared.lock().idle()
    }

    /// The session id, from the last `system/init`.
    pub fn session_id(&self) -> Option<String> {
        self.shared.lock().session_id.clone()
    }
}

/// Wait for the writer task to write a line.
async fn wait_written(done: oneshot::Receiver<io::Result<()>>) -> Result<(), Error> {
    match done.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(Error::Write(e)),
        Err(_) => Err(Error::InputClosed),
    }
}

/// Forgets a pending request when its future ends or is dropped; one
/// still waiting for its answer is cancelled with the CLI.
struct Pending<'a> {
    shared: &'a Shared,
    id: &'a str,
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        let mut inner = self.shared.lock();
        // Still there: no answer came, and the session is open.
        if inner.pending.remove(self.id).is_some() {
            let cancel = Inbound::ControlCancelRequest(ControlCancelRequest {
                request_id: self.id.to_owned(),
                ..ControlCancelRequest::default()
            });
            if let Ok(line) = encode_line(&cancel) {
                // The input closed: the CLI won't answer anyway.
                let _ = self.shared.queue(&inner, line, None, None);
            }
        }
        self.shared.settle(&mut inner);
    }
}

/// 64 random bits, from std's per-process random hash keys, the time and a
/// counter: for ids, not for secrets.
fn random_u64() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut h: DefaultHasher = RandomState::new().build_hasher();
    COUNTER.fetch_add(1, Ordering::Relaxed).hash(&mut h);
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
        .hash(&mut h);
    h.finish()
}

/// A random (version 4) UUID, for a user message.
fn new_uuid() -> String {
    let (a, b) = (random_u64(), random_u64());
    let a = (a & !0xf000) | 0x4000;
    let b = (b & !(0b11 << 62)) | (0b10 << 62);
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        a >> 32,
        (a >> 16) & 0xffff,
        a & 0xffff,
        b >> 48,
        b & 0xffff_ffff_ffff
    )
}

enum WriteCmd {
    Line {
        line: Vec<u8>,
        ack: Option<oneshot::Sender<io::Result<()>>>,
        /// A user message's uuid: in flight once written.
        turn: Option<String>,
    },
    Close,
}

struct Shared {
    inner: Mutex<Inner>,
    state: watch::Sender<SessionState>,
    writer: mpsc::UnboundedSender<WriteCmd>,
    /// The stream, for the errors nobody awaits. Weak: it ends with the
    /// reader task.
    errors: mpsc::WeakUnboundedSender<Item>,
    next_id: AtomicU64,
    /// Makes this session's request ids its own.
    nonce: String,
    replay_until: u64,
    callbacks: bool,
    handler: Option<Arc<dyn ControlHandler>>,
}

#[derive(Default)]
struct Inner {
    /// This session's control requests, by id, waiting for their answer.
    pending: HashMap<String, oneshot::Sender<ControlResponseBody>>,
    /// The CLI's control requests not answered yet, by id, with the
    /// handler's task when a handler answers them.
    incoming: HashMap<String, Option<AbortHandle>>,
    /// A turn is running: a `system/init` came, and no `result` yet.
    running: bool,
    /// The user messages in flight (written, not done), by uuid, in order,
    /// each with whether the CLI said it started on it.
    in_flight: Vec<(String, bool)>,
    /// The CLI sends `command_lifecycle` events.
    lifecycle: bool,
    /// [`SessionHandle::finish`] was called.
    finish: bool,
    input_closed: bool,
    closed: bool,
    session_id: Option<String>,
}

impl Inner {
    fn in_turn(&self) -> bool {
        self.running || !self.in_flight.is_empty()
    }

    fn idle(&self) -> bool {
        !self.in_turn() && self.pending.is_empty() && self.incoming.is_empty()
    }

    /// A `result`: the turn is over, and so are the messages it took in.
    fn end_turn(&mut self) {
        self.running = false;
        if self.lifecycle {
            // The ones not started yet start the next turn.
            self.in_flight.retain(|&(_, started)| !started);
        } else {
            self.in_flight.clear();
        }
    }

    /// A `command_lifecycle` event: `queued`, `started` or `completed`.
    fn lifecycle(&mut self, c: &CommandLifecycle) {
        let (Some(uuid), Some(state)) = (c.command_uuid.as_deref(), c.state.as_deref()) else {
            return;
        };
        self.lifecycle = true;
        let at = self.in_flight.iter().position(|(u, _)| u == uuid);
        match (state, at) {
            ("queued", None) => self.in_flight.push((uuid.to_owned(), false)),
            ("started", Some(i)) => self.in_flight[i].1 = true,
            ("started", None) => self.in_flight.push((uuid.to_owned(), true)),
            ("completed", Some(i)) => {
                self.in_flight.remove(i);
            }
            _ => {}
        }
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queue a line for the writer task (in order: everything queued under
    /// the lock is written in that order).
    fn queue(
        &self,
        inner: &Inner,
        line: Vec<u8>,
        ack: Option<oneshot::Sender<io::Result<()>>>,
        turn: Option<String>,
    ) -> Result<(), Error> {
        if inner.input_closed {
            return Err(Error::InputClosed);
        }
        self.writer
            .send(WriteCmd::Line { line, ack, turn })
            .map_err(|_| Error::InputClosed)
    }

    /// Pass an error nobody awaits on to the stream, while it is open.
    fn report(&self, e: Error) {
        if let Some(out) = self.errors.upgrade() {
            let _ = out.send(Err(e));
        }
    }

    fn close_input(&self, inner: &mut Inner) {
        if !inner.input_closed {
            inner.input_closed = true;
            let _ = self.writer.send(WriteCmd::Close);
        }
    }

    /// Publish the state, and close the input if the run is finishing and
    /// now idle. Called after every change.
    fn settle(&self, inner: &mut Inner) {
        let state = if inner.closed {
            SessionState::Closed
        } else if inner.in_turn() {
            SessionState::InTurn
        } else {
            SessionState::Idle
        };
        self.state.send_if_modified(|s| {
            let changed = *s != state;
            *s = state;
            changed
        });
        if inner.finish && inner.idle() {
            self.close_input(inner);
        }
    }
}

async fn write_loop<W: AsyncWrite + Unpin>(
    shared: Arc<Shared>,
    mut w: FrameWriter<W>,
    mut rx: mpsc::UnboundedReceiver<WriteCmd>,
) {
    while let Some(cmd) = rx.recv().await {
        let WriteCmd::Line { line, ack, turn } = cmd else {
            break;
        };
        if let Some(uuid) = &turn {
            // In flight before the CLI can see it, so its answer can't
            // come first.
            let mut inner = shared.lock();
            inner.in_flight.push((uuid.clone(), false));
            shared.settle(&mut inner);
        }
        let r = w.write_line(&line).await;
        let failed = r.is_err();
        if let (true, Some(uuid)) = (failed, &turn) {
            let mut inner = shared.lock();
            inner.in_flight.retain(|(u, _)| u != uuid);
            shared.settle(&mut inner);
        }
        match (ack, r) {
            (Some(ack), r) => {
                let _ = ack.send(r);
            }
            (None, Err(e)) => shared.report(Error::Write(e)),
            (None, Ok(())) => {}
        }
        if failed {
            break;
        }
    }
    {
        let mut inner = shared.lock();
        inner.input_closed = true;
        shared.settle(&mut inner);
    }
    // Lines queued after the close are dropped: their senders see
    // `InputClosed`.
    rx.close();
    if let Err(e) = w.shutdown().await {
        shared.report(Error::Write(e));
    }
}

async fn read_loop<R: AsyncRead + Unpin>(
    shared: Arc<Shared>,
    mut frames: FrameReader<R>,
    out: mpsc::UnboundedSender<Item>,
) {
    let end = loop {
        match frames.next_at().await {
            Ok(Some((frame, at))) => {
                let frame = if at <= shared.replay_until {
                    replay(&shared, frame)
                } else {
                    dispatch(&shared, frame)
                };
                if let Some(frame) = frame {
                    // The receiver gone (session dropped) is fine: this task
                    // is being aborted.
                    let _ = out.send(Ok((frame, at)));
                }
            }
            Ok(None) => break None,
            Err(e) => break Some(Error::Read(e)),
        }
    };
    let mid_turn = {
        let mut inner = shared.lock();
        let mid_turn = inner.in_turn();
        inner.closed = true;
        inner.running = false;
        inner.in_flight.clear();
        // Dropping the senders fails the waiting requests with `Closed`.
        inner.pending.clear();
        for (_, task) in inner.incoming.drain() {
            if let Some(task) = task {
                task.abort();
            }
        }
        shared.close_input(&mut inner);
        shared.settle(&mut inner);
        mid_turn
    };
    if let Some(e) = end.or_else(|| mid_turn.then_some(Error::EndedMidTurn)) {
        let _ = out.send(Err(e));
    }
}

/// Handle a frame's bookkeeping; the frame back if it goes on to the
/// stream.
fn dispatch(shared: &Arc<Shared>, frame: Inbound) -> Option<Inbound> {
    match frame {
        Inbound::ControlResponse(resp) => {
            let mut inner = shared.lock();
            let waiter = resp.request_id().and_then(|id| inner.pending.remove(id));
            let Some(waiter) = waiter else {
                return Some(Inbound::ControlResponse(resp));
            };
            let _ = waiter.send(resp.response);
            shared.settle(&mut inner);
            None
        }
        Inbound::ControlRequest(req) => {
            if let Some(handler) = &shared.handler {
                spawn_handler(shared, handler.as_ref(), req);
                return None;
            }
            let mut inner = shared.lock();
            inner.incoming.insert(req.request_id.clone(), None);
            shared.settle(&mut inner);
            Some(Inbound::ControlRequest(req))
        }
        Inbound::ControlCancelRequest(cancel) => {
            let mut inner = shared.lock();
            let open = inner.incoming.remove(&cancel.request_id);
            shared.settle(&mut inner);
            match open {
                Some(Some(task)) => {
                    task.abort();
                    None
                }
                _ => Some(Inbound::ControlCancelRequest(cancel)),
            }
        }
        frame => {
            track(shared, &frame);
            Some(frame)
        }
    }
}

/// A replayed frame (an earlier reader handled it): bookkeeping only.
fn replay(shared: &Shared, frame: Inbound) -> Option<Inbound> {
    track(shared, &frame);
    Some(frame)
}

/// The turn, lifecycle and session id bookkeeping of a frame.
fn track(shared: &Shared, frame: &Inbound) {
    let mut inner = shared.lock();
    match frame {
        f if f.ends_turn() => inner.end_turn(),
        Inbound::System(System::Init(init)) => {
            inner.running = true;
            if let Some(id) = init.session_id.get() {
                inner.session_id = Some(id.clone());
            }
        }
        Inbound::CommandLifecycle(c) => inner.lifecycle(c),
        _ => return,
    }
    shared.settle(&mut inner);
}

fn spawn_handler(shared: &Arc<Shared>, handler: &dyn ControlHandler, req: ControlRequest) {
    let id = req.request_id.clone();
    {
        let mut inner = shared.lock();
        inner.incoming.insert(id.clone(), None);
        shared.settle(&mut inner);
    }
    let answer = handler.handle(req);
    let guard = Answer {
        shared: Arc::clone(shared),
        id: id.clone(),
        done: false,
    };
    let task = tokio::spawn(async move {
        let reply = answer.await;
        guard.send(reply);
    });
    if let Some(slot) = shared.lock().incoming.get_mut(&id) {
        *slot = Some(task.abort_handle());
    }
}

/// Writes a handler's answer. Dropped without one (the handler panicked,
/// or its task was aborted while the request was still open), it answers
/// with an error, so the CLI isn't left waiting.
struct Answer {
    shared: Arc<Shared>,
    id: String,
    done: bool,
}

impl Answer {
    fn send(mut self, reply: ControlReply) {
        self.write(reply);
        self.done = true;
    }

    fn write(&self, reply: ControlReply) {
        let mut inner = self.shared.lock();
        // Not open any more: cancelled, or the session ended.
        if inner.incoming.remove(&self.id).is_none() {
            return;
        }
        let resp = match reply {
            Ok(payload) => ControlResponse::success(&self.id, payload),
            Err(message) => ControlResponse::error(&self.id, message),
        };
        match encode_line(&Inbound::ControlResponse(resp)) {
            Ok(line) => {
                if let Err(e) = self.shared.queue(&inner, line, None, None) {
                    self.shared.report(e);
                }
            }
            Err(e) => self.shared.report(e.into()),
        }
        self.shared.settle(&mut inner);
    }
}

impl Drop for Answer {
    fn drop(&mut self) {
        if !self.done {
            self.write(Err("the control handler ended without an answer".to_owned()));
        }
    }
}
