#!/usr/bin/env python3
"""Check that every flag Options::to_argv can emit appears in `claude --help`.

Usage: check_cli_flags.py SRC_OPTIONS_RS HELP_TXT

The flags are read from the source of `to_argv` (every string literal that
starts with `-`), so this needs no build and no credentials. It is a stopgap
until the crate exports its flag list (see the PR that added it).
"""

import re
import sys

# Flags the CLI accepts but leaves out of `--help` (hidden options). They
# can't be checked this way; keep the list short and say why.
HIDDEN = {
    # Hidden in `claude --help` since at least 2.1.x; still honored in -p mode.
    "--max-turns",
    # Hidden in `claude --help`; the official SDKs pass it for `stdio`
    # permission prompts (see Options::permission_prompt_tool).
    "--permission-prompt-tool",
}


def argv_flags(src: str) -> list[str]:
    start = src.index("pub fn to_argv")
    # The body ends at the first line that closes the function (4 spaces, `}`).
    end = src.index("\n    }\n", start)
    body = src[start:end]
    flags = re.findall(r'"(-{1,2}[A-Za-z][A-Za-z0-9-]*)"', body)
    return sorted(set(flags))


def main() -> int:
    src_path, help_path = sys.argv[1], sys.argv[2]
    with open(src_path, encoding="utf-8") as f:
        flags = argv_flags(f.read())
    with open(help_path, encoding="utf-8") as f:
        help_text = f.read()
    if not flags:
        print("::error::found no flags in to_argv; has src/options.rs moved?")
        return 1
    missing = []
    for flag in flags:
        # A whole word: `--resume` must not match `--resume-foo`.
        if flag in HIDDEN:
            print(f"hidden   {flag} (known hidden flag, not checked)")
        elif re.search(r"(?<![A-Za-z0-9-])" + re.escape(flag) + r"(?![A-Za-z0-9-])", help_text):
            print(f"ok       {flag}")
        else:
            print(f"MISSING  {flag}")
            missing.append(flag)
    if missing:
        print(f"::error::claude --help no longer lists: {' '.join(missing)}")
        return 1
    print(f"all {len(flags)} flags found in claude --help")
    return 0


if __name__ == "__main__":
    sys.exit(main())
