(function () {
  "use strict";

  const canvas = document.getElementById("sky");
  const shell = document.getElementById("shell");
  if (!canvas || !shell) return;
  const ctx = canvas.getContext("2d");
  const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

  const PLATE = { w: 1536, h: 864, src: "assets/nura-legacy-scene.jpg" };
  const FIGURE = [
    [545, 262], [582, 268], [606, 290], [618, 330], [616, 372], [612, 410], [600, 440], [640, 432],
    [690, 430], [730, 460], [752, 510], [764, 580], [770, 640], [776, 720], [770, 790], [762, 864],
    [380, 864], [350, 810], [322, 740], [316, 640], [322, 580], [330, 500], [360, 455], [410, 432],
    [470, 440], [484, 410], [478, 370], [480, 325], [494, 290], [515, 270]
  ];
  const TITLE = { x: 580, y: 30, w: 380, h: 122 };
  const CORE = { x: 592, y: 150, w: 400, h: 492, cx: 770, cy: 400, d0: 150 };
  const COVERED = [[0, 0, 292, 864], [786, 566, 1536, 864]];
  const STARS = 260;
  const MAG_LIMIT = 4.6;
  const EMPHASIS = {
    MEANING_SIDE: [0.18, 0.95],
    CENTER: [0.45, 0.45],
    X_INHERITS_Y: [0.85, 0.85],
    SUSPENDED: [0.5, 0.5],
    ABSENCE_MARK: [0.05, 0.05],
    NONE: [0, 0]
  };
  const QUIET = new Set(["NEGATIVE_WITNESS", "TECHNICAL_FAULT"]);

  const rand = mulberry32(1941);
  const stars = [];
  const view = { w: 1, h: 1, dpr: 1, s: 1, ox: 0, oy: 0 };
  const mood = { tint: [215, 169, 76], glow: 0.25, tempo: 4200, warm: 0.3, cool: 0.3, amp: 0.6, period: 11 };
  const target = { warm: 0.3, cool: 0.3, amp: 0.6, period: 11 };
  let plate = null, lumen = null, figureMask = null, glow = null, spike = null;
  let energy = null, ectx = null;
  let clock = 0, phase = 0, last = 0, raf = 0;

  function mulberry32(seed) {
    let a = seed >>> 0;
    return function () {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  function smoothstep(a, b, x) {
    const u = Math.min(Math.max((x - a) / (b - a), 0), 1);
    return u * u * (3 - 2 * u);
  }

  function approach(current, goal, dt, tau) {
    return goal + (current - goal) * Math.exp(-dt / tau);
  }

  function planckianXY(kelvin) {
    const t = Math.min(Math.max(kelvin, 1667), 25000);
    const x = t < 4000
      ? -0.2661239e9 / t ** 3 - 0.2343589e6 / t ** 2 + 0.8776956e3 / t + 0.17991
      : -3.0258469e9 / t ** 3 + 2.1070379e6 / t ** 2 + 0.2226347e3 / t + 0.24039;
    const y = t < 2222
      ? -1.1063814 * x ** 3 - 1.3481102 * x ** 2 + 2.18555832 * x - 0.20219683
      : t < 4000
        ? -0.9549476 * x ** 3 - 1.37418593 * x ** 2 + 2.09137015 * x - 0.16748867
        : 3.081758 * x ** 3 - 5.8733867 * x ** 2 + 3.75112997 * x - 0.37001483;
    return [x, y];
  }

  function srgbEncode(c) {
    return c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
  }

  function blackbody(kelvin, whiten) {
    const [x, y] = planckianXY(kelvin);
    const X = x / y, Z = (1 - x - y) / y;
    const linear = [
      3.2406 * X - 1.5372 - 0.4986 * Z,
      -0.9689 * X + 1.8758 + 0.0415 * Z,
      0.0557 * X - 0.204 + 1.057 * Z
    ].map(v => Math.max(v, 0));
    const peak = Math.max(...linear) || 1;
    return linear.map(v => Math.round(255 * (whiten + (1 - whiten) * srgbEncode(v / peak))));
  }

  function inside(poly, x, y) {
    let hit = false;
    for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
      const [xi, yi] = poly[i], [xj, yj] = poly[j];
      if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) hit = !hit;
    }
    return hit;
  }

  function luminance(x, y) {
    if (!lumen) return 0;
    const i = (Math.min(PLATE.h - 1, Math.max(0, y | 0)) * PLATE.w + Math.min(PLATE.w - 1, Math.max(0, x | 0))) * 4;
    return (0.2126 * lumen[i] + 0.7152 * lumen[i + 1] + 0.0722 * lumen[i + 2]) / 255;
  }

  function openSky(x, y) {
    const covered = COVERED.some(([x0, y0, x1, y1]) => x >= x0 && x <= x1 && y >= y0 && y <= y1);
    const title = x >= TITLE.x - 12 && x <= TITLE.x + TITLE.w + 12 && y >= TITLE.y - 8 && y <= TITLE.y + TITLE.h + 8;
    const figure = [[0, 0], [16, 0], [-16, 0], [0, 16], [0, -16]].some(([dx, dy]) => inside(FIGURE, x + dx, y + dy));
    return !covered && !title && !figure && luminance(x, y) < 0.42;
  }

  function sprite(rgb) {
    const c = document.createElement("canvas");
    c.width = c.height = 32;
    const g = c.getContext("2d");
    const grad = g.createRadialGradient(16, 16, 0, 16, 16, 16);
    grad.addColorStop(0, "rgba(255,255,255,1)");
    grad.addColorStop(0.18, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},0.95)`);
    grad.addColorStop(0.45, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},0.22)`);
    grad.addColorStop(1, "rgba(0,0,0,0)");
    g.fillStyle = grad;
    g.fillRect(0, 0, 32, 32);
    return c;
  }

  function spikeSprite() {
    const c = document.createElement("canvas");
    c.width = c.height = 96;
    const g = c.getContext("2d");
    for (const [x0, y0, x1, y1] of [[0, 48, 96, 48], [48, 0, 48, 96]]) {
      const grad = g.createLinearGradient(x0, y0, x1, y1);
      grad.addColorStop(0, "rgba(255,255,255,0)");
      grad.addColorStop(0.5, "rgba(255,255,255,0.9)");
      grad.addColorStop(1, "rgba(255,255,255,0)");
      g.strokeStyle = grad;
      g.lineWidth = 1.2;
      g.beginPath();
      g.moveTo(x0, y0);
      g.lineTo(x1, y1);
      g.stroke();
    }
    return c;
  }

  function scintillationWaves() {
    const knee = 0.9, low = 0.12, high = 2.6, count = 4;
    const waves = [];
    let power = 0;
    for (let k = 0; k < count; k++) {
      const f = low * (high / low) ** ((k + 0.6 * rand()) / count);
      const amp = Math.sqrt(1 / (1 + (f / knee) ** (8 / 3)));
      waves.push([2 * Math.PI * f, amp, 2 * Math.PI * rand()]);
      power += (amp * amp) / 2;
    }
    const norm = 1 / Math.sqrt(power);
    waves.forEach(w => { w[1] *= norm; });
    return waves;
  }

  function place(star) {
    for (let k = 0; k < 80; k++) {
      const x = rand() * PLATE.w, y = rand() * PLATE.h;
      if (openSky(x, y)) {
        star.x = x;
        star.y = y;
        return;
      }
    }
    star.x = 1180 + rand() * 300;
    star.y = 40 + rand() * 180;
  }

  function ignite(star, t, early) {
    place(star);
    const m = Math.log10(1 + rand() * (10 ** (0.6 * MAG_LIMIT) - 1)) / 0.6;
    star.flux = 10 ** (-0.4 * m);
    star.radius = 0.95 + 2.3 * Math.sqrt(star.flux);
    star.base = Math.min(1, 0.55 + 0.75 * Math.sqrt(star.flux));
    const kelvin = (3300 + 8200 * rand() ** 1.6) * (star.x / PLATE.w < 0.5 ? 0.82 : 1.18);
    star.sprite = sprite(blackbody(kelvin, 0.35));
    const zenith = ((15 + 63 * (star.y / PLATE.h)) * Math.PI) / 180;
    star.sigma = Math.min(0.9, 0.16 * (1 / Math.cos(zenith)) ** 1.75);
    star.waves = scintillationWaves();
    star.rise = 1.2 + 2.2 * rand();
    star.hold = 5 + 16 * rand();
    star.fade = 0.8 + 1.8 * rand();
    star.rest = 2 + 10 * rand();
    star.born = early ? t - rand() * (star.rise + star.hold) : t + rand() * star.rest;
  }

  function envelope(star, t) {
    const age = t - star.born;
    if (age < 0) return 0;
    if (age < star.rise) return smoothstep(0, 1, age / star.rise);
    const decay = age - star.rise - star.hold;
    return decay < 0 ? 1 : Math.exp(-decay / star.fade);
  }

  function scintillation(star, t) {
    let x = 0;
    for (const [w, a, p] of star.waves) x += a * Math.sin(w * t + p);
    return Math.exp(star.sigma * x - (star.sigma * star.sigma) / 2);
  }

  function pulse(u) {
    const r = u / 0.22;
    return r * r * Math.exp(2 * (1 - r)) * (1 - u ** 6);
  }

  function buildFigureMask() {
    const c = document.createElement("canvas");
    c.width = PLATE.w;
    c.height = PLATE.h;
    const g = c.getContext("2d");
    const blurred = document.createElement("canvas");
    blurred.width = PLATE.w;
    blurred.height = PLATE.h;
    const b = blurred.getContext("2d");
    g.fillStyle = "#fff";
    g.strokeStyle = "#fff";
    g.lineWidth = 18;
    g.lineJoin = "round";
    g.beginPath();
    FIGURE.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
    g.closePath();
    g.fill();
    g.stroke();
    b.filter = "blur(10px)";
    b.drawImage(c, 0, 0);
    b.filter = "none";
    return blurred;
  }

  function buildGlow() {
    const x0 = Math.min(TITLE.x, CORE.x), y0 = TITLE.y;
    const x1 = Math.max(TITLE.x + TITLE.w, CORE.x + CORE.w), y1 = CORE.y + CORE.h;
    const w = x1 - x0, h = y1 - y0;
    const source = plate.getContext("2d").getImageData(x0, y0, w, h);
    const shield = figureMask.getContext("2d").getImageData(x0, y0, w, h).data;
    const px = source.data;
    for (let j = 0; j < h; j++) {
      for (let i = 0; i < w; i++) {
        const k = (j * w + i) * 4, x = x0 + i, y = y0 + j;
        const lum = (0.2126 * px[k] + 0.7152 * px[k + 1] + 0.0722 * px[k + 2]) / 255;
        const inTitle = x >= TITLE.x && x < TITLE.x + TITLE.w && y >= TITLE.y && y < TITLE.y + TITLE.h ? 1 : 0;
        const dx = x - CORE.cx, dy = y - CORE.cy;
        const inCore = x >= CORE.x && x < CORE.x + CORE.w && y >= CORE.y && y < CORE.y + CORE.h;
        const lorentz = inCore ? 1 / (1 + (dx * dx + dy * dy) / (CORE.d0 * CORE.d0)) : 0;
        const weight = smoothstep(0.28, 0.72, lum) * Math.max(inTitle, lorentz) * (1 - shield[k + 3] / 255);
        px[k + 3] = Math.round(255 * weight);
      }
    }
    const layer = document.createElement("canvas");
    layer.width = w;
    layer.height = h;
    layer.getContext("2d").putImageData(source, 0, 0);
    const pad = 28;
    const bloom = document.createElement("canvas");
    bloom.width = w + 2 * pad;
    bloom.height = h + 2 * pad;
    const bg = bloom.getContext("2d");
    bg.filter = "blur(9px)";
    bg.drawImage(layer, pad, pad);
    bg.filter = "none";
    return { layer, bloom, x: x0, y: y0, pad };
  }

  function resize() {
    const rect = canvas.getBoundingClientRect();
    view.dpr = Math.min(window.devicePixelRatio || 1, 2);
    view.w = Math.max(1, Math.round(rect.width));
    view.h = Math.max(1, Math.round(rect.height));
    view.s = Math.max(view.w / PLATE.w, view.h / PLATE.h);
    view.ox = (view.w - PLATE.w * view.s) / 2;
    view.oy = (view.h - PLATE.h * view.s) / 2;
    canvas.width = Math.round(view.w * view.dpr);
    canvas.height = Math.round(view.h * view.dpr);
    energy = energy || document.createElement("canvas");
    energy.width = Math.max(1, Math.ceil(view.w / 4));
    energy.height = Math.max(1, Math.ceil(view.h / 4));
    ectx = energy.getContext("2d");
    if (reduced && plate) render(0, 0);
  }

  function lobe(g, cx, cy, rx, ry, rgb, alpha) {
    if (alpha <= 0.002) return;
    g.save();
    g.translate(cx, cy);
    g.scale(rx / ry, 1);
    const grad = g.createRadialGradient(0, 0, 0, 0, 0, ry);
    grad.addColorStop(0, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${alpha})`);
    grad.addColorStop(0.7, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},0)`);
    g.fillStyle = grad;
    g.fillRect(-ry, -ry, 2 * ry, 2 * ry);
    g.restore();
  }

  function drawEnergy(t) {
    const q = 0.25, tau = 2 * Math.PI;
    ectx.setTransform(1, 0, 0, 1, 0, 0);
    ectx.clearRect(0, 0, energy.width, energy.height);
    ectx.setTransform(view.s * q, 0, 0, view.s * q, view.ox * q, view.oy * q);
    ectx.globalCompositeOperation = "lighter";
    lobe(ectx, 460 + 34 * Math.sin((tau * t) / 46), 363 + 22 * Math.sin((tau * t) / 58 + 1.3),
      583 * (1 + 0.04 * Math.sin((tau * t) / 71)), 475, [229, 170, 70], 0.55 * mood.warm);
    lobe(ectx, 1106 + 30 * Math.sin((tau * t) / 58 + 2), 346 + 24 * Math.sin((tau * t) / 46 + 0.4),
      583 * (1 + 0.04 * Math.sin((tau * t) / 83 + 1)), 475, [47, 127, 255], 0.6 * mood.cool);
    const sweep = ((t / 72) % 1) * 2 - 1;
    ectx.save();
    ectx.translate(PLATE.w / 2 + sweep * PLATE.w * 0.62, PLATE.h / 2);
    ectx.rotate((-6 * Math.PI) / 180);
    const band = ectx.createLinearGradient(-260, 0, 260, 0);
    band.addColorStop(0, "rgba(255,223,154,0)");
    band.addColorStop(0.4, "rgba(255,223,154,0.07)");
    band.addColorStop(0.6, "rgba(120,180,255,0.09)");
    band.addColorStop(1, "rgba(120,180,255,0)");
    ectx.fillStyle = band;
    ectx.fillRect(-260, -PLATE.h, 520, PLATE.h * 2);
    ectx.restore();
    ectx.globalCompositeOperation = "destination-out";
    ectx.drawImage(figureMask, 0, 0);
    ectx.globalCompositeOperation = "source-over";
  }

  function drawGlow(level) {
    if (!glow || level <= 0.003) return;
    ctx.globalCompositeOperation = "lighter";
    ctx.globalAlpha = Math.min(1, level);
    ctx.drawImage(glow.layer, glow.x, glow.y);
    ctx.globalAlpha = Math.min(1, level * 0.85);
    ctx.drawImage(glow.bloom, glow.x - glow.pad, glow.y - glow.pad);
    const [r, g, b] = mood.tint;
    const halo = ctx.createRadialGradient(CORE.cx, CORE.cy, 0, CORE.cx, CORE.cy, 150);
    halo.addColorStop(0, `rgba(${r},${g},${b},${0.22 * level})`);
    halo.addColorStop(1, `rgba(${r},${g},${b},0)`);
    ctx.globalAlpha = 1;
    ctx.fillStyle = halo;
    ctx.fillRect(CORE.cx - 150, CORE.cy - 150, 300, 300);
  }

  function drawStars(t) {
    ctx.globalCompositeOperation = "lighter";
    for (const star of stars) {
      const age = t - star.born;
      if (age > star.rise + star.hold + 4.6 * star.fade + star.rest) {
        ignite(star, t, false);
        continue;
      }
      const e = envelope(star, t);
      if (e <= 0.004) continue;
      const shimmer = reduced ? 1 : scintillation(star, t);
      const a = Math.min(1, star.base * e * shimmer);
      const r = star.radius * (0.85 + 0.25 * Math.min(shimmer, 2));
      ctx.globalAlpha = a;
      ctx.drawImage(star.sprite, star.x - r * 3, star.y - r * 3, r * 6, r * 6);
      const flare = star.flux > 0.25 ? Math.max(0, a * (shimmer - 0.95)) : 0;
      if (flare > 0.01) {
        const span = 10 + 22 * star.flux;
        ctx.globalAlpha = Math.min(0.8, flare);
        ctx.drawImage(spike, star.x - span, star.y - span, 2 * span, 2 * span);
      }
    }
    ctx.globalAlpha = 1;
  }

  function render(t, dt) {
    const quiet = QUIET.has(shell.dataset.phase);
    const speaking = shell.dataset.phase === "NURA_SPEAKING";
    const [warm, cool] = EMPHASIS[shell.dataset.emphasis] || [0.3, 0.3];
    target.warm = warm;
    target.cool = cool;
    target.amp = quiet ? 0.06 : (0.45 + 0.55 * mood.glow) * (speaking ? 1.15 : 1);
    target.period = quiet ? 18 : Math.min(Math.max((2.6 * mood.tempo) / 1000, 3.2), 14);
    mood.warm = approach(mood.warm, target.warm, dt, 0.6);
    mood.cool = approach(mood.cool, target.cool, dt, 0.6);
    mood.amp = approach(mood.amp, target.amp, dt, 0.8);
    mood.period = approach(mood.period, target.period, dt, 1.5);
    phase = (phase + dt / mood.period) % 1;

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.globalCompositeOperation = "source-over";
    ctx.globalAlpha = 1;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    drawEnergy(t);
    ctx.setTransform(view.dpr, 0, 0, view.dpr, 0, 0);
    ctx.drawImage(energy, 0, 0, view.w, view.h);
    ctx.setTransform(view.dpr * view.s, 0, 0, view.dpr * view.s, view.dpr * view.ox, view.dpr * view.oy);
    drawGlow(reduced ? 0 : mood.amp * pulse(phase));
    drawStars(t);
    ctx.globalCompositeOperation = "source-over";
  }

  function frame(now) {
    raf = requestAnimationFrame(frame);
    if (now - last < 33) return;
    const dt = last ? Math.min(now - last, 100) / 1000 : 1 / 30;
    last = now;
    clock += dt;
    render(clock, dt);
  }

  function start() {
    if (reduced || raf || !plate) return;
    last = 0;
    raf = requestAnimationFrame(frame);
  }

  function stop() {
    cancelAnimationFrame(raf);
    raf = 0;
  }

  function begin(img) {
    plate = document.createElement("canvas");
    plate.width = PLATE.w;
    plate.height = PLATE.h;
    const pg = plate.getContext("2d", { willReadFrequently: true });
    figureMask = buildFigureMask();
    spike = spikeSprite();
    if (img) {
      pg.drawImage(img, 0, 0, PLATE.w, PLATE.h);
      try {
        lumen = pg.getImageData(0, 0, PLATE.w, PLATE.h).data;
        glow = buildGlow();
      } catch (_) {
        lumen = null;
        glow = null;
      }
    }
    for (let i = 0; i < STARS; i++) {
      const star = {};
      ignite(star, 0, i % 10 < 7);
      stars.push(star);
    }
    resize();
    render(0, 0);
    start();
  }

  window.addEventListener("resize", resize);
  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start()));
  resize();

  const img = new Image();
  img.onload = () => begin(img);
  img.onerror = () => begin(null);
  img.src = PLATE.src;

  window.nuraScene = {
    mood(color, glowMass, tempoMs) {
      const hex = /^#?([0-9a-f]{6})$/i.exec(String(color || "").trim());
      if (hex) {
        const n = parseInt(hex[1], 16);
        mood.tint = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
      }
      mood.glow = Math.min(Math.max((glowMass ?? 2500) / 10000, 0), 1);
      mood.tempo = tempoMs > 0 ? tempoMs : 4200;
    }
  };
})();
