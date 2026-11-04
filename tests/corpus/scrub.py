"""Scrub stream-json transcripts for the golden corpus.

Usage: python3 -I scrub.py [--word OLD=NEW ...] OUT.jsonl IN.jsonl [IN.jsonl ...]

Replaces UUIDs and API ids (msg_, req_, toolu_ longer than 12 characters)
with stable placeholders, home and /srv paths with neutral ones, each
`--word` OLD (case-insensitive: a host's or a person's name) with NEW, and
drops lines that aren't JSON objects. Ids are numbered per output file, in order of
first appearance, so references between lines (tool_use_id, session_id) stay
consistent.
"""

import json
import re
import sys

UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
API_ID = re.compile(r"\b(msg|req|toolu)_[A-Za-z0-9]{13,}")
WORDS = [
    (re.compile(r"/home/[a-z0-9_-]+"), "/home/user"),
    (re.compile(r"/srv/[a-z0-9_/-]+"), "/srv/data"),
]


def main() -> None:
    args = sys.argv[1:]
    words = list(WORDS)
    while args and args[0] == "--word":
        old, new = args[1].split("=", 1)
        words.append((re.compile(re.escape(old), re.IGNORECASE), new))
        args = args[2:]
    out, *inputs = args
    seen: dict[str, str] = {}

    def uuid(m: re.Match[str]) -> str:
        n = seen.setdefault(m.group(0), str(len(seen) + 1))
        return f"00000000-0000-4000-8000-{int(n):012d}"

    def api_id(m: re.Match[str]) -> str:
        n = seen.setdefault(m.group(0), str(len(seen) + 1))
        return f"{m.group(1)}_{int(n):024d}"

    lines = []
    for path in inputs:
        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line.startswith("{"):
                    continue
                json.loads(line)  # must be valid
                line = UUID.sub(uuid, line)
                line = API_ID.sub(api_id, line)
                for pat, repl in words:
                    line = pat.sub(repl, line)
                json.loads(line)
                lines.append(line)
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
