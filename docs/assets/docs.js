/* Figures for the reference page. Runs on truchet.js (the plugin's tiling code). */
(function () {
  'use strict';
  var T = window.Truchet, ROUND = T.TYPE.ROUND, ALL = T.TYPE.ALL;
  var INK = [226, 226, 222], GROUND = [16, 16, 16];
  function h(tag, attrs) { var e = document.createElement(tag), k, i, c; for (k in attrs || {}) { if (k === 'text') e.textContent = attrs[k]; else e.setAttribute(k, attrs[k]); } for (i = 2; i < arguments.length; i++) { c = arguments[i]; if (c) e.appendChild(typeof c === 'string' ? document.createTextNode(c) : c); } return e; }
  function fig(canvas, label) { return h('figure', {}, canvas, h('figcaption', { text: label })); }
  function paint(c, fn) { var g = c.getContext('2d'), im = g.createImageData(c.width, c.height), d = im.data, i = 0, x, y, col; for (y = 0; y < c.height; y++) for (x = 0; x < c.width; x++) { col = fn(x, y) ? INK : GROUND; d[i++] = col[0]; d[i++] = col[1]; d[i++] = col[2]; d[i++] = 255; } g.putImageData(im, 0, 0); }
  var BASE = { types: ROUND, seed: 5, levels: 3, split: 0.6, noise: 0.1, width: 6, height: 6 };
  function opts(o) { var r = {}, k; for (k in BASE) r[k] = BASE[k]; for (k in o) r[k] = o[k]; return r; }

  /* name -> list of {label, o: mosaic options, span, flip, scale} */
  var DEMOS = {
    seed: [1, 2, 3].map(function (s) { return { l: 'seed ' + s, o: { seed: s } }; }),
    size: [{ l: 'width 2', o: { width: 2, height: 2 }, span: 6 }, { l: 'width 6', o: {}, span: 6 }],
    levels: [0, 2, 4].map(function (v) { return { l: 'levels ' + v, o: { levels: v } }; }),
    split: [0.15, 0.5, 0.85].map(function (v) { return { l: 'split ' + v, o: { levels: 4, split: v, noise: 0 } }; }),
    noise: [0, 0.5, 2].map(function (v) { return { l: 'noise ' + v, o: { levels: 4, split: 0.15, noise: v } }; }),
    style: [{ l: 'style 0', o: { types: ALL, seed: 7, levels: 3 } }, { l: 'style 1', o: { types: ROUND, seed: 7, levels: 3 } }],
    scale: [{ l: 'scale 1', o: {}, span: 3, scale: 1 }, { l: 'scale 2', o: {}, span: 3, scale: 2 }],
    grade: [0.3, 0.6, 1.2].map(function (v) { return { l: 'grade ' + v, o: { levels: 6, grade: v }, span: 3, flip: true }; })
  };
  function renderDemo(el) {
    var list = DEMOS[el.getAttribute('data-demo')], px = 150;
    list.forEach(function (d) {
      var c = h('canvas', { width: px, height: px }), m = T.Mosaic(opts(d.o)), span = d.span || 3, sc = d.scale || 1;
      paint(c, function (x, y) { var fx = (x + 0.5) / px, fy = (y + 0.5) / px; return m.ink(fx * span * sc, (d.flip ? 1 - fy : fy) * span * sc * (d.flip ? 1 : 1)); });
      el.appendChild(fig(c, d.l));
    });
  }

  /* point-cloud figures for the modes */
  function rng(seed) { var s = seed >>> 0; return function () { s = (s + 0x6D2B79F5) >>> 0; var t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
  function renderMode(el) {
    var mode = +el.getAttribute('data-mode'), snap = +(el.getAttribute('data-snap') || 0.4), px = 220, N = 90000, u = rng(7 + mode);
    var c = h('canvas', { width: px, height: px }), g = c.getContext('2d'), m;
    g.fillStyle = '#101010'; g.fillRect(0, 0, px, px); g.fillStyle = 'rgba(226,226,222,.55)';
    var X = 3, Y = 3, S = 6; // incoming space is -3..3
    function plot(x, y) { g.fillRect((x + X) / S * px, (1 - (y + Y) / S) * px - 1, 1.3, 1.3); }
    if (mode === 3) {
      m = T.Mosaic({ types: ROUND, seed: 3, levels: 5, split: 0.6, noise: 0.1, grade: 0.5, width: 6, height: 6 });
      g.fillStyle = 'rgba(226,226,222,.6)';
      for (var i = 0; i < N * 2; i++) {
        var r = Math.sqrt(u()), t = u() * 6.2832, wr = r * Math.cos(t), wi = r * Math.sin(t), nr = 1 + wr, ni = wi, dr = 1 - wr, di = -wi, dd = dr * dr + di * di;
        if (dd < 1e-9) continue; var qr = (nr * dr + ni * di) / dd, qi = (ni * dr - nr * di) / dd;
        if (m.ink(-qi, qr)) g.fillRect((wr * 0.46 + 0.5) * px, (0.5 - wi * 0.46) * px, 1.2, 1.2);
      }
      g.strokeStyle = 'rgba(255,120,80,.8)'; g.lineWidth = 1.5; g.beginPath(); g.arc(px / 2, px / 2, 0.46 * px, 0, 6.2832); g.stroke();
    } else {
      m = T.Mosaic({ types: ROUND, seed: 5, levels: 3, split: 0.6, noise: 0.1, width: 6, height: 6 });
      for (var n = 0; n < N; n++) {
        if (mode === 0) { for (var k = 0; k < 24; k++) { var sx = u() * 6, sy = u() * 6; if (m.ink(sx, sy)) { plot(sx - 3, sy - 3); break; } } continue; }
        var rr = 2.2 * Math.sqrt(u()), tt = u() * 6.2832, x = rr * Math.cos(tt), y = rr * Math.sin(tt);
        if (m.ink(x + 3, y + 3)) { plot(x, y); continue; }
        if (mode === 2) for (var j = 0; j < 24; j++) { var jx = x + (u() * 2 - 1) * snap, jy = y + (u() * 2 - 1) * snap; if (m.ink(jx + 3, jy + 3)) { plot(jx, jy); break; } }
      }
      if (mode !== 0) { g.strokeStyle = 'rgba(255,120,80,.8)'; g.setLineDash([5, 5]); g.lineWidth = 1.5; g.beginPath(); g.arc(px / 2, px / 2, 2.2 / S * px, 0, 6.2832); g.stroke(); }
    }
    el.appendChild(fig(c, el.getAttribute('data-caption') || ''));
  }

  /* the bitmask calculator */
  var BITS = [['dl', 1, ['Dl']], ['dr', 2, ['Dr']], ['bars', 4, ['H', 'V']], ['dots', 8, ['Dots']], ['cross', 16, ['Cross']], ['clover', 32, ['Clover']], ['T', 64, ['Tn', 'Te', 'Ts', 'Tw']], ['fillet', 128, ['Fnw', 'Fne', 'Fsw', 'Fse']]];
  function tileThumb(name) { var c = h('canvas', { width: 56, height: 56 }); paint(c, function (x, y) { return T.tileInk('square', name, 0, (x + 0.5) / 56 - 0.5, 0.5 - (y + 0.5) / 56); }); return c; }
  function renderTypes(el) {
    var val = 3, boxes = [], out = h('output', { 'class': 'bigsum' }), prev = h('canvas', { width: 300, height: 300 }), eq = h('div', { 'class': 'eq' });
    var grid = h('div', { 'class': 'bitgrid' });
    BITS.forEach(function (b) {
      var cb = h('input', { type: 'checkbox', 'aria-label': b[0] }); if (b[1] & val) cb.checked = true; boxes.push(cb);
      var thumbs = h('span', { 'class': 'thumbs' }); b[2].forEach(function (n) { thumbs.appendChild(tileThumb(n)); });
      grid.appendChild(h('label', { 'class': 'bit' }, cb, h('b', { text: b[0] }), h('code', { text: String(b[1]) }), thumbs));
    });
    function draw() {
      var sum = 0, parts = []; BITS.forEach(function (b, i) { if (boxes[i].checked) { sum += b[1]; parts.push(b[1]); } });
      out.textContent = sum; eq.textContent = parts.length ? parts.join(' + ') + ' = ' + sum : 'nothing ticked: treated as 3 (the two arcs)';
      var m = T.Mosaic({ types: sum, seed: 9, levels: 3, split: 0.55, noise: 0.1, width: 6, height: 6 });
      paint(prev, function (x, y) { return m.ink(3 * (x + 0.5) / 300, 3 * (y + 0.5) / 300); });
    }
    boxes.forEach(function (b) { b.addEventListener('change', draw); });
    el.appendChild(h('div', { 'class': 'typesbox' }, grid, h('div', { 'class': 'typesout' }, h('div', { 'class': 'lbl', text: 'types =' }), out, eq, prev)));
    draw();
  }

  /* tile strips: data-tiles="square:Dl,Dr" */
  function renderTiles(el) {
    var spec = el.getAttribute('data-tiles').split(':'), set = spec[0];
    spec[1].split(',').forEach(function (n) { var c = h('canvas', { width: 64, height: 64, title: n }); paint(c, function (x, y) { return T.tileInk(set, n, 0, (x + 0.5) / 64 - 0.5, 0.5 - (y + 0.5) / 64); }); el.appendChild(c); });
  }

  function run(sel, fn) { Array.prototype.forEach.call(document.querySelectorAll(sel), function (el) {
    var done = false, go = function () { if (done) return; done = true; try { fn(el); } catch (e) { console.error(e); } };
    if ('IntersectionObserver' in window) { var io = new IntersectionObserver(function (es) { if (es[0].isIntersecting) { io.disconnect(); go(); } }, { rootMargin: '300px' }); io.observe(el); } else go(); }); }
  run('[data-demo]', renderDemo); run('[data-mode]', renderMode); run('[data-types]', renderTypes); run('[data-tiles]', renderTiles);
})();
