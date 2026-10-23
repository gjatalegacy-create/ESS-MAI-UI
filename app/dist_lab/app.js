"use strict";
// ui_ess_mai — shtresa e paraqitjes. Nuk llogarit kuptim, emocion apo pozicione:
// të gjitha vijnë nga Rust (nura_lingua) përmes komandës së vetme `nura`.
// Transporti: Tauri 2 (window.__TAURI__) ose shërbimi lokal /api/<komanda>.

const tauriInvoke = window.__TAURI__?.core?.invoke ?? null;
const $ = (id) => document.getElementById(id);
const SVGNS = "http://www.w3.org/2000/svg";
const shell = $("shell");
const state = { reduced: false, playing: false, lastTypingAt: 0, view: null, finalStatus: new Map() };

const PALETTE = {
  NURA_GOLD: "#e5bd6a", LIGHT_CYAN: "#64d7ff", SHADOW_VIOLET: "#9f7cff", SPINE_BLUE: "#5d8dff",
  QUANTUM_BLUE: "#2f7fff", LIGHT_AMBER: "#d7a94c",
  // të reja në ui_ess_mai
  HOLD_SILVER: "#c9d3e6", WITNESS_ASH: "#8b93a6", FAULT_RED: "#ff5c6e"
};

async function call(cmd, args = {}) {
  if (tauriInvoke) return tauriInvoke("nura", { cmd, args });
  const r = await fetch(`/api/${cmd}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(args) });
  const j = await r.json();
  if (!j.ok) throw new Error(j.error);
  return j.data;
}

const sleep = (ms) => new Promise((res) => setTimeout(res, ms));
const el = (tag, cls, text) => { const n = document.createElement(tag); if (cls) n.className = cls; if (text !== undefined) n.textContent = text; return n; };
const svg = (tag, attrs) => { const n = document.createElementNS(SVGNS, tag); for (const [k, v] of Object.entries(attrs || {})) n.setAttribute(k, v); return n; };

// ───────────── emocioni ─────────────
function applyExpressive(x) {
  if (!x) return;
  const color = PALETTE[x.color] || PALETTE.LIGHT_AMBER;
  shell.style.setProperty("--emo", color);
  shell.style.setProperty("--tempo", `${Math.max(x.tempo_ms, 1)}ms`);
  shell.style.setProperty("--glow", (x.glow_mass / 10000).toFixed(3));
  shell.dataset.phase = x.phase;
  shell.dataset.emphasis = x.emphasis;
  shell.dataset.motion = x.motion;
  shell.classList.toggle("reduced", !!x.reduced_motion);
  $("emotionPhase").textContent = x.phase + (x.inherited ? "" : " ·i ri");
  $("emotionLabel").textContent = x.phase_label;
  $("emotionMeter").style.width = `${Math.round(x.intensity_mass / 100)}%`;
  $("fRegister").textContent = x.register;
  $("fTempo").textContent = x.tempo_ms ? `${x.tempo_ms} ms` : "pa lëvizje";
  $("fEmphasis").textContent = x.emphasis;
  $("fTrust").textContent = `${(x.trust_mass / 100).toFixed(0)}%`;
  $("fMotion").textContent = `${x.motion} · ${x.animation}`;
  state.spread = x.spread_mass;
}

function renderLog(log) {
  const ol = $("transitionLog");
  ol.textContent = "";
  (log || []).slice(0, 30).forEach((t) => {
    const li = el("li");
    li.append(el("b", "", `#${t.seq} ${t.from} → ${t.to}`));
    li.append(el("span", "", `shkak: ${t.cause}`));
    li.append(el("span", "", t.rule));
    ol.append(li);
  });
}

function applyEmotion(emotion) {
  if (!emotion) return;
  applyExpressive(emotion.expressive);
  renderLog(emotion.log);
}

// ───────────── grafi ─────────────
const short = (s, n = 26) => (s.length > n ? s.slice(0, n - 1) + "…" : s);
function relClass(rel) {
  if (rel === "has_sense") return "rel-has_sense";
  if (rel.startsWith("wn:")) return "rel-wn";
  if (rel.startsWith("cn:")) return "rel-cn";
  if (rel.startsWith("dep:")) return "rel-dep";
  return `rel-${rel}`;
}

// Pamja përshtatet me formën e skenës: gjerësia logjike 2300, lartësia sipas raportit.
function fitViewBox() {
  const st = $("stage").getBoundingClientRect();
  const w = 2300, h = Math.max(900, Math.round(w * (st.height / Math.max(st.width, 1))));
  $("graph").setAttribute("viewBox", `-1150 ${-h / 2} ${w} ${h}`);
  ["halfX", "halfY"].forEach((id) => { $(id).setAttribute("y", -h / 2); $(id).setAttribute("height", h); });
  $("splitLine").setAttribute("y1", -h / 2); $("splitLine").setAttribute("y2", h / 2);
  state.viewH = h;
}

// Shpërndarja vertikale: hapësira e lirë × masa e shpërndarjes nga motori emocional.
function yScale() {
  const room = Math.max(0.3, ((state.viewH ?? 1960) / 2 - 150) / 820);
  return room * (0.6 + ((state.spread ?? 5000) / 10000) * 0.4);
}

function renderView(view, revealAll) {
  state.view = view;
  fitViewBox();
  $("emptyHint")?.remove();
  const gE = $("edges"), gN = $("nodes");
  gE.textContent = ""; gN.textContent = "";
  const k = yScale();
  const pos = new Map(view.nodes.map((n) => [n.id, { x: n.x, y: n.y * k }]));
  view.edges.forEach((e) => {
    const a = pos.get(e.from), b = pos.get(e.to);
    if (!a || !b) return;
    const mx = (a.x + b.x) / 2, my = (a.y + b.y) / 2 - 40;
    const p = svg("path", { d: `M${a.x} ${a.y} Q${mx} ${my} ${b.x} ${b.y}`, class: `edge ${relClass(e.rel)}${e.on_path ? " on-path" : ""}`, "data-edge": e.id, "data-from": e.from, "data-to": e.to });
    p.append(svg("title"));
    p.firstChild.textContent = `${e.rel_label} · ${e.source_label} · gjuha ${e.lang} · metoda ${e.method}`;
    const hit = svg("path", { d: p.getAttribute("d"), class: "edge-hit", "data-hit": e.id });
    hit.addEventListener("click", () => inspectEdge(e));
    p.addEventListener("click", () => inspectEdge(e));
    gE.append(p, hit);
  });
  view.nodes.forEach((n) => {
    const g = svg("g", { class: `node kind-${n.kind} side-${n.side} st-${n.status}`, transform: `translate(${n.x} ${n.y * k})`, "data-id": n.id });
    if (n.side === "C" && n.kind !== "absent") {
      g.classList.add("focus");
      g.append(svg("image", { href: "assets/i0-star.png", x: -85, y: -85, width: 170, height: 170 }));
      const t = svg("text", { x: 0, y: 125, "text-anchor": "middle" }); t.textContent = n.label; t.setAttribute("font-size", "44"); g.append(t);
      const s = svg("text", { x: 0, y: 160, "text-anchor": "middle", class: "sub" }); s.textContent = "i₀"; g.append(s);
    } else {
      if (n.kind === "sentence") g.append(svg("rect", { x: -26, y: -16, width: 52, height: 32, rx: 8, class: "dot" }));
      else if (n.kind === "concept") g.append(svg("rect", { x: -18, y: -18, width: 36, height: 36, class: "dot", transform: "rotate(45)" }));
      else g.append(svg("circle", { r: n.kind === "absent" ? 70 : n.kind === "word" ? 26 : 22, class: "dot" }));
      const left = n.x < 0;
      const t = svg("text", { x: left ? -36 : 36, y: 10, "text-anchor": left ? "end" : "start", class: n.lang !== "sq" ? "lang-en" : "" });
      t.textContent = n.kind === "absent" ? `∅ ${n.label}` : short(n.label) + (n.lang !== "sq" ? ` ·${n.lang}` : "");
      if (n.kind === "absent") { t.setAttribute("x", 0); t.setAttribute("y", 110); t.setAttribute("text-anchor", "middle"); }
      g.append(t);
      if (n.kind === "sense") {
        const s = svg("text", { x: left ? -36 : 36, y: 40, "text-anchor": left ? "end" : "start", class: "sub" });
        s.textContent = `${n.key.split(":").pop()} · ${n.status}`;
        g.append(s);
      }
    }
    const title = svg("title"); title.textContent = `${n.label} · ${n.kind} · ${n.source}${n.note ? " · " + n.note : ""}`; g.append(title);
    g.addEventListener("click", () => n.id !== 4294967295 && explore(n.key));
    if (!revealAll && n.side !== "C") g.classList.add("hidden");
    gN.append(g);
  });
}

function nodeEl(id) { return $("nodes").querySelector(`[data-id="${id}"]`); }
function setStatus(g, status) {
  [...g.classList].filter((c) => c.startsWith("st-")).forEach((c) => g.classList.remove(c));
  g.classList.add(`st-${status}`);
  const sub = g.querySelector("text.sub");
  if (sub && g.classList.contains("kind-sense")) sub.textContent = sub.textContent.replace(/· \S+$/, `· ${status}`);
}
function reveal(ids) { ids.forEach((id) => { const g = nodeEl(id); if (g && g.classList.contains("hidden")) { g.classList.remove("hidden"); g.classList.add("appear"); } }); syncEdges(); }
function revealAll() { $("nodes").querySelectorAll(".node.hidden").forEach((g) => { g.classList.remove("hidden"); g.classList.add("appear"); }); syncEdges(); }
function syncEdges() {
  $("edges").querySelectorAll(".edge").forEach((p) => {
    const a = nodeEl(p.dataset.from), b = nodeEl(p.dataset.to);
    const hide = !a || !b || a.classList.contains("hidden") || b.classList.contains("hidden");
    p.classList.toggle("hidden", hide);
    p.nextSibling?.classList.toggle("hidden", hide);
  });
}
function highlight(ids, edges) {
  $("nodes").querySelectorAll(".node.hl").forEach((g) => g.classList.remove("hl"));
  ids.forEach((id) => nodeEl(id)?.classList.add("hl"));
  (edges || []).forEach((id) => $("edges").querySelector(`[data-edge="${id}"]`)?.classList.add("on-path"));
}

// ───────────── procesi ─────────────
function stepDelay(x) {
  if (state.reduced || x.reduced_motion) return 0;
  const d = Math.min(1400, Math.max(260, x.tempo_ms * 0.35));
  // Kur grafi është i mbyllur, ritmi ndihet vetëm te orb-i: pritje më e shkurtër.
  return $("home").classList.contains("graph-open") ? d : Math.min(d, 320);
}

async function playSteps(resp) {
  state.playing = true;
  const rail = $("processRail");
  rail.textContent = "";
  const view = resp.view;
  // statuset përfundimtare ruhen; para eliminimit kuptimet shfaqen të hapura
  const final = new Map(view.nodes.map((n) => [n.id, n.status]));
  const pre = { ...view, nodes: view.nodes.map((n) => ({ ...n, status: n.side === "Y" && ["eliminated", "selected"].includes(n.status) ? "open" : n.status })) };
  renderView(pre, false);
  for (const st of resp.steps) {
    applyExpressive(st.expressive);
    const li = el("li");
    li.append(el("b", "", st.code));
    li.append(document.createTextNode(st.label));
    if (st.transition) li.append(el("small", "", `${st.transition.rule.split(" ")[0]} → ${st.transition.to}`));
    rail.append(li);
    if (st.code === "SPLIT") revealAll();
    else reveal(st.nodes);
    if (["ELIM", "VERIFY", "HOLD", "CHOICE"].includes(st.code)) {
      st.nodes.forEach((id) => { const g = nodeEl(id); if (g && final.has(id)) setStatus(g, final.get(id)); });
    }
    highlight(st.nodes, st.edges);
    await sleep(stepDelay(st.expressive));
  }
  revealAll();
  view.nodes.forEach((n) => { const g = nodeEl(n.id); if (g) setStatus(g, n.status); });
  state.playing = false;
}

// ───────────── biseda ─────────────
function message(type, text) {
  const node = el("div", `message ${type}`, text);
  $("conversation").append(node);
  $("conversation").scrollTop = $("conversation").scrollHeight;
  return node;
}

async function nuraMessage(resp) {
  const r = resp.realization;
  const node = el("div", "message nura");
  node.append(el("span", "frame", r.frame_open));
  const core = el("span", "core");
  node.append(core);
  node.append(el("span", "frame", r.frame_close));
  $("conversation").append(node);
  const words = r.core.split(" ");
  const tempo = resp.emotion.expressive.tempo_ms;
  const perWord = state.reduced || !tempo ? 0 : Math.min(60, tempo / 45);
  for (const [i, w] of words.entries()) {
    const s = el("span", perWord ? "word" : "", i + 1 < words.length ? w + " " : w);
    core.append(s);
    if (perWord) await sleep(perWord);
  }
  if (r.question) {
    node.append(el("div", "", r.question));
    const f = resp.analysis?.focus;
    if (f) {
      const opts = el("div", "options");
      f.senses.forEach((s, i) => {
        if (s.status !== "open") return;
        const b = el("button", "", `(${i + 1}) ${s.gloss_sq ? short(s.gloss_sq, 70) : s.labels_en.join(", ") + " [anglisht]"}`);
        b.addEventListener("click", () => choose(f.lemma, s.key));
        opts.append(b);
      });
      node.append(opts);
    }
  }
  if (r.sources) node.append(el("span", "sources", r.sources));
  node.append(el("span", "meta", `regjistri ${r.register} · thelbi i njëjtë në 7 regjistra: ${resp.core_invariant ? "PO ✓" : "JO ✗"} · vendimi ${resp.plan.verdict} · gjurma #${resp.trace}`));
  $("conversation").scrollTop = $("conversation").scrollHeight;
}

function renderProcessPanel(resp) {
  const list = $("registerList");
  list.textContent = "";
  const base = resp.registers[0].core;
  resp.registers.forEach((r) => {
    const d = el("div");
    d.append(el("b", "", r.register));
    d.append(el("span", "frame", r.frame_open));
    d.append(document.createTextNode(r.core));
    d.append(el("span", "frame", r.frame_close));
    d.append(el("span", r.core === base ? "ok" : "ok bad", r.core === base ? "thelbi = ✓" : "thelbi ≠ ✗"));
    list.append(d);
  });
  $("planBox").textContent = JSON.stringify(resp.plan, null, 2) + "\n\nnënshkrimi: " + resp.signature;
  const fb = $("formsBox"); fb.textContent = "";
  resp.realization.forms.forEach((f) => fb.append(el("div", "", `«${f.surface}» — ${f.origin}`)));
  $("cmdBox").textContent = JSON.stringify(resp.emotional_command, null, 2);
}

async function ask(text) {
  if (!text || state.playing) return;
  setView("home");
  $("send").disabled = true;
  message("user", `Ti › ${text}`);
  try {
    const resp = await call("ask", { text });
    if (resp.unsupported) {
      applyEmotion(resp.emotion);
      message("system", resp.message);
    } else {
      await playSteps(resp);
      applyEmotion(resp.emotion);
      await nuraMessage(resp);
      renderProcessPanel(resp);
    }
  } catch (e) {
    message("error", `Gabim teknik › ${e.message || e}`);
    refreshStatus();
  } finally {
    $("send").disabled = false;
    $("question").focus();
  }
}

async function choose(lemma, sense) {
  if (state.playing) return;
  message("user", `Ti › zgjodha kuptimin ${sense.split(":").pop()} për «${lemma}»`);
  try {
    const resp = await call("choose", { lemma, sense });
    await playSteps(resp);
    applyEmotion(resp.emotion);
    await nuraMessage(resp);
    renderProcessPanel(resp);
  } catch (e) { message("error", `Gabim › ${e.message || e}`); }
}

// ───────────── eksplorimi & inspektori ─────────────
async function explore(key) {
  if (state.playing) return;
  try {
    const r = await call("explore", { key });
    applyEmotion(r.emotion);
    renderView(r.view, true);
    $("processRail").textContent = "";
    const li = el("li"); li.append(el("b", "", "EXPLORE")); li.append(document.createTextNode(`${r.node.label} · ${r.node.kind} · ${r.node.out_degree} dalje / ${r.node.in_degree} hyrje`));
    if (r.transition) li.append(el("small", "", `${r.transition.rule.split(" ")[0]} → ${r.transition.to}`));
    $("processRail").append(li);
    inspectNode(r.node);
  } catch (e) { message("error", `Eksplorimi › ${e.message || e}`); }
}

function inspectNode(n) {
  const box = $("inspector"); box.textContent = "";
  box.append(el("h4", "", `${n.label} ${n.lang !== "sq" ? `(${n.lang})` : ""}`));
  box.append(el("div", "kv", `${n.kind} · ${n.key}`));
  const sec = (title, arr) => {
    if (!arr.length) return;
    box.append(el("div", "kv", title));
    arr.slice(0, 6).forEach((t) => { const d = el("div"); d.append(el("span", "tag", `${t.lang} · ${t.source}`)); d.append(document.createTextNode(t.text)); box.append(d); });
  };
  sec("etiketa", n.labels); sec("përkufizime", n.glosses); sec("shembuj", n.examples);
  if (n.kind === "word") { const b = el("button", "", "Pyet Nura-n për këtë fjalë"); b.addEventListener("click", () => ask(n.label)); box.append(b); }
}

function inspectEdge(e) {
  const box = $("inspector"); box.textContent = "";
  box.append(el("h4", "", e.rel_label));
  const a = state.view.nodes.find((n) => n.id === e.from), b = state.view.nodes.find((n) => n.id === e.to);
  box.append(el("div", "", `${a?.label ?? e.from} → ${b?.label ?? e.to}`));
  [["marrëdhënia", e.rel], ["burimi", e.source_label], ["gjuha", e.lang], ["metoda", e.method], ["në rrugën e verifikimit", e.on_path ? "po" : "jo"]]
    .forEach(([k, v]) => { const d = el("div"); d.append(el("span", "kv", `${k}: `)); d.append(document.createTextNode(v)); box.append(d); });
  e.attrs.forEach(([k, v]) => { const d = el("div"); d.append(el("span", "tag", k)); d.append(document.createTextNode(v)); box.append(d); });
  box.append(el("div", "kv", "Vija në ekran = ky skaj i grafit. Afërsia vizuale nuk është marrëdhënie."));
}

// ───────────── pamjet ─────────────
const TITLES = { home: "Nura · Grafi i fjalëve", process: "Shprehja & toni", law: "Ligji emocional", sources: "Burimet gjuhësore" };
function setView(name) {
  document.querySelectorAll(".view").forEach((v) => v.classList.toggle("active-view", v.id === name));
  document.querySelectorAll(".nav-item").forEach((b) => b.classList.toggle("active", b.dataset.view === name));
  $("viewTitle").textContent = TITLES[name];
}

async function refreshStatus() {
  const pill = $("runtimePill");
  try {
    const s = await call("status");
    applyEmotion(s.emotion);
    state.reduced = !!s.emotion.expressive.reduced_motion;
    $("motionToggle").classList.toggle("active", state.reduced);
    $("motionToggle").setAttribute("aria-pressed", String(state.reduced));
    const pt = $("phaseTable"); pt.textContent = "";
    s.phases.forEach((p) => { const d = el("div"); d.append(el("b", "", p.code)); d.append(el("span", "", p.label)); d.append(el("span", p.inherited ? "inh" : "new", p.inherited ? "trashëguar" : "i ri")); pt.append(d); });
    if (!s.ok) { pill.className = "runtime-pill bad"; pill.querySelector("span").textContent = s.error; return; }
    pill.className = "runtime-pill ok";
    pill.querySelector("span").textContent = `Indeksi lokal: ${s.nodes.toLocaleString()} nyje · ${s.edges.toLocaleString()} skaje · ${s.sq_words.toLocaleString()} fjalë shqipe · ${s.load_ms} ms · pa rrjet`;
    const sum = $("sourceSummary"); sum.textContent = "";
    [`${s.nodes} nyje`, `${s.edges} skaje`, `${s.sq_words} fjalë shqipe`, `${s.morph} hyrje morfologjike UD`].forEach((t) => sum.append(el("span", "", t)));
    const tb = $("sourceTable"); tb.textContent = "";
    const head = el("tr"); ["burimi", "skedari", "versioni", "licenca", "bajt", "SHA-256 lokale", "kontrolli i botuesit", "statusi"].forEach((h) => head.append(el("th", "", h))); tb.append(head);
    s.sources.forEach((r) => { const tr = el("tr"); [r.source, r.file, r.version, r.license, r.bytes, r.sha256.slice(0, 16) + "…", r.publisher_check, r.status].forEach((v) => tr.append(el("td", "", String(v)))); tb.append(tr); });
    $("reportBox").textContent = s.report.join("\n");
  } catch (e) {
    pill.className = "runtime-pill bad";
    pill.querySelector("span").textContent = `Gabim teknik: ${e.message || e}`;
  }
}

// ───────────── ngjarjet ─────────────
document.querySelectorAll(".nav-item").forEach((b) => b.addEventListener("click", () => setView(b.dataset.view)));
$("examples").querySelectorAll("button").forEach((b) => b.addEventListener("click", () => ask(b.textContent)));
$("send").addEventListener("click", () => { const t = $("question").value.trim(); $("question").value = ""; ask(t); });
$("question").addEventListener("keydown", (e) => { if (e.key === "Enter") { const t = $("question").value.trim(); $("question").value = ""; ask(t); } });
$("question").addEventListener("input", async () => {
  const now = Date.now();
  if (now - state.lastTypingAt < 700) return;
  state.lastTypingAt = now;
  try { const r = await call("typing"); if (r.transition) applyEmotion(r.emotion); } catch (_) { /* transporti raportohet te statusi */ }
});
$("motionToggle").addEventListener("click", async () => {
  state.reduced = !state.reduced;
  $("motionToggle").classList.toggle("active", state.reduced);
  $("motionToggle").setAttribute("aria-pressed", String(state.reduced));
  try { const r = await call("motion", { reduced: state.reduced }); applyEmotion(r.emotion); } catch (e) { message("error", String(e)); }
});

$("graphToggle").addEventListener("click", () => {
  const open = !$("home").classList.contains("graph-open");
  $("home").classList.toggle("graph-open", open);
  $("graphToggle").classList.toggle("active", open);
  $("graphToggle").setAttribute("aria-pressed", String(open));
  if (open && state.view) renderView(state.view, true);
});

setInterval(async () => {
  if (state.playing) return;
  try { const r = await call("tick", { ms: 1000 }); if (r.transition) applyEmotion(r.emotion); } catch (_) { /* heshtur */ }
}, 1000);

(async () => {
  await refreshStatus();
  const prefersReduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if (prefersReduced && !state.reduced) $("motionToggle").click();
  message("system", "Nura Legacy · ui_ess_mai. Të dhënat gjuhësore janë burime të importuara (WordNet, Albanet/OMW, ConceptNet, UD) — jo histori organike e ESS-MAI. Emocioni këtu është funksion i modeluar i ndërveprimit, jo ndjenjë.");
  $("question").focus();
})();
