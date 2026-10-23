"use strict";

const tauriInvoke = window.__TAURI__?.core?.invoke ?? null;

const state = {
  deepResearch: false,
  selectedDomain: "Computer Science",
  files: [],
  activeView: "home",
  busy: false,
  lastTypingAt: 0,
  reducedMotion: window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false
};

const byId = (id) => document.getElementById(id);
const shell = byId("shell");
const conversation = byId("conversation");
const question = byId("question");
const send = byId("send");
const deepResearch = byId("deepResearch");
const runtimePill = byId("runtimePill");
const projectResult = byId("projectResult");
const knowledgeFiles = byId("knowledgeFiles");
const fileList = byId("fileList");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const PALETTE = {
  NURA_GOLD: "#e5bd6a",
  LIGHT_CYAN: "#64d7ff",
  SHADOW_VIOLET: "#9f7cff",
  SPINE_BLUE: "#5d8dff",
  QUANTUM_BLUE: "#2f7fff",
  LIGHT_AMBER: "#d7a94c",
  HOLD_SILVER: "#c9d3e6",
  WITNESS_ASH: "#8b93a6",
  FAULT_RED: "#ff5c6e"
};

const FEELING = {
  LIGHT_COORDINATION: "në pritje",
  QUANTUM_ELIMINATION: "po mendon",
  SHADOW_VERIFICATION: "po verifikon",
  NURA_SPEAKING: "flet",
  HOLD_IZ_QUESTION: "pret një sqarim",
  NEGATIVE_WITNESS: "nuk e gjen",
  TECHNICAL_FAULT: "problem teknik"
};

const TITLES = {
  home: "Home",
  domains: "Research Domains",
  knowledge: "My Light Knowledge",
  evolution: "My Evolution",
  architecture: "System Modules",
  project: "My Living Project"
};

async function call(cmd, args = {}) {
  if (tauriInvoke) return tauriInvoke("nura", { cmd, args });
  const r = await fetch(`/api/${cmd}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args)
  });
  const j = await r.json();
  if (!j.ok) throw new Error(j.error);
  return j.data;
}

function message(type, text) {
  const node = document.createElement("div");
  node.className = `message ${type}`;
  node.textContent = text;
  conversation.appendChild(node);
  conversation.scrollTop = conversation.scrollHeight;
  return node;
}

function setView(name) {
  state.activeView = name;
  document.querySelectorAll(".view").forEach((view) => view.classList.remove("active-view"));
  document.querySelectorAll("[data-view]").forEach((button) => {
    button.classList.toggle("active", button.dataset.view === name);
    if (button.classList.contains("living-project")) {
      button.classList.toggle("active-project", name === "project");
    }
  });
  byId(name).classList.add("active-view");
  byId("viewTitle").textContent = TITLES[name] ?? "Nura Legacy";
}

function applyExpressive(x) {
  if (!x) return;
  const color = PALETTE[x.color] || PALETTE.LIGHT_AMBER;
  const tempo = Math.max(x.tempo_ms || 0, 600);
  shell.style.setProperty("--emo", color);
  shell.style.setProperty("--tempo", `${tempo}ms`);
  shell.style.setProperty("--glow", (Math.min(x.glow_mass ?? 2000, 10000) / 10000).toFixed(3));
  shell.dataset.phase = x.phase;
  shell.dataset.motion = state.reducedMotion ? "NONE" : x.motion;
  shell.dataset.emphasis = x.emphasis;
  shell.classList.toggle("reduced", state.reducedMotion);
  window.nuraScene?.mood(color, x.glow_mass, x.tempo_ms);

  const feeling = FEELING[x.phase] || "në pritje";
  byId("emotionPhase").textContent = feeling;
  byId("lastPhase").textContent = feeling;
  const mass = Math.min(100, Math.round(Number(x.intensity_mass || 0) / 100));
  byId("emotionMeter").style.width = `${mass}%`;
  byId("emotionMeter").style.background = color;
  byId("orb").style.background = color;
  byId("orb").style.boxShadow = `0 0 ${20 + mass / 2}px ${color}aa`;
  byId("orbRing").style.borderColor = `${color}99`;
}

function applyEmotionalCommand(command) {
  if (!command) return;
  const color = PALETTE[command.color] || PALETTE.LIGHT_AMBER;
  byId("orb").style.background = color;
  byId("orbRing").style.borderColor = `${color}99`;
}

function stepDelay(x) {
  if (state.reducedMotion) return 0;
  return Math.min(900, Math.max(280, (x?.tempo_ms || 900) * 0.3));
}

async function playProcessing(steps) {
  for (const st of steps || []) {
    applyExpressive(st.expressive);
    await sleep(stepDelay(st.expressive));
  }
}

async function refreshStatus() {
  try {
    const status = await call("status");
    applyExpressive(status.emotion?.expressive);
    runtimePill.className = `runtime-pill ${status.ok ? "ok" : "bad"}`;
    runtimePill.querySelector("span").textContent = status.ok
      ? "Nura është gati — fjalori shqip është i ngarkuar"
      : "Fjalori nuk u ngarkua. Nura nuk mund të përgjigjet.";
    try {
      const p = await call("product.status");
      if (p.ok) runtimePill.querySelector("span").textContent += " · Light i lidhur";
    } catch (_) {}
  } catch (_) {
    runtimePill.className = "runtime-pill bad";
    runtimePill.querySelector("span").textContent = "Nura nuk u lidh me motorin e saj.";
    shell.dataset.phase = "TECHNICAL_FAULT";
  }
}

async function nuraSpeaks(resp) {
  const r = resp.realization;
  const node = document.createElement("div");
  node.className = "message nura";
  node.append(document.createTextNode("Nura › "));
  conversation.appendChild(node);

  const words = `${r.frame_open}${r.core}${r.frame_close}`.split(" ");
  const tempo = resp.emotion?.expressive?.tempo_ms || 0;
  const perWord = state.reducedMotion ? 0 : Math.min(55, Math.max(18, tempo / 60));
  for (const [i, w] of words.entries()) {
    const s = document.createElement("span");
    s.className = perWord ? "word" : "";
    s.textContent = i + 1 < words.length ? `${w} ` : w;
    node.appendChild(s);
    if (perWord) await sleep(perWord);
    conversation.scrollTop = conversation.scrollHeight;
  }

  if (r.question) {
    const q = document.createElement("div");
    q.className = "nura-question";
    q.textContent = r.question.replace(/\s*\(\d+\).*$/s, "");
    node.appendChild(q);
    const f = resp.analysis?.focus;
    const opts = document.createElement("div");
    opts.className = "options";
    const shown = new Set();
    (f?.senses || []).forEach((s) => {
      if (s.status !== "open") return;
      const label = s.label || s.gloss_sq || (s.gloss_en ? `${s.gloss_en} (në anglisht)` : null);
      if (!label || shown.has(label)) return;
      shown.add(label);
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = label;
      b.addEventListener("click", () => choose(f.lemma, s.key, label));
      opts.appendChild(b);
    });
    const link = r.question.match(/si lidhet (.+?) me (.+?)\?$/);
    const about = r.question.match(/çfarë di për (.+?)\?$/);
    const follow = link ? `si lidhet ${link[1]} me ${link[2]}` : about && about[1] !== "këtë" && !about[1].startsWith("secil") ? about[1] : null;
    if (follow && !opts.childElementCount) {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = "Po, le ta shohim";
      b.addEventListener("click", () => {
        if (state.busy) return;
        question.value = follow;
        askNura();
      });
      opts.appendChild(b);
    }
    // Fjala me disa lexime: «E ke fjalën për barin si vend, si bimë apo për bariun?» → një buton
    // për çdo lexim; klikimi e pyet Nurën për atë lexim («barin si bimë», «bariun»).
    const pick = r.question.match(/^E ke fjalën (.+)\?$/);
    if (pick && !opts.childElementCount) {
      let acc = "";
      pick[1].split(/, | apo /).forEach((o) => {
        const full = o.match(/^për (\S+)(?: si (.+))?$/);
        const bare = o.match(/^si (.+)$/);
        if (full) acc = full[1];
        const sense = full ? full[2] : bare ? bare[1] : null;
        if (!acc) return;
        const send = sense ? `${acc} si ${sense}` : acc;
        const b = document.createElement("button");
        b.type = "button";
        b.textContent = sense ? `si ${sense}` : acc;
        b.addEventListener("click", () => {
          if (state.busy) return;
          question.value = send;
          askNura();
        });
        opts.appendChild(b);
      });
    }
    node.appendChild(opts);
  }

  if (state.deepResearch && r.sources) {
    const src = document.createElement("div");
    src.className = "nura-sources";
    src.textContent = r.sources;
    node.appendChild(src);
  }
  if (state.deepResearch && r.oratoria) {
    const o = r.oratoria;
    const orat = document.createElement("div");
    orat.className = "nura-sources";
    const cursus = o.cursus ? ` (${o.cursus})` : "";
    orat.textContent = `Oratoria: ritmi ${o.ritmi}‰ · kadenca ${o.kadenca}‰${cursus} · baraspesha ${o.baraspesha}‰ · rritja ${o.rritja}‰ · fryma ${o.fryma}‰ — gjithsej ${o.gjithsej}‰`;
    node.appendChild(orat);
  }
  conversation.scrollTop = conversation.scrollHeight;
}

function waiting(on) {
  if (!on) return;
  shell.dataset.phase = "SHADOW_VERIFICATION";
  shell.dataset.motion = state.reducedMotion ? "NONE" : "FOCUS";
  shell.style.setProperty("--emo", PALETTE.SHADOW_VIOLET);
  byId("emotionPhase").textContent = FEELING.SHADOW_VERIFICATION;
}

function productRequest(rest) {
  const words = rest.split(/\s+/).filter(Boolean);
  const plainWords = words.length >= 2 && words.length <= 3 && words.every((w) => /^[a-zçë]+$/i.test(w));
  return plainWords ? { words } : { text: rest };
}

async function askProduct(rest, continueCase) {
  waiting(true);
  const args = productRequest(rest);
  args.continue = continueCase;
  const r = await call("ask".replace("ask", continueCase ? "product.reply" : "product.ask"), args);
  byId("lastTrace").textContent = String(r.trace ?? "—");
  const c = r.composed;
  if (c) {
    const link = c.links.length ? ` Lidhja: ${c.links.map((l) => l.label).join(" → ")}.` : "";
    const against = c.counter ? ` Përkundër: «${c.counter}»` : "";
    message("nura", `Nura › Fjalia: «${c.text}»${link}${against}`);
  }
  (r.receipts || []).forEach((rc, i) => {
    const node = message("nura", `Nura › ${r.plain[i]}`);
    if (state.deepResearch) {
      const q = document.createElement("div");
      q.className = "nura-sources";
      q.textContent = `${rc.operation} · ${rc.status} · ${rc.elapsed_ms} ms\n${rc.question}`;
      node.appendChild(q);
    }
  });
  const last = (r.receipts || []).slice(-1)[0];
  if (last) byId("lastProjectRoute").textContent = `${last.operation} · ${last.status}`;
  await playProcessing(r.steps?.map((t) => ({ expressive: r.emotion?.expressive })) || []);
  applyExpressive(r.emotion?.expressive);
}

async function askNura() {
  const text = question.value.trim();
  if (!text || state.busy) return;
  question.value = "";
  state.busy = true;
  send.disabled = true;
  message("user", `You › ${text}`);
  const probe = text.match(/^(provo|dërgo|vazhdo)\s*:?\s+(.+)$/i);
  if (probe) {
    try {
      await askProduct(probe[2].trim(), /^vazhdo/i.test(probe[1]));
    } catch (error) {
      shell.dataset.phase = "TECHNICAL_FAULT";
      message("error", `Nura › Sistemi nuk u përgjigj: ${error.message || error}`);
    } finally {
      state.busy = false;
      send.disabled = false;
      question.focus();
    }
    return;
  }
  try {
    const response = await call("ask", { text });
    byId("lastTrace").textContent = String(response.trace ?? "—");
    if (response.unsupported) {
      applyExpressive(response.emotion?.expressive);
      message("nura", "Nura › Këtë nuk e kuptova. Më shkruaj një fjalë, ose pyet p.sh. «a është qeni kafshë?», «ç'është zjarr», «e kundërta e mirë».");
    } else {
      await playProcessing(response.steps);
      applyExpressive(response.emotion?.expressive);
      applyEmotionalCommand(response.emotional_command);
      await nuraSpeaks(response);
    }
  } catch (_) {
    shell.dataset.phase = "TECHNICAL_FAULT";
    message("error", "Nura › Pata një problem teknik dhe nuk u përgjigja dot. Provo përsëri.");
  } finally {
    state.busy = false;
    send.disabled = false;
    question.focus();
  }
}

async function choose(lemma, sense, gloss) {
  if (state.busy) return;
  state.busy = true;
  message("user", `You › ${gloss}`);
  try {
    const response = await call("choose", { lemma, sense });
    byId("lastTrace").textContent = String(response.trace ?? "—");
    await playProcessing(response.steps);
    applyExpressive(response.emotion?.expressive);
    applyEmotionalCommand(response.emotional_command);
    await nuraSpeaks(response);
  } catch (_) {
    message("error", "Nura › Pata një problem teknik. Provo përsëri.");
  } finally {
    state.busy = false;
  }
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MiB`;
}

function renderFiles() {
  fileList.textContent = "";
  state.files.forEach((file) => {
    const row = document.createElement("div");
    row.className = "file-chip";
    const name = document.createElement("span");
    name.textContent = file.name;
    const size = document.createElement("span");
    size.textContent = formatBytes(file.size);
    row.append(name, size);
    fileList.appendChild(row);
  });
}

function setFiles(fileCollection) {
  const files = Array.from(fileCollection || []);
  const total = files.reduce((sum, file) => sum + file.size, 0);
  if (files.length > 8) {
    message("error", "Project intake allows at most 8 files.");
    return;
  }
  if (files.some((file) => file.size > 8 * 1024 * 1024)) {
    message("error", "Each project file must be 8 MiB or smaller.");
    return;
  }
  if (total > 24 * 1024 * 1024) {
    message("error", "Project files together must be 24 MiB or smaller.");
    return;
  }
  state.files = files;
  renderFiles();
}

async function submitProject(event) {
  event.preventDefault();
  if (state.busy) return;
  const content = byId("projectContent").value.trim();
  const route = document.querySelector("input[name=route]:checked")?.value ?? "workspace";
  if (!content) {
    projectResult.textContent = "Shkruaj materialin e projektit: ai dërgohet te Light si një rresht i vetëm (deri në 512 bajte).";
    return;
  }
  state.busy = true;
  byId("submitProject").disabled = true;
  projectResult.textContent = "Po dërgohet te Light…";
  waiting(true);
  try {
    const r = await call("product.ask", { text: content, auto_reply: false });
    const rc = r.receipts?.[0];
    byId("lastProjectRoute").textContent = rc ? `${route} · ${rc.operation} · ${rc.status}` : route;
    projectResult.textContent = [
      `Rruga: ${route}`,
      `Operacioni: ${rc?.operation ?? "—"}`,
      `Gjendja: ${rc?.status ?? "—"} (${rc?.elapsed_ms ?? 0} ms)`,
      "",
      r.plain?.[0] ?? "",
      "",
      rc?.question ?? ""
    ].join("\n");
    applyExpressive(r.emotion?.expressive);
  } catch (error) {
    shell.dataset.phase = "TECHNICAL_FAULT";
    projectResult.textContent = `Light nuk u përgjigj: ${error.message || error}`;
  } finally {
    state.busy = false;
    byId("submitProject").disabled = false;
  }
}

document.querySelectorAll("[data-view]").forEach((button) => {
  button.addEventListener("click", () => setView(button.dataset.view));
});

document.querySelectorAll("#domainGrid button").forEach((button) => {
  button.addEventListener("click", () => {
    state.selectedDomain = button.dataset.domain;
    byId("selectedDomain").textContent = state.selectedDomain;
    byId("projectDomain").value = state.selectedDomain;
    document.querySelectorAll("#domainGrid button").forEach((item) => item.classList.remove("selected"));
    button.classList.add("selected");
  });
});

send.addEventListener("click", askNura);
question.addEventListener("keydown", (event) => {
  if (event.key === "Enter") askNura();
});
question.addEventListener("input", async () => {
  const now = Date.now();
  if (now - state.lastTypingAt < 700) return;
  state.lastTypingAt = now;
  try {
    const r = await call("typing");
    if (r.transition && !state.busy) applyExpressive(r.emotion?.expressive);
  } catch (_) {}
});
deepResearch.addEventListener("click", () => {
  state.deepResearch = !state.deepResearch;
  deepResearch.classList.toggle("active", state.deepResearch);
  deepResearch.setAttribute("aria-pressed", String(state.deepResearch));
});
byId("attachButton").addEventListener("click", () => {
  setView("knowledge");
  knowledgeFiles.click();
});
knowledgeFiles.addEventListener("change", () => setFiles(knowledgeFiles.files));
byId("projectForm").addEventListener("submit", submitProject);

setInterval(async () => {
  if (state.busy) return;
  try {
    const r = await call("tick", { ms: 1000 });
    if (r.transition) applyExpressive(r.emotion?.expressive);
  } catch (_) {}
}, 1000);

(async () => {
  if (state.reducedMotion) {
    try { await call("motion", { reduced: true }); } catch (_) {}
  }
  message("system", "Nura Legacy është gati. Shkruaj një fjalë shqipe ose një pyetje të shkurtër.");
  setView("home");
  await refreshStatus();
  question.focus();
})();
