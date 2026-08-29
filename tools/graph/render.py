"""Render the connection graph as one self-contained HTML file.

No CDN, no framework, no build step -- the force layout and interaction
are a few dozen lines of plain JavaScript, and the graph data is inlined
into the document. That is a deliberate constraint, not minimalism for
its own sake: this has to open from `file://` on a machine with no
network, which is the situation the analyser itself was written for.

Run:  python3 tools/graph/render.py
Out:  graph-out/graph.html
"""

from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import analyze  # noqa: E402

TEMPLATE_HEAD = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ADRIAN OS &mdash; connection graph</title>
<style>
:root {
  color-scheme: dark;
  --bg: #0e1116; --panel: #161b22; --line: #262d36;
  --ink: #e6edf3; --dim: #8b949e; --accent: #58a6ff;
  --warn: #d29922; --bad: #f85149; --good: #3fb950;
}
* { box-sizing: border-box; }
body {
  margin: 0; background: var(--bg); color: var(--ink);
  font: 14px/1.55 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
header { padding: 22px 28px 16px; border-bottom: 1px solid var(--line); }
h1 { margin: 0 0 4px; font-size: 17px; letter-spacing: .04em; font-weight: 600; }
.sub { color: var(--dim); font-size: 12px; }
.stats { display: flex; flex-wrap: wrap; gap: 26px; margin-top: 16px; }
.stat b { display: block; font-size: 21px; font-weight: 600; }
.stat span { color: var(--dim); font-size: 11px; text-transform: uppercase; letter-spacing: .08em; }
.stat.bad b { color: var(--bad); } .stat.warn b { color: var(--warn); }
.stat.good b { color: var(--good); }
main { display: grid; grid-template-columns: minmax(0,1.35fr) minmax(340px,1fr); }
@media (max-width: 1000px) { main { grid-template-columns: 1fr; } }
"""

TEMPLATE_STYLE2 = """
#canvas { border-right: 1px solid var(--line); position: relative; min-height: 560px; }
svg { display: block; width: 100%; height: 100%; cursor: grab; }
.edge { stroke: #30363d; stroke-linecap: round; }
.edge.hot { stroke: var(--accent); }
.node circle { stroke: #0e1116; stroke-width: 1.5; cursor: pointer; }
.node text { font-size: 9.5px; fill: var(--dim); pointer-events: none; }
.node.sel circle { stroke: var(--ink); stroke-width: 2.5; }
.node.sel text, .node.hot text { fill: var(--ink); }
aside { padding: 20px 24px 40px; overflow-y: auto; max-height: 100vh; }
section { margin-bottom: 26px; }
h2 { font-size: 11px; text-transform: uppercase; letter-spacing: .09em;
     color: var(--dim); margin: 0 0 8px; font-weight: 600; }
table { width: 100%; border-collapse: collapse; font-size: 12px; }
td, th { text-align: left; padding: 3px 8px 3px 0; vertical-align: top; }
th { color: var(--dim); font-weight: 500; font-size: 10.5px; text-transform: uppercase; }
tr + tr td { border-top: 1px solid #1c222b; }
.num { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
.mod { color: var(--dim); }
.pill { display: inline-block; padding: 0 6px; border-radius: 8px; font-size: 10px;
        background: #21262d; color: var(--dim); }
.empty { color: var(--good); font-size: 12px; }
.note { color: var(--dim); font-size: 11.5px; margin: 6px 0 0; }
#detail { background: var(--panel); border: 1px solid var(--line);
          border-radius: 8px; padding: 14px 16px; min-height: 90px; }
#detail h3 { margin: 0 0 6px; font-size: 13px; }
#legend { position: absolute; left: 14px; bottom: 12px; font-size: 10.5px; color: var(--dim); }
#legend i { display: inline-block; width: 9px; height: 9px; border-radius: 50%;
            margin: 0 5px 0 12px; vertical-align: -1px; }
#legend i:first-of-type { margin-left: 0; }
</style>
</head>
"""

TEMPLATE_BODY = """<body>
<header>
  <h1>ADRIAN OS &mdash; connection graph</h1>
  <div class="sub">Generated offline by <code>tools/graph</code> &mdash;
    <span id="stamp"></span>. Node size = how many modules depend on it.</div>
  <div class="stats" id="stats"></div>
</header>
<main>
  <div id="canvas">
    <svg id="svg"></svg>
    <div id="legend"></div>
  </div>
  <aside>
    <section>
      <h2>Selected</h2>
      <div id="detail">Click any node to inspect the module.</div>
    </section>
    <section>
      <h2>Most depended-on symbols</h2>
      <table id="hubs"></table>
      <p class="note">A change here ripples outward. Counts are
        cross-module / same-module / test.</p>
    </section>
    <section>
      <h2>Biggest connectors</h2>
      <table id="conn"></table>
      <p class="note">These reach into the most other symbols &mdash; the
        places where the system is wired together.</p>
    </section>
    <section>
      <h2>Import cycles</h2>
      <div id="cycles"></div>
    </section>
    <section>
      <h2>Files that look like several files</h2>
      <table id="cohesion"></table>
      <p class="note">Disconnected clusters among a file's own top-level
        items. More than one is a candidate for splitting.</p>
    </section>
    <section>
      <h2>Unreferenced symbols</h2>
      <table id="dead"></table>
      <p class="note">Nothing in the tree names these. Entry points
        (<code>main</code>) are expected here; the rest are worth a look.</p>
    </section>
    <section>
      <h2>Outside the cargo workspace</h2>
      <table id="outside"></table>
      <p class="note">Not compiled and not imported by anything that is.</p>
    </section>
  </aside>
</main>
<script>
const DATA = __DATA__;
</script>
"""

TEMPLATE_SCRIPT = """<script>
const S = DATA.summary;
const CRATE_COLOR = {};
const PALETTE = ["#58a6ff", "#3fb950", "#d29922", "#bc8cff", "#f778ba", "#39c5cf"];
[...new Set(DATA.modules.map(m => m.crate))].sort().forEach((c, i) => {
  CRATE_COLOR[c] = PALETTE[i % PALETTE.length];
});

document.getElementById("stamp").textContent = DATA.generated_at;
document.getElementById("legend").innerHTML =
  Object.entries(CRATE_COLOR)
    .map(([c, col]) => `<i style="background:${col}"></i>${c}`).join("");

const cycleCount = S.import_cycles;
document.getElementById("stats").innerHTML = [
  ["modules", S.modules, ""],
  ["symbols", S.symbols, ""],
  ["edges", S.edges, ""],
  ["rust loc", S.rust_loc, ""],
  ["test loc", S.test_loc, ""],
  ["import cycles", cycleCount, cycleCount ? "bad" : "good"],
  ["unreferenced", S.unreferenced_symbols, S.unreferenced_symbols ? "warn" : "good"],
  ["outside workspace", S.files_outside_workspace.length,
   S.files_outside_workspace.length ? "warn" : "good"],
].map(([label, value, cls]) =>
  `<div class="stat ${cls}"><b>${value}</b><span>${label}</span></div>`).join("");

// ---- force layout -------------------------------------------------------
// Hand-rolled so the page has zero dependencies. 34 nodes does not need a
// quadtree; O(n^2) repulsion is imperceptible at this size.
const svg = document.getElementById("svg");
const box = document.getElementById("canvas");
let W = box.clientWidth, H = Math.max(560, box.clientHeight);
const index = new Map(DATA.modules.map((m, i) => [m.id, i]));
const nodes = DATA.modules.map((m, i) => ({
  ...m,
  x: W / 2 + Math.cos(i * 2.399) * (60 + i * 7),
  y: H / 2 + Math.sin(i * 2.399) * (60 + i * 7),
  vx: 0, vy: 0,
  r: 4.5 + Math.sqrt(m.fan_in) * 3.2 + Math.sqrt(m.symbols || 0) * 0.9,
}));
const links = DATA.edges
  .filter(e => index.has(e.source) && index.has(e.target))
  .map(e => ({ s: index.get(e.source), t: index.get(e.target), w: e.weight }));

const neighbours = nodes.map(() => new Set());
links.forEach(l => { neighbours[l.s].add(l.t); neighbours[l.t].add(l.s); });

function settle(rounds) {
  for (let step = 0; step < rounds; step++) {
    const cool = 1 - step / rounds;
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        let dx = nodes[j].x - nodes[i].x, dy = nodes[j].y - nodes[i].y;
        let d2 = dx * dx + dy * dy || 0.01;
        const push = 2600 / d2;
        const d = Math.sqrt(d2);
        const ux = dx / d * push, uy = dy / d * push;
        nodes[i].vx -= ux; nodes[i].vy -= uy;
        nodes[j].vx += ux; nodes[j].vy += uy;
      }
    }
    links.forEach(l => {
      const a = nodes[l.s], b = nodes[l.t];
      const dx = b.x - a.x, dy = b.y - a.y;
      const d = Math.hypot(dx, dy) || 0.01;
      const pull = (d - 110) * 0.012 * Math.min(3, l.w);
      const ux = dx / d * pull, uy = dy / d * pull;
      a.vx += ux; a.vy += uy; b.vx -= ux; b.vy -= uy;
    });
    nodes.forEach(n => {
      n.vx += (W / 2 - n.x) * 0.004;
      n.vy += (H / 2 - n.y) * 0.004;
      n.x += n.vx * cool * 0.5; n.y += n.vy * cool * 0.5;
      n.vx *= 0.72; n.vy *= 0.72;
      n.x = Math.max(n.r + 26, Math.min(W - n.r - 26, n.x));
      n.y = Math.max(n.r + 16, Math.min(H - n.r - 22, n.y));
    });
  }
}
settle(420);

// ---- draw ---------------------------------------------------------------
const NS = "http://www.w3.org/2000/svg";
let selected = null;

function short(id) {
  const parts = id.split("::");
  return parts.length > 1 ? parts.slice(1).join("::") : parts[0];
}

function draw() {
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.innerHTML = "";
  const edgeLayer = document.createElementNS(NS, "g");
  const nodeLayer = document.createElementNS(NS, "g");
  svg.append(edgeLayer, nodeLayer);

  links.forEach(l => {
    const line = document.createElementNS(NS, "line");
    line.setAttribute("x1", nodes[l.s].x); line.setAttribute("y1", nodes[l.s].y);
    line.setAttribute("x2", nodes[l.t].x); line.setAttribute("y2", nodes[l.t].y);
    line.setAttribute("stroke-width", Math.min(3, 0.6 + l.w * 0.22));
    const hot = selected !== null && (l.s === selected || l.t === selected);
    line.setAttribute("class", "edge" + (hot ? " hot" : ""));
    line.setAttribute("opacity", selected === null ? 0.75 : (hot ? 0.95 : 0.16));
    edgeLayer.appendChild(line);
  });

  nodes.forEach((n, i) => {
    const g = document.createElementNS(NS, "g");
    const near = selected !== null && (i === selected || neighbours[selected].has(i));
    g.setAttribute("class", "node" + (i === selected ? " sel" : near ? " hot" : ""));
    g.setAttribute("opacity", selected === null || near ? 1 : 0.28);
    const c = document.createElementNS(NS, "circle");
    c.setAttribute("cx", n.x); c.setAttribute("cy", n.y); c.setAttribute("r", n.r);
    c.setAttribute("fill", n.in_workspace ? CRATE_COLOR[n.crate] : "#6e7681");
    const t = document.createElementNS(NS, "text");
    t.setAttribute("x", n.x); t.setAttribute("y", n.y + n.r + 10);
    t.setAttribute("text-anchor", "middle");
    t.textContent = short(n.id);
    g.append(c, t);
    g.addEventListener("click", () => { selected = selected === i ? null : i; draw(); detail(); });
    nodeLayer.appendChild(g);
  });
}

function detail() {
  const target = document.getElementById("detail");
  if (selected === null) {
    target.innerHTML = "Click any node to inspect the module.";
    return;
  }
  const n = nodes[selected];
  const dependsOn = links.filter(l => l.s === selected).map(l => short(nodes[l.t].id));
  const dependedBy = links.filter(l => l.t === selected).map(l => short(nodes[l.s].id));
  const owned = DATA.symbols.filter(s => s.module === n.id)
    .sort((a, b) => b.hub_score - a.hub_score).slice(0, 8);
  target.innerHTML = `
    <h3>${n.id}</h3>
    <div class="mod">${n.file} &middot; ${n.loc} lines
      (${n.test_loc} in tests) &middot; ${n.symbols} top-level items
      &middot; ${n.components} cluster${n.components === 1 ? "" : "s"}
      ${n.in_workspace ? "" : '&middot; <span class="pill">outside workspace</span>'}</div>
    <p class="note"><b>Depends on</b> (${dependsOn.length}):
      ${dependsOn.join(", ") || "&mdash;"}</p>
    <p class="note"><b>Depended on by</b> (${dependedBy.length}):
      ${dependedBy.join(", ") || "&mdash;"}</p>
    <p class="note"><b>Busiest items:</b>
      ${owned.map(s => `${s.name} <span class="mod">(${s.refs_prod})</span>`).join(", ")
        || "&mdash;"}</p>`;
}

function table(id, rows, head) {
  const el = document.getElementById(id);
  if (!rows.length) { el.outerHTML = `<div class="empty">None.</div>`; return; }
  el.innerHTML = "<tr>" + head.map(h => `<th>${h}</th>`).join("") + "</tr>"
    + rows.map(r => "<tr>" + r.join("") + "</tr>").join("");
}

table("hubs", DATA.symbols.slice(0, 12).map(s => [
  `<td>${s.name}<div class="mod">${short(s.module)}</div></td>`,
  `<td><span class="pill">${s.kind}</span></td>`,
  `<td class="num">${s.refs_cross_module} / ${s.refs_same_module} / ${s.refs_from_tests}</td>`,
]), ["symbol", "kind", "x / same / test"]);

table("conn", DATA.connectors.filter(s => s.out_degree > 0).slice(0, 12).map(s => [
  `<td>${s.name}<div class="mod">${short(s.module)}</div></td>`,
  `<td><span class="pill">${s.kind}</span></td>`,
  `<td class="num">${s.out_degree}</td>`,
]), ["symbol", "kind", "reaches"]);

document.getElementById("cycles").innerHTML = DATA.cycles.length
  ? DATA.cycles.map(c => `<div class="note">${c.join(" &rarr; ")}</div>`).join("")
  : '<div class="empty">None. Every module dependency is acyclic.</div>';

table("cohesion", DATA.low_cohesion.map(m => [
  `<td>${m.file}</td>`,
  `<td class="num">${m.components}</td>`,
  `<td class="num">${m.loc}</td>`,
]), ["file", "clusters", "lines"]);

table("dead", DATA.unreferenced.map(s => [
  `<td>${s.name}<div class="mod">${s.file}:${s.line}</div></td>`,
  `<td><span class="pill">${s.kind}</span></td>`,
]), ["symbol", "kind"]);

table("outside", S.files_outside_workspace.map(f => [`<td>${f}</td>`]), ["file"]);

// Re-settle on resize so the layout uses the space it actually has.
let resizeTimer;
addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    W = box.clientWidth; H = Math.max(560, box.clientHeight);
    settle(120); draw();
  }, 180);
});

draw();
detail();
</script>
</body>
</html>
"""


def render(graph: dict) -> str:
    payload = {k: v for k, v in graph.items() if not k.startswith("_")}
    data = json.dumps(payload, separators=(",", ":")).replace("</", "<\\/")
    return (
        TEMPLATE_HEAD
        + TEMPLATE_STYLE2
        + TEMPLATE_BODY.replace("__DATA__", data)
        + TEMPLATE_SCRIPT
    )


def main() -> int:
    import datetime

    graph = analyze.build(analyze.REPO_ROOT)
    graph["generated_at"] = datetime.datetime.now().strftime("%Y-%m-%d %H:%M")
    os.makedirs(analyze.OUT_DIR, exist_ok=True)

    json_path = os.path.join(analyze.OUT_DIR, "graph.json")
    with open(json_path, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(graph, handle, indent=2)
        handle.write("\n")

    html_path = os.path.join(analyze.OUT_DIR, "graph.html")
    with open(html_path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(render(graph))

    for path in (json_path, html_path):
        size = os.path.getsize(path) / 1024
        print(f"wrote {os.path.relpath(path, analyze.REPO_ROOT)}  ({size:.0f} KB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
