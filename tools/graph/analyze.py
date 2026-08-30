"""Offline connection analysis for the ADRIAN OS tree.

Builds a reference graph over Rust modules/symbols (and Dart libraries)
using nothing but the Python standard library, so it runs in any
environment that has a Python interpreter -- no network, no toolchain,
no cargo metadata, no LLM calls.

Deliberate design choices, each for a reason:

*   References are counted at *symbol* granularity, not just at `use`
    granularity. This tree routinely writes fully-qualified inline
    paths (`crate::object::KernelObjectId`, `crate::sync::SpinLock`)
    instead of importing, so a `use`-only analysis systematically
    under-reports how connected the kernel actually is.
*   A symbol used only as a struct field type, an enum variant, or a
    generic argument still counts as used. Missing exactly this is the
    documented blind spot that made a previous tool flag
    `ChannelState`, `MessageHeader` and `EventObject` as dead code when
    each has several real usages. `validate.py` asserts we do not
    repeat it.
*   Production and `#[cfg(test)]` references are tracked separately. A
    symbol touched only by its own unit tests is a different situation
    from one the kernel actually depends on, and collapsing them hides
    that.

Usage:
    python3 tools/graph/analyze.py            # writes graph-out/graph.json
    python3 tools/graph/analyze.py --print    # also summarise to stdout
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rustlex  # noqa: E402

REPO_ROOT = os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
)
OUT_DIR = os.path.join(REPO_ROOT, "graph-out")
# `graphify` is a read-only third-party clone kept in the tree for
# reference. Without it here, its own test fixtures (e.g.
# graphify/tests/fixtures/sample.rs) get indexed as if they were part of
# this codebase, which is exactly the kind of false signal this tool
# exists to avoid producing.
SKIP_DIRS = {
    ".git",
    "target",
    "graph-out",
    "graphify-out",
    "graphify",
    ".dart_tool",
    "build",
}

# Kinds of item declaration we index. `impl` is handled separately since
# it names a type rather than declaring a new one.
DECL_RE = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?"
    r"(?:(?:const|async|unsafe|extern[ \t]+\"[^\"]*\")[ \t]+)*"
    r"(fn|struct|enum|trait|union|type|const|static|macro_rules!)"
    r"[ \t]+(?:mut[ \t]+)?([A-Za-z_][A-Za-z0-9_]*)",
    re.MULTILINE,
)
MOD_DECL_RE = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?mod[ \t]+([A-Za-z_][A-Za-z0-9_]*)[ \t]*;",
    re.MULTILINE,
)
USE_RE = re.compile(r"^[ \t]*(?:pub[ \t]+)?use[ \t]+([^;]+);", re.MULTILINE)
# Any `a::b::c` path. The root is classified at resolution time -- it may
# be `crate`/`self`/`super`, another crate in the workspace, an external
# crate we ignore, or a type qualifying an associated item.
PATH_RE = re.compile(
    r"\b([A-Za-z_][A-Za-z0-9_]*)((?:[ \t]*::[ \t]*[A-Za-z_][A-Za-z0-9_]*)+)"
)
IDENT_RE = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*\b")
# `ident` immediately preceded by `::`, with the qualifier captured.
QUALIFIED_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)[ \t]*::[ \t]*$")

RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self",
    "Self", "static", "struct", "super", "trait", "true", "type",
    "union", "unsafe", "use", "where", "while", "await", "async",
}


def read_text(path: str) -> str:
    with open(path, "r", encoding="utf-8-sig", errors="replace") as handle:
        return handle.read()


def walk(root: str, suffix: str) -> list[str]:
    found = []
    for base, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        for name in sorted(files):
            if name.endswith(suffix):
                found.append(os.path.join(base, name))
    return sorted(found)


def workspace_members(root: str) -> list[str]:
    """Read `members` out of the root Cargo.toml.

    Hand-parsed rather than via `tomllib`, which only exists from Python
    3.11 -- this has to run on whatever interpreter is present.
    """
    path = os.path.join(root, "Cargo.toml")
    if not os.path.isfile(path):
        return []
    text = read_text(path)
    block = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
    if not block:
        return []
    return re.findall(r"\"([^\"]+)\"", block.group(1))


def discover_crates(root: str) -> dict:
    """Map crate name -> {dir, src_root, in_workspace}."""
    members = {m.replace("\\", "/").strip("/") for m in workspace_members(root)}
    crates = {}
    for manifest in walk(root, "Cargo.toml"):
        text = read_text(manifest)
        if "[package]" not in text:
            continue
        package = text.split("[package]", 1)[1]
        name_match = re.search(r"^\s*name\s*=\s*\"([^\"]+)\"", package, re.M)
        if not name_match:
            continue
        crate_dir = os.path.dirname(manifest)
        rel = os.path.relpath(crate_dir, root).replace("\\", "/")
        src = os.path.join(crate_dir, "src")
        crates[name_match.group(1)] = {
            "dir": crate_dir,
            "rel": rel,
            "src": src if os.path.isdir(src) else crate_dir,
            "in_workspace": rel in members,
        }
    return crates


def module_id(crate_name: str, src_root: str, path: str) -> str:
    """Rust file path -> module path, e.g. arch/x86_64/idt.rs -> ...::arch::x86_64::idt."""
    rel = os.path.relpath(path, src_root).replace("\\", "/")
    rel = rel[:-3] if rel.endswith(".rs") else rel
    parts = [p for p in rel.split("/") if p]
    if parts and parts[-1] in ("mod", "lib", "main"):
        parts = parts[:-1]
    return "::".join([crate_name] + parts) if parts else crate_name


def brace_depths(stripped: str) -> list[int]:
    """Depth *before* each byte. Lets us keep declaration indexing to
    module top level (depth 0), which is what separates a free function
    like `dispatch_syscall` from the dozens of same-named `new`/`count`
    methods living inside `impl` blocks."""
    depths = [0] * (len(stripped) + 1)
    depth = 0
    for i, ch in enumerate(stripped):
        depths[i] = depth
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth = max(0, depth - 1)
    depths[len(stripped)] = depth
    return depths


IMPL_RE = re.compile(r"\bimpl\b[^{;]*", re.S)


def impl_spans(stripped: str) -> list[tuple[int, int]]:
    """Spans of `impl ... ` headers (up to the opening brace).

    Identifiers here name the type being implemented, not a dependency
    on it. Counting them as references would make every type look used
    by virtue of having its own methods, which would defeat orphan
    detection entirely.
    """
    return [(m.start(), m.end()) for m in IMPL_RE.finditer(stripped)]


def parse_rust(path: str, crate: str, src_root: str) -> dict:
    raw = read_text(path)
    stripped = rustlex.strip(raw)
    tests = rustlex.test_spans(stripped)
    depths = brace_depths(stripped)
    impls = impl_spans(stripped)

    decls = []
    for match in DECL_RE.finditer(stripped):
        offset = match.start(2)
        name = match.group(2)
        # `const _: () = assert!(...)` is a compile-time assertion, not a
        # named item, and `_` matches every closure placeholder and match
        # arm in the tree -- indexing it produces pure noise.
        if depths[match.start()] != 0 or name.startswith("_"):
            continue
        decls.append({
            "name": name,
            "kind": match.group(1).rstrip("!"),
            "line": rustlex.line_of(raw, offset),
            "offset": offset,
            "test_only": rustlex.in_spans(offset, tests),
        })

    uses = []
    for match in USE_RE.finditer(stripped):
        uses.append({
            "spec": " ".join(match.group(1).split()),
            "line": rustlex.line_of(raw, match.start()),
            "test_only": rustlex.in_spans(match.start(), tests),
        })

    paths = []
    for match in PATH_RE.finditer(stripped):
        paths.append({
            "root": match.group(1),
            "segments": [s.strip() for s in match.group(2).split("::") if s.strip()],
            "line": rustlex.line_of(raw, match.start()),
            "test_only": rustlex.in_spans(match.start(), tests),
        })

    return {
        "module": module_id(crate, src_root, path),
        "crate": crate,
        "path": os.path.relpath(path, REPO_ROOT).replace("\\", "/"),
        "loc": raw.count("\n") + 1,
        "test_loc": sum(raw[a:b].count("\n") for a, b in tests),
        "child_mods": sorted(set(MOD_DECL_RE.findall(stripped))),
        "decls": decls,
        "uses": uses,
        "paths": paths,
        "_stripped": stripped,
        "_raw": raw,
        "_tests": tests,
        "_impls": impls,
    }


def resolve_module(
    root: str,
    segments: list[str],
    current: str,
    known: set,
    crate_idents: dict | None = None,
) -> str | None:
    """Resolve a Rust path to a known module id, or None if it points
    outside the tree.

    Handles `crate`/`self`/`super` *and* sibling workspace crates, which
    matters here: `rian/boot-image` reaches the kernel as
    `adrian_kernel::entry::kernel_entry`, so a resolver that only knew
    about `crate::` would report the boot wrapper as depending on
    nothing at all -- the single most important edge in the tree.

    Takes the longest prefix that is actually a module, so the trailing
    symbol in `crate::object::KernelObjectId` resolves to the `object`
    module rather than failing.
    """
    if root == "crate":
        base = [current.split("::")[0]]
    elif root == "self":
        base = current.split("::")
    elif root == "super":
        base = current.split("::")[:-1] or [current.split("::")[0]]
    elif crate_idents and root in crate_idents:
        base = [crate_idents[root]]
    else:
        return None

    parts = base + segments
    for end in range(len(parts), 0, -1):
        candidate = "::".join(parts[:end])
        if candidate in known:
            return candidate
    return None


def decl_spans(parsed: dict) -> list[tuple[dict, int, int]]:
    """Approximate each top-level declaration's body extent, used for
    the intra-module cohesion measure. The next declaration's start is a
    good enough right edge -- precision here would need a real parser
    and would not change which modules look incoherent."""
    decls = sorted(parsed["decls"], key=lambda d: d["offset"])
    spans = []
    for index, decl in enumerate(decls):
        end = decls[index + 1]["offset"] if index + 1 < len(decls) else len(parsed["_stripped"])
        spans.append((decl, decl["offset"], end))
    return spans


def build(root: str) -> dict:
    crates = discover_crates(root)
    files = walk(root, ".rs")

    parsed_files = []
    outside = []
    for path in files:
        owner = None
        for name, info in crates.items():
            common = os.path.commonpath([os.path.abspath(path), os.path.abspath(info["src"])])
            if os.path.abspath(common) == os.path.abspath(info["src"]):
                owner = (name, info)
                break
        if owner is None:
            outside.append(os.path.relpath(path, root).replace("\\", "/"))
            continue
        parsed_files.append(parse_rust(path, owner[0], owner[1]["src"]))

    known = {p["module"] for p in parsed_files}
    crate_idents = {name.replace("-", "_"): name for name in crates}
    return analyse(parsed_files, crates, known, outside, crate_idents)


def analyse(parsed_files: list, crates: dict, known: set, outside: list, crate_idents: dict) -> dict:
    # ---- symbol index -------------------------------------------------
    owners = defaultdict(list)          # name -> [module_id, ...]
    symbols = {}                        # (module, name) -> record
    for parsed in parsed_files:
        for decl in parsed["decls"]:
            key = (parsed["module"], decl["name"])
            if key in symbols:
                continue
            owners[decl["name"]].append(parsed["module"])
            symbols[key] = {
                "name": decl["name"],
                "kind": decl["kind"],
                "module": parsed["module"],
                "file": parsed["path"],
                "line": decl["line"],
                "declared_in_test": decl["test_only"],
                "refs_same_module": 0,
                "refs_cross_module": 0,
                "refs_from_tests": 0,
                "referencing_modules": set(),
                "references_out": [],
                "ambiguous": False,
            }

    for name, mods in owners.items():
        if len(mods) > 1:
            for module in mods:
                symbols[(module, name)]["ambiguous"] = True

    # ---- reference counting -------------------------------------------
    # A symbol's own `impl` header, its declaration site, and anything
    # qualified by a *different* known type are all excluded. The last
    # rule is what keeps `KernelObjectKind::Channel` (an enum variant)
    # from being miscounted as a use of the `Channel` struct in ipc.rs.
    known_names = set(owners)
    uses_map = defaultdict(set)         # (module, name) -> symbols it references
    for parsed in parsed_files:
        here = parsed["module"]
        stripped = parsed["_stripped"]
        skip_offsets = {d["offset"] for d in parsed["decls"]}
        impls = parsed["_impls"]
        tests = parsed["_tests"]
        spans = [(d, a, b) for d, a, b in decl_spans(parsed)]
        for match in IDENT_RE.finditer(stripped):
            name = match.group(0)
            if name in RUST_KEYWORDS or name not in owners:
                continue
            offset = match.start()
            if offset in skip_offsets or rustlex.in_spans(offset, impls):
                continue
            qualifier = QUALIFIED_RE.search(stripped, max(0, offset - 64), offset)
            if qualifier and qualifier.group(1) in known_names and qualifier.group(1) != name:
                continue
            is_test = rustlex.in_spans(offset, tests)
            if not is_test:
                for decl, start, end in spans:
                    if start <= offset < end and decl["name"] != name:
                        uses_map[(here, decl["name"])].add(name)
                        break
            for module in owners[name]:
                record = symbols[(module, name)]
                if is_test:
                    record["refs_from_tests"] += 1
                elif module == here:
                    record["refs_same_module"] += 1
                else:
                    record["refs_cross_module"] += 1
                    record["referencing_modules"].add(here)

    for key, referenced in uses_map.items():
        if key in symbols:
            symbols[key]["references_out"] = sorted(referenced)
    return finish(parsed_files, crates, known, outside, symbols, crate_idents)


def finish(parsed_files, crates, known, outside, symbols, crate_idents) -> dict:
    # ---- module dependency edges --------------------------------------
    dep = defaultdict(lambda: defaultdict(int))   # src -> dst -> weight
    child = defaultdict(set)
    for parsed in parsed_files:
        here = parsed["module"]
        for name in parsed["child_mods"]:
            candidate = f"{here}::{name}"
            if candidate in known:
                child[here].add(candidate)
        for use in parsed["uses"]:
            match = PATH_RE.search(use["spec"])
            if not match:
                continue
            target = resolve_module(
                match.group(1),
                [s.strip() for s in match.group(2).split("::") if s.strip()],
                here,
                known,
                crate_idents,
            )
            if target and target != here:
                dep[here][target] += 2      # an explicit import is a stronger signal
        for ref in parsed["paths"]:
            target = resolve_module(ref["root"], ref["segments"], here, known, crate_idents)
            if target and target != here:
                dep[here][target] += 1

    # ---- cycles (Tarjan) ----------------------------------------------
    cycles = tarjan({m: set(dep[m]) for m in known})

    # ---- per-module rollup --------------------------------------------
    fan_out = {m: len(dep[m]) for m in known}
    fan_in = defaultdict(int)
    for src in dep:
        for dst in dep[src]:
            fan_in[dst] += 1

    modules = []
    for parsed in parsed_files:
        here = parsed["module"]
        modules.append({
            "id": here,
            "crate": parsed["crate"],
            "file": parsed["path"],
            "loc": parsed["loc"],
            "test_loc": parsed["test_loc"],
            "symbols": len(parsed["decls"]),
            "fan_in": fan_in.get(here, 0),
            "fan_out": fan_out.get(here, 0),
            "children": sorted(child[here]),
            "components": cohesion(parsed),
            "in_workspace": crates.get(parsed["crate"], {}).get("in_workspace", False),
        })
    return assemble(modules, dep, cycles, symbols, crates, outside)


def tarjan(graph: dict) -> list[list[str]]:
    """Strongly connected components of size > 1, i.e. real import
    cycles. Parent/child `mod` edges are excluded by the caller: a child
    module reaching back through `super::` is ordinary Rust, not a
    dependency cycle, and counting it would bury genuine cycles in
    noise."""
    index = {}
    low = {}
    stack = []
    on_stack = set()
    result = []
    counter = [0]

    def strong_connect(node: str) -> None:
        # Iterative to stay safe on deep module trees.
        work = [(node, iter(sorted(graph.get(node, ()))))]
        index[node] = low[node] = counter[0]
        counter[0] += 1
        stack.append(node)
        on_stack.add(node)
        while work:
            current, children = work[-1]
            advanced = False
            for nxt in children:
                if nxt not in index:
                    index[nxt] = low[nxt] = counter[0]
                    counter[0] += 1
                    stack.append(nxt)
                    on_stack.add(nxt)
                    work.append((nxt, iter(sorted(graph.get(nxt, ())))))
                    advanced = True
                    break
                if nxt in on_stack:
                    low[current] = min(low[current], index[nxt])
            if advanced:
                continue
            work.pop()
            if work:
                low[work[-1][0]] = min(low[work[-1][0]], low[current])
            if low[current] == index[current]:
                component = []
                while True:
                    popped = stack.pop()
                    on_stack.discard(popped)
                    component.append(popped)
                    if popped == current:
                        break
                if len(component) > 1:
                    result.append(sorted(component))

    for node in sorted(graph):
        if node not in index:
            strong_connect(node)
    return result


def cohesion(parsed: dict) -> int:
    """Number of disconnected clusters among a module's own top-level
    items.

    One cluster means everything in the file relates to everything else,
    directly or transitively. Several clusters means the file is really
    several unrelated files sharing a name -- which is what drove the
    `pulse/src/lib.rs` split into manifest/restart/lifecycle/health.
    Reported as a raw count rather than a normalised score so it stays
    interpretable: "this file is 4 unrelated things" is more actionable
    than "cohesion 0.31".
    """
    spans = [(d, a, b) for d, a, b in decl_spans(parsed) if not d["test_only"]]
    if not spans:
        return 0
    names = {d["name"]: i for i, (d, _, _) in enumerate(spans)}
    parent = list(range(len(spans)))

    def find(x: int) -> int:
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a: int, b: int) -> None:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[rb] = ra

    text = parsed["_stripped"]
    for i, (_, start, end) in enumerate(spans):
        for match in IDENT_RE.finditer(text, start, end):
            other = names.get(match.group(0))
            if other is not None and other != i:
                union(i, other)
    return len({find(i) for i in range(len(spans))})


def assemble(modules, dep, cycles, symbols, crates, outside) -> dict:
    symbol_list = []
    for original in symbols.values():
        record = dict(original)
        record["referencing_modules"] = sorted(original["referencing_modules"])
        record["refs_prod"] = record["refs_same_module"] + record["refs_cross_module"]
        record["refs_total"] = record["refs_prod"] + record["refs_from_tests"]
        # Cross-module references mean "other parts of the system depend
        # on this"; same-module ones mean "this file leans on it". Both
        # count, the first counts more for identifying real hubs.
        record["hub_score"] = record["refs_cross_module"] * 3 + record["refs_same_module"]
        record["out_degree"] = len(record["references_out"])
        symbol_list.append(record)

    symbol_list.sort(key=lambda s: (-s["hub_score"], s["name"]))
    connectors = sorted(symbol_list, key=lambda s: (-s["out_degree"], s["name"]))
    dead = [s for s in symbol_list if s["refs_total"] == 0 and not s["declared_in_test"]]
    test_only = [
        s for s in symbol_list
        if s["refs_prod"] == 0 and s["refs_from_tests"] > 0 and not s["declared_in_test"]
    ]

    edges = [
        {"source": src, "target": dst, "weight": weight}
        for src in sorted(dep) for dst, weight in sorted(dep[src].items())
    ]
    return {
        "summary": {
            "crates": len(crates),
            "workspace_crates": sorted(n for n, c in crates.items() if c["in_workspace"]),
            "non_workspace_crates": sorted(n for n, c in crates.items() if not c["in_workspace"]),
            "modules": len(modules),
            "symbols": len(symbol_list),
            "edges": len(edges),
            "import_cycles": len(cycles),
            "rust_loc": sum(m["loc"] for m in modules),
            "test_loc": sum(m["test_loc"] for m in modules),
            "unreferenced_symbols": len(dead),
            "test_only_symbols": len(test_only),
            "files_outside_workspace": outside,
        },
        "modules": sorted(modules, key=lambda m: m["id"]),
        "edges": edges,
        "cycles": cycles,
        "symbols": symbol_list,
        "connectors": connectors[:40],
        "unreferenced": dead,
        "test_only": test_only,
        "low_cohesion": sorted(
            [m for m in modules if m["components"] > 1], key=lambda m: -m["components"]
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="ADRIAN OS connection analysis")
    parser.add_argument("--print", action="store_true", dest="show")
    parser.add_argument("--out", default=os.path.join(OUT_DIR, "graph.json"))
    args = parser.parse_args()

    graph = build(REPO_ROOT)
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(graph, handle, indent=2, sort_keys=False)
        handle.write("\n")

    summary = graph["summary"]
    print(f"wrote {os.path.relpath(args.out, REPO_ROOT)}")
    print(
        f"  {summary['modules']} modules  {summary['symbols']} symbols  "
        f"{summary['edges']} edges  {summary['rust_loc']} LOC "
        f"({summary['test_loc']} in tests)"
    )
    print(f"  import cycles: {summary['import_cycles']}")
    print(f"  unreferenced symbols: {summary['unreferenced_symbols']}")
    print(f"  files outside the cargo workspace: {len(summary['files_outside_workspace'])}")

    if args.show:
        print("\nmost depended-on symbols (things a change would ripple from):")
        for record in graph["symbols"][:10]:
            print(
                f"  {record['hub_score']:>4}  {record['module']}::{record['name']}"
                f"  ({record['kind']}, x-mod {record['refs_cross_module']},"
                f" same {record['refs_same_module']}, tests {record['refs_from_tests']})"
            )
        print("\nbiggest connectors (things that touch the most other symbols):")
        for record in graph["connectors"][:10]:
            print(
                f"  {record['out_degree']:>4}  {record['module']}::{record['name']}"
                f"  ({record['kind']})"
            )
        if graph["low_cohesion"]:
            print("\nmodules that look like several unrelated files:")
            for module in graph["low_cohesion"]:
                print(f"  {module['components']} clusters  {module['file']}")
        if graph["unreferenced"]:
            print("\nunreferenced symbols (check before deleting):")
            for record in graph["unreferenced"]:
                print(f"  {record['file']}:{record['line']}  {record['name']} ({record['kind']})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

