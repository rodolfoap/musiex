// Parses ABC files with abcjs and checks that it reads them without warnings
// and that every bar adds up to the time signature.
//
// Usage: npm install --no-save abcjs@6.4.4 && node tools/check-abc.js FILE...
const abcjs = require('abcjs');
const fs = require('fs');

let bad = 0;
const files = process.argv.slice(2);
for (const f of files) {
  const tune = abcjs.parseOnly(fs.readFileSync(f, 'utf8'))[0];
  if (tune.warnings && tune.warnings.length) {
    bad++;
    console.log(f, tune.warnings.slice(0, 3));
    continue;
  }
  const meter = tune.getMeterFraction();
  const want = meter.num / meter.den;
  let dur = 0, mult = 1, bars = 0, wrong = 0;
  for (const line of tune.lines) {
    for (const staff of line.staff || []) {
      for (const voice of staff.voices) {
        for (const el of voice) {
          if (el.el_type === 'note') {
            // abcjs marks the triplet ratio on the first note only.
            if (el.startTriplet) mult = el.tripletMultiplier;
            dur += el.duration * mult;
            if (el.endTriplet) mult = 1;
          } else if (el.el_type === 'bar') {
            bars++;
            if (Math.abs(dur - want) > 1e-9) wrong++;
            dur = 0;
          }
        }
      }
    }
  }
  if (wrong) {
    bad++;
    console.log(f, `${wrong} of ${bars} bars have the wrong length`);
  }
}
console.log(`${files.length} files, ${bad} with problems`);
process.exit(bad ? 1 : 0);
