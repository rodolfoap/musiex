// Smoke test for the practice screen: loads it in headless Chromium, presses
// Play and checks that the music advances, systems turn and nothing throws.
//
// Usage: node tools/web-smoke.js http://localhost:8000/
const { chromium } = require('playwright');

(async () => {
  const url = process.argv[2] + '#seed=11&tempo=240&level=4&triplets=1&ties=1&sharps=1';
  const browser = await chromium.launch({ args: ['--autoplay-policy=no-user-gesture-required'] });
  const page = await browser.newPage({ viewport: { width: 900, height: 700 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await page.goto(url);
  await page.waitForSelector('.system svg');
  await page.click('#play');
  const bar = async () => {
    const text = await page.textContent('#where');
    const m = /Bar (\d+)/.exec(text);
    return m ? Number(m[1]) : 0;
  };
  await page.waitForTimeout(4000);
  const first = await bar();
  await page.waitForTimeout(6000);
  const later = await bar();
  const highlighted = await page.locator('.note.playing').count();
  await browser.close();
  const problems = [...errors];
  if (first < 1) problems.push(`playback did not start (status bar ${first})`);
  if (later < first + 4) problems.push(`playback did not advance enough (bar ${first} then ${later})`);
  if (highlighted > 1) problems.push(`${highlighted} notes highlighted at once`);
  if (problems.length) {
    console.error(problems.join('\n'));
    process.exit(1);
  }
  console.log(`ok: bar ${first} after 4 s, bar ${later} after 10 s`);
})();
