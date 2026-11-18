/* Widgets for the landing page. Everything runs on truchet.js, the exact port of the crate. */
(function () {
  'use strict';
  var T = window.Truchet;
  var INK = [226, 226, 222], GROUND = [16, 16, 16];
  var SQUARE = T.TYPE.ALL, ROUND = T.TYPE.ROUND;

  function h(tag, attrs) {
    var el = document.createElement(tag), k, i, c;
    attrs = attrs || {};
    for (k in attrs) { if (k === 'text') el.textContent = attrs[k]; else if (k === 'html') el.innerHTML = attrs[k]; else if (k.slice(0, 2) === 'on') el.addEventListener(k.slice(2), attrs[k]); else el.setAttribute(k, attrs[k]); }
    for (i = 2; i < arguments.length; i++) { c = arguments[i]; if (c) el.appendChild(typeof c === 'string' ? document.createTextNode(c) : c); }
    return el;
  }
  function raf(fn) { var q = false; return function () { if (q) return; q = true; requestAnimationFrame(function () { q = false; fn(); }); }; }
  function slider(label, min, max, step, val, change, fmt) {
    var out = h('output', { text: fmt ? fmt(val) : val });
    var inp = h('input', { type: 'range', min: min, max: max, step: step, value: val, 'aria-label': label });
    inp.addEventListener('input', function () { out.textContent = fmt ? fmt(+inp.value) : inp.value; change(+inp.value); });
    var l = h('label', {}, label, inp, out);
    l.get = function () { return +inp.value; };
    l.set = function (v) { inp.value = v; out.textContent = fmt ? fmt(v) : v; };
    return l;
  }
  function seg(opts, val, change) {
    var wrap = h('span', { 'class': 'seg' }), btns = [];
    opts.forEach(function (o) {
      var b = h('button', { type: 'button', text: o[1] });
      if (o[0] === val) b.className = 'on';
      b.addEventListener('click', function () { btns.forEach(function (x) { x.className = ''; }); b.className = 'on'; change(o[0]); });
      btns.push(b); wrap.appendChild(b);
    });
    return wrap;
  }
  function canvas(w, hgt) { var c = h('canvas', { 'class': 'cv', width: w, height: hgt || w }); return c; }
  function pixels(c, fn) {
    var ctx = c.getContext('2d'), w = c.width, hh = c.height, img = ctx.createImageData(w, hh), d = img.data, i = 0, j, k, col;
    for (j = 0; j < hh; j++) for (k = 0; k < w; k++) {
      col = fn(k, j);
      if (col === null) { d[i++] = 0; d[i++] = 0; d[i++] = 0; d[i++] = 0; }
      else { col = col === true ? INK : col === false ? GROUND : col; d[i++] = col[0]; d[i++] = col[1]; d[i++] = col[2]; d[i++] = 255; }
    }
    ctx.putImageData(img, 0, 0);
  }
  function pickedSeed() { return 1 + Math.floor(Math.random() * 99); }
  var seedNum = function (v, change) {
    var i = h('input', { type: 'number', min: 0, max: 9999, value: v, style: 'width:4.6em;background:#111;color:inherit;border:2px solid var(--line);padding:4px 6px;font:inherit' });
    i.addEventListener('input', function () { change(Math.max(0, Math.floor(+i.value || 0))); });
    var shuffle = h('button', { type: 'button', text: 'shuffle', onclick: function () { var s = pickedSeed(); i.value = s; change(s); } });
    var l = h('label', {}, 'seed', i); var box = h('span', { style: 'display:inline-flex;gap:8px;align-items:center' }, l, shuffle);
    box.set = function (v) { i.value = v; };
    return box;
  };
  function types(set) { return set === 'round' ? ROUND : SQUARE; }

  /* ---------- 01 one tile ---------- */
  var EXT = 0.28; // how far past the tile's square we draw, in tile units
  var ACCENT = [255, 120, 80];
  function portInk(set, u, v) { // the dots on the edges overhang the square; draw them whole
    var x = u + 0.5, y = 0.5 - v, i, k, d;
    if (set === 'round') {
      for (i = 0; i < 2; i++) { k = i ? 0.75 : 0.25;
        if (Math.hypot(x - k, y) < 1 / 12 || Math.hypot(x - k, y - 1) < 1 / 12 || Math.hypot(x, y - k) < 1 / 12 || Math.hypot(x - 1, y - k) < 1 / 12) return true; }
      return false;
    }
    return Math.hypot(u, v - 0.5) < 1 / 6 || Math.hypot(u, v + 0.5) < 1 / 6 || Math.hypot(u - 0.5, v) < 1 / 6 || Math.hypot(u + 0.5, v) < 1 / 6;
  }
  function tileCanvas(c, set, name, rot, margin, dotted) {
    margin = margin || 0; var span = 1 + 2 * margin;
    pixels(c, function (x, y) {
      var u = (x + 0.5) / c.width * span - 0.5 - margin, v = 0.5 + margin - (y + 0.5) / c.height * span;
      if (Math.abs(u) > 0.5 || Math.abs(v) > 0.5) return portInk(set, u, v) ? [118, 118, 114] : false;
      return T.tileInk(set, name, rot, u, v);
    });
    if (dotted) {
      var g = c.getContext('2d'), a = margin / span * c.width, w = c.width / span;
      g.save(); g.strokeStyle = 'rgb(' + ACCENT.join(',') + ')'; g.lineWidth = Math.max(1.5, c.width / 220); g.setLineDash([c.width / 60, c.width / 60]);
      g.strokeRect(a, a, w, w); g.restore();
    }
  }
  function names(set) { return set === 'round' ? T.ROUND.map(function (t) { return t.name; }) : T.CARLSON; }
  function wTile(root) {
    var set = 'round', name = 'SadFace', rot = 0, timer = null;
    var big = canvas(620), pick = h('div', { 'class': 'pick' }), strip = h('div', { 'class': 'strip' }), strips = [];
    for (var r = 0; r < 4; r++) { var c = canvas(240); strips.push(c); strip.appendChild(h('figure', {}, c, h('figcaption', { text: (r * 90) + '\u00B0' }))); }
    var label = h('div', { 'class': 'readout' });
    var spin = h('button', { type: 'button', 'aria-pressed': 'false', text: 'spin' });
    function draw() {
      tileCanvas(big, set, name, rot, EXT, true);
      strips.forEach(function (c, i) { tileCanvas(c, set, name, i, EXT, true); });
      var turns = set === 'round' ? T.ROUND.filter(function (t) { return t.name === name; })[0].turns : 4;
      label.innerHTML = '<b>' + name + '</b> \u00B7 ' + (set === 'round' ? 'dot' : 'band') + ' set \u00B7 rotated <b>' + (rot * 90) + '\u00B0</b> \u00B7 ' + (turns === 4 ? 'four distinct turns' : turns === 2 ? 'two distinct turns' : 'one turn (looks the same any way up)');
    }
    function buildPick() {
      pick.innerHTML = '';
      names(set).forEach(function (n) {
        var c = h('canvas', { width: 96, height: 96, title: n }); if (n === name) c.className = 'on';
        tileCanvas(c, set, n, 0, 0.2, true);
        c.addEventListener('click', function () { name = n; Array.prototype.forEach.call(pick.children, function (x) { x.className = ''; }); c.className = 'on'; draw(); });
        pick.appendChild(c);
      });
    }
    var setSeg = seg([['square', 'band set'], ['round', 'dot set']], set, function (v) { set = v; name = names(set)[set === 'round' ? 4 : 11]; rot = 0; buildPick(); draw(); });
    var rotBtn = h('button', { type: 'button', text: 'rotate 90\u00B0', onclick: function () { rot = (rot + 1) % 4; draw(); } });
    spin.addEventListener('click', function () {
      if (timer) { clearInterval(timer); timer = null; spin.setAttribute('aria-pressed', 'false'); return; }
      spin.setAttribute('aria-pressed', 'true'); timer = setInterval(function () { rot = (rot + 1) % 4; draw(); }, 800);
    });
    root.className = 'w';
    root.append(big, h('div', { 'class': 'row' }, setSeg, rotBtn, spin), strip, label, pick);
    buildPick(); draw();
  }

  /* ---------- 02 every edge matches ---------- */
  function wEdges(root) {
    var set = 'round', seed = 11, lines = true, c = canvas(480);
    var draw = raf(function () {
      var m = T.Mosaic({ types: types(set), seed: seed, levels: 0, width: 4, height: 4, noise: 0 });
      pixels(c, function (x, y) { return m.ink(4 * (x + 0.5) / c.width, 4 * (y + 0.5) / c.height); });
      if (lines) {
        var g = c.getContext('2d'); g.strokeStyle = 'rgba(255,90,60,.9)'; g.lineWidth = 1.5; g.setLineDash([5, 5]);
        for (var i = 1; i < 4; i++) { var p = i * c.width / 4; g.beginPath(); g.moveTo(p, 0); g.lineTo(p, c.height); g.moveTo(0, p); g.lineTo(c.width, p); g.stroke(); }
      }
    });
    var s = seedNum(seed, function (v) { seed = v; draw(); });
    var setSeg = seg([['round', 'dot set'], ['square', 'band set']], set, function (v) { set = v; draw(); });
    var tog = h('button', { type: 'button', 'aria-pressed': 'true', text: 'show tile edges', onclick: function () { lines = !lines; tog.setAttribute('aria-pressed', lines); draw(); } });
    root.className = 'w'; root.append(c, h('div', { 'class': 'row' }, setSeg, s, tog)); draw();
  }

  /* ---------- 03 scales ---------- */
  function wScales(root) {
    var st = { set: 'round', seed: 3, levels: 4, split: 0.6, noise: 0.1 }, c = canvas(520), timer = null;
    var draw = raf(function () {
      var m = T.Mosaic({ types: types(st.set), seed: st.seed, levels: st.levels, split: st.split, noise: st.noise, width: 4, height: 4 });
      pixels(c, function (x, y) { return m.ink(4 * (x + 0.5) / c.width, 4 * (y + 0.5) / c.height); });
    });
    var lv = slider('levels', 0, 6, 1, st.levels, function (v) { st.levels = v; draw(); });
    var sp = slider('split', 0, 1, 0.05, st.split, function (v) { st.split = v; draw(); });
    var setSeg = seg([['round', 'dot set'], ['square', 'band set']], st.set, function (v) { st.set = v; draw(); });
    var s = seedNum(st.seed, function (v) { st.seed = v; draw(); });
    var play = h('button', { type: 'button', text: 'play levels 0 → 6', onclick: function () {
      if (timer) clearInterval(timer); var n = 0; st.levels = 0; lv.set(0); draw();
      timer = setInterval(function () { n++; st.levels = n; lv.set(n); draw(); if (n >= 6) { clearInterval(timer); timer = null; } }, 750);
    } });
    root.className = 'w'; root.append(c, h('div', { 'class': 'row' }, lv, sp), h('div', { 'class': 'row' }, setSeg, s, play)); draw();
  }

  /* ---------- 04 grade ---------- */
  function wGrade(root) {
    var st = { set: 'round', seed: 3, levels: 5, grade: 0.5 }, c = canvas(600, 300);
    var draw = raf(function () {
      var m = T.Mosaic({ types: types(st.set), seed: st.seed, levels: st.levels, split: 0.6, noise: 0.1, grade: st.grade, width: 6, height: 6 });
      pixels(c, function (x, y) { return m.ink(6 * (x + 0.5) / c.width, 3 * (1 - (y + 0.5) / c.height)); });
      var g = c.getContext('2d'); g.fillStyle = 'rgba(255,90,60,.95)'; g.fillRect(0, c.height - 2, c.width, 2);
      g.font = '12px ui-monospace,monospace'; g.fillText('y = 0', 8, c.height - 8);
    });
    var gr = slider('grade', 0.1, 1.4, 0.05, st.grade, function (v) { st.grade = v; draw(); }, function (v) { return v.toFixed(2); });
    var lv = slider('levels', 2, 7, 1, st.levels, function (v) { st.levels = v; draw(); });
    var setSeg = seg([['round', 'dot set'], ['square', 'band set']], st.set, function (v) { st.set = v; draw(); });
    var s = seedNum(st.seed, function (v) { st.seed = v; draw(); });
    root.className = 'w'; root.append(c, h('div', { 'class': 'row' }, gr, lv), h('div', { 'class': 'row' }, setSeg, s));
    var out = snip(root); var upd = function () { out(tiles(st, 0)); }; root.addEventListener('input', upd); root.addEventListener('click', function () { setTimeout(upd, 0); }); upd(); draw();
  }

  /* ---------- the disk, shared by 05 and 07 ---------- */
  function diskInk(m, wr, wi, off) { // z = i(1+w)/(1-w), then ink(Re z, Im z)
    var nr = 1 + wr, ni = wi, dr = 1 - wr, di = -wi, den = dr * dr + di * di;
    if (den < 1e-9) return false;
    var qr = (nr * dr + ni * di) / den, qi = (ni * dr - nr * di) / den;
    return m.ink(-qi + off, qr);
  }
  function wDisk(root) {
    var st = { set: 'round', seed: 3, levels: 5, grade: 0.5, off: 0 }, c = canvas(520), anim = null;
    function render(res) {
      c.width = c.height = res;
      var m = T.Mosaic({ types: types(st.set), seed: st.seed, levels: st.levels, split: 0.6, noise: 0.1, grade: st.grade, width: 6, height: 6 });
      pixels(c, function (x, y) {
        var wr = (x + 0.5) / res * 2 - 1, wi = 1 - (y + 0.5) / res * 2;
        if (wr * wr + wi * wi > 0.992) return wr * wr + wi * wi > 1 ? null : [90, 90, 88];
        return diskInk(m, wr, wi, st.off);
      });
    }
    var still = raf(function () { if (!anim) render(520); });
    var gr = slider('grade', 0.2, 1.2, 0.05, st.grade, function (v) { st.grade = v; still(); }, function (v) { return v.toFixed(2); });
    var lv = slider('levels', 2, 7, 1, st.levels, function (v) { st.levels = v; still(); });
    var setSeg = seg([['round', 'dot set'], ['square', 'band set']], st.set, function (v) { st.set = v; still(); });
    var s = seedNum(st.seed, function (v) { st.seed = v; still(); });
    var drift = h('button', { type: 'button', 'aria-pressed': 'false', text: 'slide the plane' });
    function tick() { if (!anim) return; st.off += 0.035; render(250); anim = requestAnimationFrame(tick); }
    drift.addEventListener('click', function () {
      if (anim) { cancelAnimationFrame(anim); anim = null; drift.setAttribute('aria-pressed', 'false'); render(520); return; }
      drift.setAttribute('aria-pressed', 'true'); anim = requestAnimationFrame(tick);
    });
    root.className = 'w'; root.append(c, h('div', { 'class': 'row' }, gr, lv), h('div', { 'class': 'row' }, setSeg, s, drift));
    var out = snip(root); var upd = function () { out(tiles(st, 3)); }; root.addEventListener('input', upd); root.addEventListener('click', function () { setTimeout(upd, 0); }); upd(); render(520);
  }

  /* ---------- 06 packing ---------- */
  function wPacking(root) {
    var st = { p: 6, q: 4, gap: 1 }, NS = 'http://www.w3.org/2000/svg';
    var svg = document.createElementNS(NS, 'svg'); svg.setAttribute('viewBox', '-210 -210 420 420'); svg.setAttribute('class', 'cv'); svg.style.background = '#101010';
    var out = h('div', { 'class': 'readout' });
    var S = 195;
    function circ(cx, cy, r, fill, stroke, op) {
      var e = document.createElementNS(NS, 'circle'); e.setAttribute('cx', cx * S); e.setAttribute('cy', -cy * S); e.setAttribute('r', Math.max(r * S, 0));
      e.setAttribute('fill', fill); e.setAttribute('fill-opacity', op == null ? 1 : op); e.setAttribute('stroke', stroke || 'none'); e.setAttribute('stroke-width', 1.5); svg.appendChild(e);
    }
    function hyp(dist, rad, ang, fill, stroke, op) { // hyperbolic circle: radius rad, centre dist from origin at angle ang
      var a = Math.tanh(Math.abs(dist - rad) / 2), b = Math.tanh((dist + rad) / 2), r = (b - a) / 2, mid = dist < rad ? 0 : (a + b) / 2;
      if (dist < 1e-9) { r = Math.tanh(rad / 2); mid = 0; }
      circ(mid * Math.cos(ang), mid * Math.sin(ang), r, fill, stroke, op);
    }
    function draw() {
      var p = st.p, q = st.q; svg.innerHTML = '';
      circ(0, 0, 1, '#161616', '#d6d6d3');
      if ((p - 1) * (q - 1) <= 4) { out.innerHTML = '<b>{' + p + ',' + q + '}</b> is not hyperbolic: (p−1)(q−1) must be above 4. Raise p or q.'; return; }
      var rho = Math.acosh(Math.cos(Math.PI / q) / Math.sin(Math.PI / p));
      var R = Math.acosh(1 / (Math.tan(Math.PI / q) * Math.tan(Math.PI / p)));
      var free = R - rho, g = free * st.gap, k;
      for (k = 0; k < p; k++) hyp(R, g, Math.PI * (2 * k + 1) / p, '#ff5a3c', '#ff5a3c', 0.28);
      hyp(0, rho, 0, '#e2e2de', '#fff', 0.16);
      for (k = 0; k < p; k++) hyp(2 * rho, rho, 2 * Math.PI * k / p, '#e2e2de', '#fff', 0.16);
      out.innerHTML = '<b>{' + p + ',' + q + '}</b><br>bubble: hyperbolic radius <b>' + rho.toFixed(4) + '</b> · linear scale <b>' + Math.tanh(rho / 2).toFixed(5) + '</b><br>' +
        'corner (where q bubbles meet): distance <b>' + R.toFixed(4) + '</b> · <b>' + Math.tanh(R / 2).toFixed(5) + '</b> from the centre<br>' +
        'free room in the corner gap: <b>' + free.toFixed(4) + '</b> · gap circle radius <b>' + Math.tanh(g / 2).toFixed(5) + '</b>';
    }
    var P = slider('p (sides)', 3, 12, 1, st.p, function (v) { st.p = v; draw(); });
    var Q = slider('q (meet at a corner)', 3, 12, 1, st.q, function (v) { st.q = v; draw(); });
    var G = slider('gap fill', 0, 1, 0.05, st.gap, function (v) { st.gap = v; draw(); }, function (v) { return Math.round(v * 100) + '%'; });
    root.className = 'w'; root.append(svg, h('div', { 'class': 'row' }, P, Q, G), out); draw();
  }


  /* ---------- helpers for snippets ---------- */
  function wrapAttrs(tag, pairs, indent, width) {
    var lines = [], cur = '<' + tag, pad = new Array(indent + 1).join(' ');
    pairs.forEach(function (p) {
      var t = ' ' + p[0] + '="' + p[1] + '"';
      if (cur.length + t.length > width) { lines.push(cur); cur = pad + t.slice(1); } else cur += t;
    });
    lines.push(cur + ' />'); return lines.join('\n');
  }
  function snip(root, text) {
    var code = h('code', { text: text || '' }), pre = h('pre', { 'class': 'snip' }, code), xml = text || '';
    var copy = h('button', { 'class': 'copy', type: 'button', text: 'copy', onclick: function () {
      if (navigator.clipboard) navigator.clipboard.writeText(xml).then(function () { copy.textContent = 'copied'; setTimeout(function () { copy.textContent = 'copy'; }, 1300); }, function () {});
    } });
    pre.appendChild(copy); root.appendChild(pre);
    return function (t) { xml = t; code.textContent = t; };
  }
  function tiles(o, mode) {
    var p = [['weight', '1'], ['color', '0.5'], ['linear', '0'], ['truchet_ms', '1'], ['truchet_ms_seed', o.seed], ['truchet_ms_levels', o.levels]];
    if (!(o.grade > 0)) p.push(['truchet_ms_split', o.split]);
    p.push(['truchet_ms_noise', o.noise == null ? 0.1 : o.noise]);
    p.push(['truchet_ms_style', o.set === 'round' ? 1 : 0]);
    if (o.set !== 'round') p.push(['truchet_ms_types', 255]);
    p.push(['truchet_ms_mode', mode]);
    if (o.grade > 0) p.push(['truchet_ms_grade', o.grade]);
    p.push(['coefs', '1 0 0 1 0 0'], ['opacity', '1']);
    return wrapAttrs('xform', p, 7, 74);
  }

  /* ---------- the splitting animation (03 and 04) ---------- */
  function wSplit(root) {
    var graded = root.getAttribute('data-grade') === '1';
    var MAXL = graded ? 5 : 4, W = graded ? 640 : 480, H = graded ? 320 : 480, RX = graded ? 4 : 2, RY = graded ? 2 : 2;
    var st = { set: 'round', seed: graded ? 3 : 6, k: 0 }, cache = {}, mosaic = null, playing = false, timer = 0, phase = 'hold', t0 = 0, raf1 = 0;
    var c = canvas(W, H), g = c.getContext('2d');
    var readout = h('div', { 'class': 'readout' });
    function make() {
      mosaic = T.Mosaic({ types: st.set === 'round' ? ROUND : SQUARE, seed: st.seed, levels: MAXL, split: 0.7, noise: 0.1, grade: graded ? 0.55 : 0, width: 6, height: 6 });
      cache = {};
    }
    function level(k) { // pixels of the mosaic cut off at k levels
      if (cache[k]) return cache[k];
      var m = T.Mosaic({ types: st.set === 'round' ? ROUND : SQUARE, seed: st.seed, levels: k, split: 0.7, noise: 0.1, grade: graded ? 0.55 : 0, width: 6, height: 6 });
      var cv = document.createElement('canvas'); cv.width = W; cv.height = H;
      pixels(cv, function (x, y) { return m.ink(RX * (x + 0.5) / W, graded ? RY * (1 - (y + 0.5) / H) : RY * (y + 0.5) / H); });
      return (cache[k] = cv);
    }
    function leaves(k) {
      var out = [], x, y;
      (function rec(cx, cy, l) {
        if (l < k && mosaic.splits(cx, cy, l)) { var s = Math.pow(0.5, l) * 0.25; rec(cx - s, cy - s, l + 1); rec(cx + s, cy - s, l + 1); rec(cx - s, cy + s, l + 1); rec(cx + s, cy + s, l + 1); }
        else out.push([cx, cy, l]);
      })(0, 0, 0);
      return out;
    }
    function allLeaves(k) {
      var out = [], cx, cy;
      for (cx = 0.5; cx < RX; cx++) for (cy = 0.5; cy < RY; cy++) (function rec(x, y, l) {
        if (l < k && mosaic.splits(x, y, l)) { var s = Math.pow(0.5, l) * 0.25; rec(x - s, y - s, l + 1); rec(x + s, y - s, l + 1); rec(x - s, y + s, l + 1); rec(x + s, y + s, l + 1); }
        else out.push([x, y, l]);
      })(cx, cy, 0);
      return out;
    }
    function toPx(cx, cy, half) { // leaf cell in canvas pixels
      var sx = W / RX, sy = H / RY, x0 = (cx - half) * sx, x1 = (cx + half) * sx;
      if (graded) return [x0, (RY - (cy + half)) * sy, x1 - x0, 2 * half * sy];
      return [x0, (cy - half) * sy, x1 - x0, 2 * half * sy];
    }
    function drawFrame(k, p) { // p in 0..1 = how far the splitting lines have grown (null: none)
      g.drawImage(level(k), 0, 0);
      var L = allLeaves(k), splitting = 0, i, r, half;
      g.save(); g.lineWidth = 1.5; g.setLineDash([6, 5]); g.strokeStyle = 'rgba(' + ACCENT.join(',') + ',.85)';
      for (i = 0; i < L.length; i++) { half = Math.pow(0.5, L[i][2]) / 2; r = toPx(L[i][0], L[i][1], half); g.strokeRect(r[0] + 0.5, r[1] + 0.5, r[2] - 1, r[3] - 1); }
      g.restore();
      if (p != null) {
        g.save(); g.lineWidth = 3; g.strokeStyle = 'rgb(' + ACCENT.join(',') + ')'; g.fillStyle = 'rgba(' + ACCENT.join(',') + ',' + (0.18 * Math.sin(p * Math.PI)).toFixed(3) + ')';
        for (i = 0; i < L.length; i++) {
          if (L[i][2] !== k || !mosaic.splits(L[i][0], L[i][1], k)) continue;
          half = Math.pow(0.5, k) / 2; r = toPx(L[i][0], L[i][1], half); splitting++;
          g.fillRect(r[0], r[1], r[2], r[3]);
          var mx = r[0] + r[2] / 2, my = r[1] + r[3] / 2, a = r[2] / 2 * p, b = r[3] / 2 * p;
          g.beginPath(); g.moveTo(mx - a, my); g.lineTo(mx + a, my); g.moveTo(mx, my - b); g.lineTo(mx, my + b); g.stroke();
        }
        g.restore();
      }
      var n = L.length, nxt = 0; for (i = 0; i < L.length; i++) if (L[i][2] === k && k < MAXL && mosaic.splits(L[i][0], L[i][1], k)) nxt++;
      readout.innerHTML = 'level <b>' + k + '</b> · <b>' + n + '</b> tile' + (n === 1 ? '' : 's') + ' · ' + (k < MAXL ? '<b>' + nxt + '</b> about to split into four' : 'nothing deeper allowed (levels = ' + MAXL + ')');
    }
    function stopTimer() { cancelAnimationFrame(raf1); clearTimeout(timer); }
    function show() { drawFrame(st.k, null); }
    function tick(now) {
      if (!playing) return;
      var el = now - t0;
      if (phase === 'hold') { if (el > (st.k === MAXL ? 1900 : 1100)) { if (st.k === MAXL) { st.k = 0; drawFrame(0, null); t0 = now; } else { phase = 'split'; t0 = now; } } }
      else { var p = Math.min(1, el / 800); drawFrame(st.k, p); if (p >= 1) { st.k++; phase = 'hold'; t0 = now; drawFrame(st.k, null); } }
      raf1 = requestAnimationFrame(tick);
    }
    var play = h('button', { type: 'button', 'aria-pressed': 'false', text: 'play' });
    function setPlay(on) { playing = on; play.setAttribute('aria-pressed', on); play.textContent = on ? 'pause' : 'play'; stopTimer(); if (on) { phase = 'hold'; t0 = performance.now(); raf1 = requestAnimationFrame(tick); } }
    play.addEventListener('click', function () { setPlay(!playing); });
    var step = h('button', { type: 'button', text: 'step', onclick: function () { setPlay(false); st.k = st.k >= MAXL ? 0 : st.k + 1; show(); } });
    var setSeg = seg([['round', 'dot set'], ['square', 'band set']], st.set, function (v) { st.set = v; make(); show(); });
    var s = seedNum(st.seed, function (v) { st.seed = v; make(); st.k = 0; show(); });
    root.className = 'w'; root.append(c, readout, h('div', { 'class': 'row' }, play, step, setSeg, s));
    make(); show();
    var reduce = window.matchMedia && matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!reduce && 'IntersectionObserver' in window) {
      var io = new IntersectionObserver(function (es) { if (es[0].isIntersecting) { if (!playing && !root._touched) setPlay(true); } else if (playing) setPlay(false); }, { threshold: 0.35 });
      io.observe(root);
      root.addEventListener('click', function () { root._touched = true; }, true);
    }
  }

  /* ---------- flame-driven blocks (the real file, so they stay true) ---------- */
  var flameCache = {};
  function loadFlame(url) {
    if (!flameCache[url]) flameCache[url] = fetch(url).then(function (r) { if (!r.ok) throw new Error(r.status); return r.text(); }).then(function (t) {
      var doc = new DOMParser().parseFromString(t, 'text/xml');
      return { text: t, xforms: Array.prototype.slice.call(doc.querySelectorAll('xform')) };
    });
    return flameCache[url];
  }
  function xformText(x, omit) {
    var pairs = [], i; omit = (omit || '').split(',');
    for (i = 0; i < x.attributes.length; i++) if (omit.indexOf(x.attributes[i].name) < 0) pairs.push([x.attributes[i].name, x.attributes[i].value]);
    return wrapAttrs('xform', pairs, 7, 74);
  }
  function wParts(root) {
    var want = root.getAttribute('data-names').split(','), omit = root.getAttribute('data-omit') || '', url = root.getAttribute('data-src');
    root.className = 'w';
    var set = snip(root, 'loading…');
    loadFlame(url).then(function (f) {
      var out = [];
      want.forEach(function (n) {
        var x = f.xforms.filter(function (e) { return e.getAttribute('name') === n; })[0];
        out.push(x ? xformText(x, omit) : '<!-- no transform named "' + n + '" in ' + url + ' -->');
      });
      set(out.join('\n'));
    }).catch(function () { set('Could not load ' + url + '.'); });
  }
  /* ---------- xaos diagrams (07) ---------- */
  var SVGNS = 'http://www.w3.org/2000/svg';
  function svgEl(tag, attrs, parent) { var e = document.createElementNS(SVGNS, tag); for (var k in attrs) e.setAttribute(k, attrs[k]); if (parent) parent.appendChild(e); return e; }
  function triPts(x, y, r) { return (x) + ',' + (y - r) + ' ' + (x - r * 0.95) + ',' + (y + r * 0.75) + ' ' + (x + r * 0.95) + ',' + (y + r * 0.75); }
  function chain(f) { // names, plotting flags, weights, row-normalised probabilities
    var n = f.xforms.length, names = [], plots = [], W = [], P = [], raw = [], i, k;
    f.xforms.forEach(function (x, i) { names.push(x.getAttribute('name') || 'xform ' + (i + 1)); var op = x.getAttribute('opacity'); plots.push(op == null || +op > 0); W.push(+(x.getAttribute('weight') || 1)); });
    f.xforms.forEach(function (x, i) {
      var row = (x.getAttribute('chaos') || '').trim().split(/\s+/).map(Number);
      if (row.length < n || row.some(isNaN)) row = names.map(function () { return 1; });
      raw.push(row.slice(0, n));
      var sum = 0; for (k = 0; k < n; k++) sum += row[k];
      P.push(sum > 0 ? row.slice(0, n).map(function (v) { return v / sum; }) : null);
    });
    var wsum = W.reduce(function (a, b) { return a + b; }, 0), w0 = W.map(function (v) { return v / wsum; });
    return { n: n, names: names, plots: plots, W: w0, P: P, raw: raw };
  }
  function longRun(C) { // where the points spend their time, and what gets plotted
    var v = C.W.slice(), it, i, k, nx;
    for (it = 0; it < 400; it++) { nx = C.names.map(function () { return 0; });
      for (i = 0; i < C.n; i++) for (k = 0; k < C.n; k++) nx[k] += v[i] * (C.P[i] ? C.P[i][k] : C.W[k]);
      v = nx; }
    var tot = 0; for (i = 0; i < C.n; i++) if (C.plots[i]) tot += v[i];
    return { time: v, plotted: v.map(function (x, i) { return C.plots[i] ? x / tot : 0; }) };
  }
  function pct(p) { return (p * 100 < 1 && p > 0 ? '<1' : Math.round(p * 100)) + '%'; }

  function wWeights(root) {
    var url = root.getAttribute('data-src'); root.className = 'w';
    var box = h('div', { 'class': 'cv', style: 'min-height:120px;padding:14px;color:var(--dim)', text: 'loading…' }); root.appendChild(box);
    loadFlame(url).then(function (f) {
      var C = chain(f), sel = h('select', { 'aria-label': 'transform' }), svg, read = h('div', { 'class': 'readout' });
      C.names.forEach(function (n, i) { sel.appendChild(h('option', { value: i, text: n })); });
      sel.value = Math.max(0, C.names.indexOf('bubble'));
      function draw() {
        var i = +sel.value, row = C.P[i], t = [], k, H;
        if (row) for (k = 0; k < C.n; k++) if (row[k] > 0) t.push(k);
        t.sort(function (a, b) { return row[b] - row[a]; });
        H = Math.max(230, 54 * t.length + 40);
        svg = svgEl('svg', { viewBox: '0 0 600 ' + H, 'class': 'cv', role: 'img' }); svg.style.background = '#101010';
        var cy = H / 2, sx = 92;
        svgEl('polygon', { points: triPts(sx, cy, 40), fill: C.plots[i] ? '#e2e2de' : 'none', stroke: '#e2e2de', 'stroke-width': 3 }, svg);
        var tx = svgEl('text', { x: sx, y: cy + 74, 'text-anchor': 'middle', fill: '#e2e2de', 'font-size': 15, 'font-weight': 700 }, svg); tx.textContent = C.names[i];
        var sub = svgEl('text', { x: sx, y: cy + 92, 'text-anchor': 'middle', fill: '#8d8d89', 'font-size': 12 }, svg); sub.textContent = C.plots[i] ? 'plots' : 'hidden';
        if (!row) { var e = svgEl('text', { x: 300, y: cy, 'text-anchor': 'middle', fill: '#8d8d89', 'font-size': 14 }, svg); e.textContent = 'all zero: the point starts over'; }
        t.forEach(function (k, j) {
          var y = 34 + j * (H - 68) / Math.max(1, t.length - 1 || 1); if (t.length === 1) y = cy;
          var p = row[k], x1 = sx + 52, x2 = 470, mx = (x1 + x2) / 2;
          svgEl('path', { d: 'M' + x1 + ',' + cy + ' C' + mx + ',' + cy + ' ' + mx + ',' + y + ' ' + (x2 - 22) + ',' + y, fill: 'none', stroke: '#ff7850', 'stroke-width': 1.5 + p * 16, 'stroke-opacity': 0.85 }, svg);
          svgEl('polygon', { points: triPts(x2, y, 17), fill: C.plots[k] ? '#e2e2de' : 'none', stroke: '#e2e2de', 'stroke-width': 2 }, svg);
          var nm = svgEl('text', { x: x2 + 30, y: y + 5, fill: '#e2e2de', 'font-size': 14, 'font-weight': 600 }, svg); nm.textContent = k === i ? C.names[k] + ' (itself)' : C.names[k];
          var pc = svgEl('text', { x: mx + 30, y: (cy + y) / 2 - 8, fill: '#ff9b7c', 'font-size': 14, 'font-weight': 700, 'font-family': 'ui-monospace,monospace' }, svg); pc.textContent = pct(p);
        });
        if (box.parentNode) box.parentNode.replaceChild(svg, box); else root.replaceChild(svg, root.querySelector('svg'));
        box = svg;
        var sum = 0; C.raw[i].forEach(function (v) { sum += v; });
        read.innerHTML = '<b>chaos</b> = ' + C.raw[i].map(function (v) { return +v.toFixed(3); }).join(' ') + '<br>Divide each by their total (' + +sum.toFixed(3) + '): ' + (t.length ? t.map(function (k) { return '<b>' + C.names[k] + '</b> ' + pct(row[k]); }).join(', ') : 'no weights') + '. That is the whole rule.';
      }
      var top = h('div', { 'class': 'row' }, h('label', {}, 'transform ', sel), h('span', { 'class': 'note', text: 'Solid triangle: plots. Outline: hidden. Thicker arrow: more likely next.' }));
      root.textContent = ''; root.append(box, top, read); sel.addEventListener('change', draw); draw();
    }).catch(function () { box.textContent = 'Could not load ' + url + '.'; });
  }

  /* ---------- flame block ---------- */
  function wFlame(root) {
    var url = root.getAttribute('data-src') || 'flames/vodoo.flame';
    var pre = h('pre', {}, h('code', { text: 'loading…' })), code = pre.firstChild, xml = '';
    var copy = h('button', { 'class': 'copy', type: 'button', text: 'copy', onclick: function () {
      if (navigator.clipboard) navigator.clipboard.writeText(xml).then(function () { copy.textContent = 'copied'; setTimeout(function () { copy.textContent = 'copy'; }, 1400); });
    } });
    pre.style.maxHeight = '420px'; pre.appendChild(copy);
    var dl = h('a', { 'class': 'btn', href: url, download: url.split('/').pop(), text: 'download .flame' });
    root.append(pre, h('p', {}, dl));
    fetch(url).then(function (r) { if (!r.ok) throw 0; return r.text(); }).then(function (t) { xml = t; code.textContent = t.replace(/^(\s{6}[0-9A-F]{48}\n)+/gm, function (m) { return '      …palette rows…\n'; }); })
      .catch(function () { code.textContent = 'Could not load ' + url + ' (opening the page from a file:// URL blocks fetch; serve it over http).'; });
  }

  /* ---------- live render (Fireheart embed) ---------- */
  function wViewer(root) {
    var url = root.getAttribute('data-src') || 'flames/vodoo.flame';
    var size = root.getAttribute('data-size') || '512', quality = root.getAttribute('data-quality') || '25';
    var plugins = root.getAttribute('data-plugins'); // Fireheart's opt-in variation pack, for flames that use starblur, mobius, hypertile2...
    var s = document.createElement('script'); s.type = 'module'; s.src = 'https://firehe.art/embed/fireheart-embed.js';
    document.head.appendChild(s);
    var fb = root.querySelector('img');
    // Credit and link to Fireheart on every live frame, even while it loads or if it fails.
    root.appendChild(h('a', { 'class': 'fh-link', href: 'https://firehe.art', target: '_blank', rel: 'noopener', title: 'Rendered with Fireheart' },
      h('img', { src: 'assets/fireheart.svg', alt: '', width: 16, height: 16 }), 'Fireheart'));
    Promise.all([fetch(url).then(function (r) { if (!r.ok) throw 0; return r.text(); }),
      Promise.race([customElements.whenDefined('fireheart-viewer'), new Promise(function (_, no) { setTimeout(no, 8000); })])])
      .then(function (a) {
        var v = document.createElement('fireheart-viewer'); v.setAttribute('size', size); v.setAttribute('quality', quality);
        if (plugins) v.setAttribute('plugins', plugins);
        var sc = document.createElement('script'); sc.type = 'text/x-flame'; sc.textContent = a[0];
        v.appendChild(sc); root.insertBefore(v, fb || null);
        if (fb) fb.style.display = 'none';
      }).catch(function () { /* embed unavailable: the still image stays */ });
  }

  var map = { split: wSplit, parts: wParts, weights: wWeights, tile: wTile, edges: wEdges, scales: wScales, grade: wGrade, disk: wDisk, packing: wPacking, flame: wFlame, viewer: wViewer };
  Array.prototype.forEach.call(document.querySelectorAll('[data-widget]'), function (el) {
    var f = map[el.getAttribute('data-widget')]; if (f) { try { f(el); } catch (e) { el.textContent = 'widget failed: ' + e.message; console.error(e); } }
  });
})();
