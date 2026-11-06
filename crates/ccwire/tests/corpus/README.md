# Golden corpus

Stream-json transcripts the tests parse line by line (`tests/corpus.rs`): every
line parses, the core types never come out as `Other` or `Malformed`, and each
frame serializes back to the same JSON value.

- `stream.jsonl`, `multiturn.jsonl`, `partial-*.jsonl`, `subagents-*.jsonl`,
  `activity.jsonl`: recorded from Claude Code 2.1.x runs, some trimmed by
  hand (`stream.jsonl` line 11 got back the `id` its `tool_use` lost), and
  scrubbed with `scrub.py` (ids renumbered, paths and names neutral).
- `synthetic.jsonl`: written by hand after the official SDKs' shapes, for what
  the recordings lack (control requests and responses, cancel, thinking,
  images, error results).

To add a recording: `python3 -I scrub.py [--word NAME=app] NEW.jsonl RECORDED.jsonl`, check
it by eye for anything personal, and add the file to `FILES` in `tests/corpus.rs`.
