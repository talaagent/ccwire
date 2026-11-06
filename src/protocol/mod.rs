//! The stream-json protocol of `claude -p --input-format stream-json
//! --output-format stream-json`, as Rust types.
//!
//! Every line is one JSON object with a `type`. [`Inbound`] is everything the
//! CLI writes; [`Message`] is the part a caller consumes (the conversation),
//! and [`Control`] the control channel. [`UserInput`] is what a caller writes.
//!
//! Forward compatibility:
//! - Every struct keeps the keys it doesn't model in `extra`, so a frame
//!   survives a parse and a serialize unchanged (as a [`serde_json::Value`]).
//! - Optional keys are [`Field`]s, which tell an absent key from `null`.
//! - An unknown `type` (or `subtype`, or content block type) parses to an
//!   `Other` variant that keeps the raw JSON.
//! - A known `type` whose shape doesn't parse becomes
//!   [`Inbound::Malformed`], with the error and the raw JSON: a protocol
//!   change shows up instead of being dropped.
//!
//! Parsing a JSON object as [`Inbound`] never fails.

mod content;
mod control;
mod field;
mod inbound;
mod input;
mod message;
mod partial;
mod result;
pub mod stats;
mod system;

pub use content::{
    ContentBlock, ImageBlock, MessageContent, RedactedThinkingBlock, ServerToolUseBlock, TextBlock,
    ThinkingBlock, ToolResultBlock, ToolResultContent, ToolUseBlock,
};
pub use control::{
    CanUseToolRequest, ControlCancelRequest, ControlError, ControlRequest, ControlRequestBody,
    ControlResponse, ControlResponseBody, ControlSuccess, HookCallbackRequest, InitializeRequest,
    InterruptRequest, McpMessageRequest, PermissionAllow, PermissionDeny, PermissionResult,
    SetModelRequest, SetPermissionModeRequest, StopTaskRequest,
};
pub use field::Field;
pub use inbound::{Control, Frame, Inbound, ParseError};
pub use input::UserInput;
pub use message::{
    ApiMessage, ApiUserMessage, AssistantMessage, CommandLifecycle, Message, RateLimitEvent,
    RateLimitInfo, StreamEvent, UserMessage,
};
pub use partial::{Partial, PartialMessage};
pub use result::{ResultMessage, ResultSubtype, Usage};
pub use system::{
    BackgroundTasksChanged, CompactBoundary, CompactMetadata, McpServerStatus, System, SystemInit,
    SystemStatus, TaskNotification, TaskProgress, TaskStarted, TaskUpdated, TaskUsage,
};

use serde::{Deserialize as _, de::DeserializeOwned};
use serde_json::{Map, Value};

/// The keys of an object that aren't modeled, kept as they came.
pub type Extra = Map<String, Value>;

/// Parse the fields of `obj` other than `skip` (the tags already read) as
/// `T`, borrowing `obj` so the caller keeps it for an error report.
pub(crate) fn from_fields<T: DeserializeOwned>(
    obj: &Map<String, Value>,
    skip: &[&str],
) -> Result<T, serde_json::Error> {
    let fields = obj
        .iter()
        .filter(|(k, _)| !skip.contains(&k.as_str()))
        .map(|(k, v)| (k.as_str(), v));
    T::deserialize(serde::de::value::MapDeserializer::<_, serde_json::Error>::new(fields))
}

/// `inner` serialized as an object, with `key: tag` first (replacing a `key`
/// the object already has).
pub(crate) fn tagged<T: serde::Serialize + ?Sized>(
    key: &str,
    tag: &str,
    inner: &T,
) -> Result<Value, serde_json::Error> {
    let Value::Object(fields) = serde_json::to_value(inner)? else {
        return Err(serde::ser::Error::custom(format_args!(
            "a `{key}: {tag}` body must serialize as an object"
        )));
    };
    let mut out = Map::with_capacity(fields.len() + 1);
    out.insert(key.to_owned(), Value::String(tag.to_owned()));
    for (k, v) in fields {
        if k != key {
            out.insert(k, v);
        }
    }
    Ok(Value::Object(out))
}

/// The string at `key` of `obj`, if it is one.
pub(crate) fn str_of<'a>(obj: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(Value::as_str)
}

/// A JSON object from a deserializer, or an error naming `what`.
pub(crate) fn object<'de, D: serde::Deserializer<'de>>(
    d: D,
    what: &str,
) -> Result<Map<String, Value>, D::Error> {
    match Value::deserialize(d)? {
        Value::Object(m) => Ok(m),
        _ => Err(serde::de::Error::custom(format_args!(
            "{what}: expected a JSON object"
        ))),
    }
}

/// Implements `Serialize` and `Deserialize` for an enum dispatched on a tag
/// (`type` or `subtype`), whose known variants are newtypes of structs and
/// whose unknown ones are `Other { $tag_field: String, raw: Value }`.
macro_rules! tagged_enum {
    (
        $name:ident, key = $key:literal, skip = [$($skip:literal),*],
        other = $tag_field:ident,
        { $($tag:literal => $variant:ident),* $(,)? }
    ) => {
        impl $name {
            /// The tag (`type` or `subtype`) of this variant.
            pub fn tag(&self) -> &str {
                match self {
                    $(Self::$variant(_) => $tag,)*
                    Self::Other { $tag_field, .. } => $tag_field,
                }
            }

            /// Parse a JSON object (borrowed: the caller keeps it).
            pub(crate) fn from_object(
                obj: &::serde_json::Map<String, ::serde_json::Value>,
            ) -> Result<Self, ::serde_json::Error> {
                let skip: &[&str] = &[$key $(, $skip)*];
                Ok(match $crate::protocol::str_of(obj, $key) {
                    $(Some($tag) => Self::$variant($crate::protocol::from_fields(obj, skip)?),)*
                    tag => Self::Other {
                        $tag_field: tag.unwrap_or_default().to_owned(),
                        raw: ::serde_json::Value::Object(obj.clone()),
                    },
                })
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                match self {
                    $(Self::$variant(inner) => $crate::protocol::tagged($key, $tag, inner)
                        .map_err(::serde::ser::Error::custom)?
                        .serialize(s),)*
                    Self::Other { raw, .. } => raw.serialize(s),
                }
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let obj = $crate::protocol::object(d, stringify!($name))?;
                Self::from_object(&obj).map_err(::serde::de::Error::custom)
            }
        }
    };
}
pub(crate) use tagged_enum;
