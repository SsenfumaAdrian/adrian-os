"""Ground-truth checks for the connection analyser.

A code-analysis tool that is confidently wrong is worse than no tool,
and this one has no compiler to check it against. So it gets checked
against facts about the tree that were established independently, by
hand, and written down in PROGRESS.md before this analyser existed.

The most important case is the false-positive one. A previous tool
flagged `ChannelState`, `MessageHeader` and `EventObject` as isolated
dead code when each is genuinely used -- as a struct field type, an enum
variant, and a table element type respectively. Those three assertions
exist specifically so that regression cannot come back silently.

Run:  python3 tools/graph/validate.py
Exit: 0 if every check passes, 1 otherwise.
"""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import analyze  # noqa: E402

PASS = "PASS"
FAIL = "FAIL"


def main() -> int:
    graph = analyze.build(analyze.REPO_ROOT)
    by_name = {}
    for record in graph["symbols"]:
        by_name.setdefault(record["name"], record)
    summary = graph["summary"]
    results: list[tuple[str, str, str]] = []

    def check(label: str, ok: bool, detail: str) -> None:
        results.append((PASS if ok else FAIL, label, detail))

    # 1. No import cycles. PROGRESS.md records this as confirmed.
    check(
        "no import cycles",
        summary["import_cycles"] == 0,
        f"found {summary['import_cycles']}: {graph['cycles']}",
    )

    # 2. The documented structural hubs must actually rank as hubs. Note
    #    that they are hubs in two different senses, and conflating them
    #    is how a tool ends up disagreeing with reality: BootContext and
    #    Channel are depended *upon*, while dispatch_syscall depends on
    #    many things. Both are checked, each in its own sense.
    #
    #    The window on this one was 8 and is now 12. Recording why, so
    #    that widening a failing check reads as a corrected ground truth
    #    rather than a silenced test: Sprint 1's `boot_trace` module
    #    introduced `BootStage` (hub 70) and `record` (hub 35), both of
    #    which are referenced more than `BootContext` (hub 30) and both
    #    of which pushed it from rank 8 to rank 9. Nothing about
    #    BootContext's own connectivity changed -- it still has 10
    #    cross-module references. What changed is that the boot path
    #    grew two new hubs above it, which is the intended outcome of
    #    making boot observable, not a regression.
    HUB_WINDOW = 12
    top_depended = {r["name"] for r in graph["symbols"][:HUB_WINDOW]}
    check(
        "BootContext ranks among the most depended-on symbols",
        "BootContext" in top_depended,
        f"top {HUB_WINDOW} were {sorted(top_depended)}",
    )
    # A rank window alone is a weak assertion: it can be satisfied by
    # everything else getting smaller. The property the claim in
    # PROGRESS.md actually cares about is *reach* -- that the
    # firmware->kernel handoff type is depended on from several modules
    # rather than being an implementation detail of one. That is
    # rank-independent, so it is asserted separately, and the pair is a
    # stronger check than the single top-8 test it replaces.
    boot_context = by_name.get("BootContext")
    check(
        "BootContext is depended on from more than one module",
        boot_context is not None and len(boot_context["referencing_modules"]) > 1,
        "missing from the index" if boot_context is None
        else f"referencing_modules={boot_context['referencing_modules']}",
    )
    top_connectors = {r["name"] for r in graph["connectors"][:5]}
    # The dispatch entry point split in two when capability enforcement
    # landed: `dispatch_syscall` became a one-line wrapper supplying the
    # kernel's own context, and `dispatch_syscall_as` inherited the
    # actual fan-out. Either satisfies the documented claim -- what is
    # being asserted is that syscall dispatch is still the place the
    # kernel is wired together, not which of the two names holds it.
    check(
        "syscall dispatch ranks among the biggest connectors",
        bool({"dispatch_syscall", "dispatch_syscall_as"} & top_connectors),
        f"top 5 were {sorted(top_connectors)}",
    )

    # 2b. Capability enforcement is actually wired, not merely present.
    #     The whole point of the security work is that syscall.rs calls
    #     into security.rs; before that change `is_authorized` had zero
    #     production references outside its own module and PROGRESS.md
    #     said so explicitly. A structural check is the one thing this
    #     tool can honestly verify about it without a compiler.
    syscall_to_security = any(
        e["source"] == "adrian-kernel::syscall" and e["target"] == "adrian-kernel::security"
        for e in graph["edges"]
    )
    check(
        "syscall dispatch depends on the security module",
        syscall_to_security,
        "no adrian-kernel::syscall -> adrian-kernel::security edge",
    )
    authorized = by_name.get("is_authorized")
    check(
        "is_authorized has production callers outside its own module",
        authorized is not None and authorized["refs_cross_module"] > 0,
        "missing from the index" if authorized is None
        else f"refs_cross_module={authorized['refs_cross_module']} (must be > 0)",
    )

    # 2c. Boot observability is wired, not merely present. Sprint 1's
    #     claim is that init *reports* what it did and that the hosted
    #     wrapper *reads* that report -- so there must be an edge in from
    #     init and an edge out to boot-image. A trace module that only
    #     its own tests referenced would satisfy neither, and that is
    #     precisely the failure mode this catches: boot_trace's symbols
    #     now dominate the hub ranking above, so if they were internal
    #     to their own module that ranking would be measuring nothing.
    init_to_trace = any(
        e["source"] == "adrian-kernel::init" and e["target"] == "adrian-kernel::boot_trace"
        for e in graph["edges"]
    )
    check(
        "init depends on the boot_trace module",
        init_to_trace,
        "no adrian-kernel::init -> adrian-kernel::boot_trace edge",
    )
    outcome = by_name.get("InitOutcome")
    check(
        "the hosted wrapper reads init's outcome",
        outcome is not None and "adrian-boot-image" in outcome["referencing_modules"],
        "missing from the index" if outcome is None
        else f"referencing_modules={outcome['referencing_modules']}",
    )

    # 3. The regression guard. Each of these is used only as a field
    #    type, a variant, or a table element -- never called, never
    #    imported across modules. That is exactly the shape that gets
    #    misread as dead code.
    for name in ("ChannelState", "MessageHeader", "EventObject", "Channel"):
        record = by_name.get(name)
        check(
            f"{name} is not misreported as unused",
            record is not None and record["refs_prod"] > 0,
            "missing from the index" if record is None
            else f"refs_prod={record['refs_prod']} (must be > 0)",
        )
    unreferenced = {r["name"] for r in graph["unreferenced"]}
    check(
        "none of the four appear in the unreferenced list",
        not unreferenced & {"ChannelState", "MessageHeader", "EventObject", "Channel"},
        f"unreferenced list was {sorted(unreferenced)}",
    )

    # 4. Cross-crate edges resolve. rian/boot-image reaches the kernel as
    #    `adrian_kernel::...`, never as `crate::...`, so a resolver that
    #    only understood `crate::` would silently report the single most
    #    important edge in the tree -- the wrapper crossing into the
    #    kernel -- as absent.
    boot_edges = {
        (e["source"], e["target"]) for e in graph["edges"]
        if e["source"].startswith("adrian-boot-image")
    }
    check(
        "boot-image -> kernel::entry edge is found",
        ("adrian-boot-image", "adrian-kernel::entry") in boot_edges,
        f"boot-image edges were {sorted(boot_edges)}",
    )

    # 5. Workspace membership matches Cargo.toml exactly.
    check(
        "workspace members match Cargo.toml",
        summary["workspace_crates"] == [
            "adrian-boot-image", "adrian-kernel", "adrian-pulse", "adrian-vault",
        ],
        f"got {summary['workspace_crates']}",
    )

    # 5b. The bare-metal image is outside the workspace *on purpose*, and
    #     that has to be asserted rather than merely arranged: as a member
    #     it would inherit `std` from boot-image through resolver-2 feature
    #     unification, silently, and still link. So the exclusion is a
    #     correctness property, and this is the only check in the project
    #     that can see it -- no Rust build can, because a build that got
    #     this wrong succeeds.
    check(
        "the bare-metal image is not a workspace member",
        summary["non_workspace_crates"] == ["adrian-bare-metal"],
        f"non-workspace crates were {summary['non_workspace_crates']}",
    )

    # 5c. ...and it must still reach the kernel. The failure this guards
    #     against is the image quietly becoming a stub: it is the one crate
    #     with no tests of its own (`cargo test` would build it for the
    #     host, which it does not link for), so nothing else here would
    #     notice if `rian_main` stopped calling into the kernel. Three
    #     edges, because main.rs uses three kernel modules and all three
    #     are load-bearing: `boot` builds the BootContext, `debug` brings
    #     the UART up before anything is said through it, `entry` is the
    #     call that makes this an OS image rather than a boot stub.
    image_edges = {
        (e["source"], e["target"]) for e in graph["edges"]
        if e["source"].startswith("adrian-bare-metal")
    }
    for target in ("adrian-kernel::boot", "adrian-kernel::debug",
                   "adrian-kernel::entry"):
        check(
            f"bare-metal -> {target.split('::')[1]} edge is found",
            ("adrian-bare-metal", target) in image_edges,
            f"bare-metal edges were {sorted(image_edges)}",
        )

    # 6. Orphaned scaffold files cleanup validation. All 6 documented
    #    scaffold files outside the Cargo workspace have been cleaned up.
    outside = set(summary["files_outside_workspace"])
    check(
        "no orphan files sit outside workspace after cleanup",
        len(outside) == 0,
        f"found unexpected outside files: {sorted(outside)}",
    )

    width = max(len(label) for _, label, _ in results)
    for status, label, detail in results:
        line = f"  [{status}] {label.ljust(width)}"
        print(line if status == PASS else f"{line}  -- {detail}")

    failures = sum(1 for status, _, _ in results if status == FAIL)
    print(f"\n{len(results) - failures}/{len(results)} checks passed")

    if outside:
        print(f"\nNote: {len(outside)} files sit outside the cargo workspace:\n")
        for path in sorted(outside):
            print(f"  {path}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
