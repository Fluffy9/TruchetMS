/* truchet.js — a line-for-line port of the truchet-tiles crate's Mosaic::ink.
   Same hash, same quadtree, same tiles, so a seed here is a seed in the plugin.
   Checked against the Rust crate (scripts/check-js.sh). */
(function (root) {
  'use strict';
  var F = Math.fround;
  var TYPE = { DL: 1, DR: 2, BAR: 4, DOTS: 8, CROSS: 16, CLOVER: 32, T: 64, FILLET: 128, ALL: 255, ROUND: 256 };
  var BAND = F(1 / 6), RADIUS = 0.5, FILLET = F(1 / 3), PORT = F(1 / 12), GROUND = F(1 / 6);
  var hypot = Math.hypot;

  function imul(a, b) { return Math.imul(a, b) >>> 0; }
  function mix(seed, x, y, level, salt) {
    var n = imul(seed >>> 0, 0x9E3779B1);
    n = (n + (x >>> 0)) >>> 0; n = imul(n, 0x85EBCA6B);
    n = (n + (y >>> 0)) >>> 0; n = imul(n, 0xC2B2AE35);
    n = (n + (level >>> 0)) >>> 0; n = imul(n, 0x27D4EB2F);
    n = (n + salt) >>> 0;
    n = (n ^ (n >>> 16)) >>> 0; n = imul(n, 0x7FEB352D);
    n = (n ^ (n >>> 15)) >>> 0; n = imul(n, 0x846CA68B);
    return (n ^ (n >>> 16)) >>> 0;
  }
  function unit(h) { return F(F(h + 0.5) / 4294967296); }
  function rround(v) { return v < 0 ? -Math.round(-v) : Math.round(v); } // half away from zero, like Rust
  function quant(v) { return rround(F(v * 4096)) | 0; }
  function gaussian(seed, x, y, level, salt) {
    var u = Math.min(Math.max(unit(mix(seed, x, y, level, salt)), 1e-4), 1 - 1e-4);
    var v = unit(mix(seed, x, y, level, (salt + 17) >>> 0));
    return F(Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v));
  }
  function wrap(v, span) {
    if (!isFinite(v) || span <= 0) return 0;
    var m = v % span; if (m < 0) m += span;
    return m === span ? 0 : m;
  }
  function pow2(level) { return Math.pow(0.5, level); }
  function clamp(v, a, b) { return Math.min(Math.max(v, a), b); }

  /* ---------- Carlson's fifteen ---------- */
  var CARLSON = ['Dl','Dr','H','V','Dots','Cross','Clover','Tn','Te','Ts','Tw','Fnw','Fne','Fsw','Fse'];
  var CARLSON_BIT = { Dl:1, Dr:2, H:4, V:4, Dots:8, Cross:16, Clover:32, Tn:64, Te:64, Ts:64, Tw:64, Fnw:128, Fne:128, Fsw:128, Fse:128 };
  function carlsonInk(t, u, v) {
    if (Math.abs(u) > 0.5 || Math.abs(v) > 0.5) return false;
    if (hypot(u, v - 0.5) < BAND || hypot(u, v + 0.5) < BAND || hypot(u - 0.5, v) < BAND || hypot(u + 0.5, v) < BAND) return true;
    function arc(cx, cy) { return Math.abs(hypot(u - cx, v - cy) - RADIUS) < BAND; }
    function disc(cx, cy) { return hypot(u - cx, v - cy) < FILLET; }
    switch (t) {
      case 'Dl': return arc(-0.5, -0.5) || arc(0.5, 0.5);
      case 'Dr': return arc(-0.5, 0.5) || arc(0.5, -0.5);
      case 'H': return Math.abs(v) < BAND;
      case 'V': return Math.abs(u) < BAND;
      case 'Dots': return false;
      case 'Cross': return Math.abs(u) < BAND || Math.abs(v) < BAND;
      case 'Clover': return !(disc(-0.5, -0.5) || disc(0.5, -0.5) || disc(-0.5, 0.5) || disc(0.5, 0.5));
      case 'Tn': return v > -BAND && !(disc(-0.5, 0.5) || disc(0.5, 0.5));
      case 'Te': return u > -BAND && !(disc(0.5, 0.5) || disc(0.5, -0.5));
      case 'Ts': return v < BAND && !(disc(-0.5, -0.5) || disc(0.5, -0.5));
      case 'Tw': return u < BAND && !(disc(-0.5, 0.5) || disc(-0.5, -0.5));
      case 'Fnw': return arc(-0.5, 0.5);
      case 'Fne': return arc(0.5, 0.5);
      case 'Fsw': return arc(-0.5, -0.5);
      case 'Fse': return arc(0.5, -0.5);
    }
    return false;
  }

  /* ---------- the round family ---------- */
  var Z = { small: 0, big: 0, semi: 0, hbar: 0, vbar: 0, fill: 'none', hole: false, centre: false, dotted: false, turns: 4 };
  function S(o) { var r = {}, k; for (k in Z) r[k] = Z[k]; for (k in o) r[k] = o[k]; return r; }
  var ROUND = [
    ['Empty', S({ turns: 1 })],
    ['Empty1', S({ centre: true, turns: 1 })],
    ['Dotted', S({ dotted: true, turns: 1 })],
    ['CornerHash', S({ small: 15, turns: 1 })],
    ['SadFace', S({ small: 3, semi: 4 })],
    ['HourGlass', S({ semi: 5, turns: 2 })],
    ['EdgeHash', S({ semi: 15, turns: 1 })],
    ['Edge34', S({ semi: 7 })],
    ['Frog', S({ small: 3, hbar: 2 })],
    ['Slash2', S({ small: 1, big: 1 })],
    ['Slash21', S({ small: 5, big: 1 })],
    ['Slash11', S({ small: 4, big: 1 })],
    ['CrossCross', S({ small: 1, hbar: 2, vbar: 2 })],
    ['Filled', S({ fill: 'full', turns: 1 })],
    ['FilledHollow', S({ fill: 'full', hole: true, turns: 1 })],
    ['Filled12', S({ fill: 'band', turns: 2 })],
    ['Filled12Hollow', S({ fill: 'band', hole: true, turns: 2 })],
    ['Filled34', S({ fill: 'upper' })],
    ['Filled34Hollow', S({ fill: 'upper', hole: true })],
    ['Filled13', S({ fill: 'strip' })],
    ['Filled13Bar', S({ fill: 'strip', hbar: 2 })],
    ['Cowboy', S({ fill: 'quarter' })]
  ];
  var CORNERS = [[0, 0], [1, 0], [1, 1], [0, 1]], MIDS = [[0.5, 0], [1, 0.5], [0.5, 1], [0, 0.5]];
  function roundInk(s, u, v, rot) {
    for (var r = 0; r < rot % 4; r++) { var t = u; u = v; v = -t; }
    var x = u + 0.5, y = 0.5 - v;
    if (!(x >= 0 && x <= 1) || !(y >= 0 && y <= 1)) return false;
    function d(cx, cy) { return hypot(x - cx, y - cy); }
    var ks = [0.25, 0.75], i;
    for (i = 0; i < 2; i++) { var k = ks[i]; if (d(k, 0) < PORT || d(k, 1) < PORT || d(0, k) < PORT || d(1, k) < PORT) return true; }
    if (s.centre && d(0.5, 0.5) < PORT) return true;
    if (s.dotted && (d(0.5, 0.25) < PORT || d(0.25, 0.5) < PORT || d(0.75, 0.5) < PORT || d(0.5, 0.75) < PORT)) return true;
    for (i = 0; i < 4; i++) {
      var c = CORNERS[i], m = MIDS[i];
      if ((s.small >> i & 1 && Math.abs(d(c[0], c[1]) - 0.25) < PORT) ||
          (s.big >> i & 1 && Math.abs(d(c[0], c[1]) - 0.75) < PORT) ||
          (s.semi >> i & 1 && Math.abs(d(m[0], m[1]) - 0.25) < PORT)) return true;
    }
    if ((s.hbar & 1 && Math.abs(y - 0.25) < PORT) || (s.hbar & 2 && Math.abs(y - 0.75) < PORT) ||
        (s.vbar & 1 && Math.abs(x - 0.25) < PORT) || (s.vbar & 2 && Math.abs(x - 0.75) < PORT)) return true;
    var inside = false;
    switch (s.fill) {
      case 'full': inside = true; break;
      case 'band': inside = y >= 1 / 6 && y <= 5 / 6; break;
      case 'upper': inside = y <= 5 / 6; break;
      case 'strip': inside = y <= 1 / 6; break;
      case 'quarter': inside = d(0, 0) <= 5 / 6; break;
    }
    if (!inside) return false;
    for (i = 0; i < 4; i++) { if (d(CORNERS[i][0], CORNERS[i][1]) < GROUND || d(MIDS[i][0], MIDS[i][1]) < GROUND) return false; }
    return !(s.hole && d(0.5, 0.5) < GROUND);
  }

  /* ---------- the mosaic ---------- */
  function Mosaic(o) {
    o = o || {};
    var m = {
      seed: (o.seed == null ? 1 : o.seed) >>> 0,
      width: clamp(o.width == null ? 6 : o.width, 1, 64),
      height: clamp(o.height == null ? 6 : o.height, 1, 64),
      levels: clamp(Math.trunc(o.levels == null ? 4 : o.levels), 0, 8),
      split: clamp(o.split == null ? 0.5 : o.split, 0, 1),
      noise: Math.max(o.noise == null ? 0.45 : o.noise, 0),
      grade: isFinite(o.grade) ? clamp(o.grade, 0, 64) : 0,
      types: o.types == null ? 3 : o.types
    };
    m.types &= (TYPE.ALL | TYPE.ROUND);
    if (m.types & TYPE.ROUND) m.types = TYPE.ROUND;
    if (!m.types) m.types = 3;
    m.round = !!(m.types & TYPE.ROUND);
    var kinds = [];
    if (m.round) {
      ROUND.forEach(function (e) { for (var r = 0; r < e[1].turns; r++) kinds.push({ spec: e[1], rot: r }); });
    } else {
      CARLSON.forEach(function (n) { if (m.types & CARLSON_BIT[n]) kinds.push({ name: n }); });
    }
    m.kinds = kinds;
    m.fold = function (x, y) {
      y = m.grade > 0 ? (isFinite(y) ? y : 0) : wrap(y, m.height);
      return [wrap(x, m.width), y];
    };
    m.splits = function (cx, cy, level) {
      if (level >= m.levels) return false;
      var roll = unit(mix(m.seed, quant(cx), quant(cy), level, 1));
      if (m.grade > 0) {
        var target = F(F(m.grade * Math.abs(cy)) * F(1 + F(F(roll - 0.5) * m.noise)));
        return pow2(level) > target;
      }
      var nudged = m.noise === 0 ? roll : clamp(F(roll + F(gaussian(m.seed, quant(cx), quant(cy), level, 3) * m.noise)), 0, 1);
      return nudged < m.split;
    };
    m.leaf = function (px, py) {
      var cx = Math.floor(px) + 0.5, cy = Math.floor(py) + 0.5, level = 0;
      while (level < m.levels) {
        if (!m.splits(cx, cy, level)) break;
        var step = pow2(level) * 0.25;
        cx += px < cx ? -step : step; cy += py < cy ? -step : step; level++;
      }
      return [cx, cy, level];
    };
    m.isLeaf = function (x, y, level) { var f = m.fold(x, y); return m.leaf(f[0], f[1])[2] === level; };
    m.tileAt = function (cx, cy, level) {
      return kinds[mix(m.seed, quant(cx), quant(cy), level, 2) % kinds.length];
    };
    m.wing = function (px, py, level, u, v) {
      if (level >= m.levels) return null;
      var k, s, ground, edge = 0.5 - Math.max(Math.abs(u), Math.abs(v));
      if (edge >= 1 / 6) return null;
      for (k = m.levels; k > level; k--) {
        s = pow2(k); ground = k % 2 === 1;
        var gx = Math.round(px / s) * s, gy = Math.round(py / s) * s, h = s * 0.5;
        if (hypot(px - gx, py - gy) < s / 3) {
          if (m.isLeaf(gx - h, gy - h, k) || m.isLeaf(gx + h, gy - h, k) || m.isLeaf(gx - h, gy + h, k) || m.isLeaf(gx + h, gy + h, k)) return ground;
        }
        var hx = (Math.round(px / s - 0.5) + 0.5) * s;
        if (hypot(px - hx, py - gy) < s / 6 && (m.isLeaf(hx, gy - h, k) || m.isLeaf(hx, gy + h, k))) return !ground;
        var vy = (Math.round(py / s - 0.5) + 0.5) * s;
        if (hypot(px - gx, py - vy) < s / 6 && (m.isLeaf(gx - h, vy, k) || m.isLeaf(gx + h, vy, k))) return !ground;
      }
      return null;
    };
    m.wingRound = function (px, py, level, u, v) {
      if (level >= m.levels) return null;
      if (0.5 - Math.max(Math.abs(u), Math.abs(v)) >= 1 / 12) return null;
      for (var k = m.levels; k > level; k--) {
        var s = pow2(k), odd = k % 2 === 1, h = s * 0.5;
        var ix = rround(px / h), iy = rround(py / h), gx = ix * h, gy = iy * h;
        if (hypot(px - gx, py - gy) < s / 6) {
          var ox = ((ix % 2) + 2) % 2, oy = ((iy % 2) + 2) % 2, t = null;
          if (ox === 0 && oy === 0) t = [[-1, -1], [1, -1], [-1, 1], [1, 1]];
          else if (ox === 1 && oy === 0) t = [[0, -1], [0, 1]];
          else if (ox === 0 && oy === 1) t = [[-1, 0], [1, 0]];
          if (t) for (var a = 0; a < t.length; a++) if (m.isLeaf(gx + t[a][0] * h, gy + t[a][1] * h, k)) return odd;
        }
        var q = s * 0.25, jx = rround(px / q), jy = rround(py / q), dx = jx * q, dy = jy * q;
        if (hypot(px - dx, py - dy) < s / 12) {
          var mx = ((jx % 4) + 4) % 4, my = ((jy % 4) + 4) % 4, t2 = null;
          if (my === 0 && (mx === 1 || mx === 3)) t2 = [[0, -1], [0, 1]];
          else if (mx === 0 && (my === 1 || my === 3)) t2 = [[-1, 0], [1, 0]];
          if (t2) for (var b = 0; b < t2.length; b++) if (m.isLeaf(dx + t2[b][0] * h, dy + t2[b][1] * h, k)) return !odd;
        }
      }
      return null;
    };
    m.ink = function (x, y) {
      if (m.grade > 0 && !(y > 0)) return false;
      var f = m.fold(x, y), px = f[0], py = f[1];
      var l = m.leaf(px, py), cx = l[0], cy = l[1], level = l[2];
      var scale = pow2(level), u = (px - cx) / scale, v = (py - cy) / scale;
      var w = m.round ? m.wingRound(px, py, level, u, v) : m.wing(px, py, level, u, v);
      if (w !== null) return w;
      var k = m.tileAt(cx, cy, level);
      var band = m.round ? roundInk(k.spec, u, v, k.rot) : carlsonInk(k.name, u, v);
      return level % 2 === 1 ? !band : band;
    };
    return m;
  }

  /* One tile, for the tile browser. name: Carlson name or Round name. */
  function tileInk(set, name, rot, u, v) {
    if (set === 'round') {
      for (var i = 0; i < ROUND.length; i++) if (ROUND[i][0] === name) return roundInk(ROUND[i][1], u, v, rot);
      return false;
    }
    for (var r = 0; r < rot % 4; r++) { var t = u; u = v; v = -t; }
    return carlsonInk(name, u, v);
  }

  /* Paint a mosaic into a canvas. Fills [x0,x0+span]x[y0,y0+span] (y down). */
  function paint(canvas, mosaic, o) {
    o = o || {};
    var n = canvas.width, h = canvas.height, ctx = canvas.getContext('2d'), img = ctx.createImageData(n, h);
    var x0 = o.x0 || 0, y0 = o.y0 || 0, span = o.span || 4;
    var ink = o.ink || [236, 232, 220], gr = o.ground || [22, 26, 38], d = img.data, i = 0;
    for (var j = 0; j < h; j++) for (var k = 0; k < n; k++) {
      var on = mosaic.ink(x0 + (k + 0.5) / n * span, y0 + (j + 0.5) / h * span);
      var c = on ? ink : gr; d[i++] = c[0]; d[i++] = c[1]; d[i++] = c[2]; d[i++] = 255;
    }
    ctx.putImageData(img, 0, 0);
  }

  var api = { TYPE: TYPE, Mosaic: Mosaic, tileInk: tileInk, paint: paint, CARLSON: CARLSON, ROUND: ROUND.map(function (e) { return { name: e[0], turns: e[1].turns }; }), mix: mix, quant: quant };
  if (typeof module !== 'undefined' && module.exports) module.exports = api; else root.Truchet = api;
})(typeof self !== 'undefined' ? self : this);
