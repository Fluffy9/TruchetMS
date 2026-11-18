// usage: node check-js.js <dump-binary> <truchet.js>
const { execFileSync } = require('child_process');
const T = require(require('path').resolve(process.argv[3]));
const bin = process.argv[2];
// types seed levels split noise grade width x0 y0 span
const cfgs = [[3,1,4,0.5,0.45,0,6,0,0,4],[255,7,5,0.6,0.45,0,6,0,0,4],[256,1,4,0.5,0.45,0,6,0,0,4],
  [256,42,6,0.55,0.3,0,6,1,1,3],[256,3,7,0.5,0.45,0.18,6,0,0.01,3.9],[25,9,5,0.5,0.45,0.18,6,0.2,0.01,3.9]];
let bad = 0;
for (const c of cfgs) {
  const ref = execFileSync(bin, c.map(String), { maxBuffer: 1e8 }).toString().trim().split('\n');
  const m = T.Mosaic({ types: c[0], seed: c[1], levels: c[2], split: c[3], noise: c[4], grade: c[5], width: c[6], height: c[6] });
  let diff = 0; const n = 256;
  for (let j = 0; j < n; j++) for (let i = 0; i < n; i++) {
    const x = c[7] + (i + .5) / n * c[9], y = c[8] + (j + .5) / n * c[9];
    if ((m.ink(x, y) ? '1' : '0') !== ref[j][i]) diff++;
  }
  console.log(c.join(','), diff === 0 ? 'ok' : 'DIFF ' + diff); if (diff) bad++;
}
process.exit(bad ? 1 : 0);
