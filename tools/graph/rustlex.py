"""Minimal Rust lexical preprocessing, standard library only.

Every downstream analysis in this package works on *stripped* source:
comments and string/char literal bodies replaced by spaces of equal
length. Keeping the length identical is deliberate -- byte offsets stay
valid, so a match found in stripped source can be mapped straight back
to a line number in the original file.

Nothing here is a real Rust parser. It is a lexer good enough to stop
the two things that actually produce wrong answers when you grep Rust
source directly: matching identifiers inside comments, and matching
them inside string literals.
"""

from __future__ import annotations


def strip(source: str) -> str:
    """Blank out comments and literal bodies, preserving length.

    Handles line comments, *nested* block comments (Rust allows them,
    unlike C), plain and raw strings with any hash count, byte strings,
    and char literals -- while not mistaking a lifetime (`'a`) for an
    unterminated char literal, which is the one case a naive scanner
    always gets wrong.
    """
    out = list(source)
    i = 0
    n = len(source)

    def blank(start: int, end: int, keep_newlines: bool = True) -> None:
        for k in range(start, min(end, n)):
            if not (keep_newlines and source[k] == "\n"):
                out[k] = " "

    while i < n:
        c = source[i]
        nxt = source[i + 1] if i + 1 < n else ""

        if c == "/" and nxt == "/":
            j = source.find("\n", i)
            j = n if j == -1 else j
            blank(i, j)
            i = j
            continue

        if c == "/" and nxt == "*":
            depth = 1
            j = i + 2
            while j < n and depth:
                if source[j] == "/" and j + 1 < n and source[j + 1] == "*":
                    depth += 1
                    j += 2
                elif source[j] == "*" and j + 1 < n and source[j + 1] == "/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            blank(i, j)
            i = j
            continue

        # Raw strings: r"...", r#"..."#, br##"..."##
        if c in "rb":
            k = i
            if source[k] == "b" and k + 1 < n and source[k + 1] == "r":
                k += 1
            if source[k] == "r":
                h = k + 1
                while h < n and source[h] == "#":
                    h += 1
                hashes = h - (k + 1)
                if h < n and source[h] == '"':
                    close = '"' + "#" * hashes
                    end = source.find(close, h + 1)
                    end = n if end == -1 else end + len(close)
                    blank(h + 1, end - len(close))
                    i = end
                    continue

        if c == '"':
            j = i + 1
            while j < n:
                if source[j] == "\\":
                    j += 2
                    continue
                if source[j] == '"':
                    break
                j += 1
            blank(i + 1, j)
            i = min(j + 1, n)
            continue

        if c == "'":
            # Char literal or lifetime? A char literal is 'x' or '\n' or
            # '\u{1F600}'. A lifetime is 'ident with no closing quote.
            # Decide by looking for the terminator, not by guessing.
            if i + 2 < n and source[i + 1] == "\\":
                end = source.find("'", i + 2)
                if end != -1 and end - i <= 12:
                    blank(i + 1, end)
                    i = end + 1
                    continue
            elif i + 2 < n and source[i + 2] == "'":
                blank(i + 1, i + 2)
                i = i + 3
                continue
            i += 1
            continue

        i += 1

    return "".join(out)


def match_block(source: str, open_brace: int) -> int:
    """Return the index just past the `}` closing the `{` at open_brace.

    Assumes `source` is already stripped, so braces inside comments and
    strings cannot throw the count off.
    """
    depth = 0
    for j in range(open_brace, len(source)):
        if source[j] == "{":
            depth += 1
        elif source[j] == "}":
            depth -= 1
            if depth == 0:
                return j + 1
    return len(source)


def test_spans(stripped: str) -> list[tuple[int, int]]:
    """Byte spans of `#[cfg(test)]`-gated blocks.

    Test code references production symbols heavily. Counting those
    references identically to production ones would make a symbol that
    only its own unit tests touch look genuinely load-bearing, so the
    two are tracked separately and these spans are how they are told
    apart.
    """
    spans: list[tuple[int, int]] = []
    needle = "#[cfg(test)]"
    at = stripped.find(needle)
    while at != -1:
        brace = stripped.find("{", at)
        semi = stripped.find(";", at)
        if brace != -1 and (semi == -1 or brace < semi):
            spans.append((at, match_block(stripped, brace)))
        at = stripped.find(needle, at + len(needle))
    return spans


def in_spans(offset: int, spans: list[tuple[int, int]]) -> bool:
    return any(a <= offset < b for a, b in spans)


def line_of(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1
