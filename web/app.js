// The musiex practice screen: endless score, audio, and a cursor that follows.
//
// The WebAssembly core (pkg/musiex_web.js) generates and engraves one system
// at a time and renders audio in blocks. This file schedules those blocks on
// the Web Audio clock, keeps a few systems ahead of the music, and moves the
// cursor using the same clock, so sound and picture stay together.

import init, { Practice } from './pkg/musiex_web.js';

const DEFAULTS = {
  meter: '4/4', key: 'C', clef: 'treble', tempo: 60, level: 2, low: '', high: '',
  leap: 3, rests: false, triplets: false, dotted: false, ties: false, sharps: false,
  flats: false, seed: 0, metronome: true, countIn: 1, zoom: 10,
};
const NUMBERS = ['tempo', 'level', 'leap', 'seed', 'countIn', 'zoom'];
const FLAGS = ['rests', 'triplets', 'dotted', 'ties', 'sharps', 'flats', 'metronome'];

const BLOCK = 2048; // samples per audio block
const AHEAD = 0.3; // seconds of audio scheduled ahead of the clock
const MIN_SYSTEMS = 3; // systems kept on screen from the current one, at least
// Staff spaces from the top of a system's drawing to the top staff line.
const STAFF_TOP = 5;

const $ = (id) => document.getElementById(id);
const form = $('settings');

let config = readHash();
let practice = null;
let ctx = null;
let nextTime = 0;
let timeline = []; // {time, tick, tps}: what tick each scheduled block starts at
let systems = []; // {el, data}
let current = -1;
let playing = false;
let started = false;
let pumpTimer = 0;
let lastNote = null;

function randomSeed() {
  return Math.floor(Math.random() * 1000000);
}

function readHash() {
  const c = { ...DEFAULTS };
  const params = new URLSearchParams(location.hash.slice(1));
  for (const [k, v] of params) {
    if (!(k in DEFAULTS)) continue;
    if (NUMBERS.includes(k)) c[k] = Number(v);
    else if (FLAGS.includes(k)) c[k] = v === '1';
    else c[k] = v;
  }
  if (!params.has('seed')) c.seed = randomSeed();
  return c;
}

function writeHash() {
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(config)) {
    if (v === DEFAULTS[k] && k !== 'seed') continue;
    params.set(k, typeof v === 'boolean' ? (v ? '1' : '0') : String(v));
  }
  history.replaceState(null, '', '#' + params.toString());
}

function writeForm() {
  for (const el of form.elements) {
    if (!el.name || !(el.name in config)) continue;
    if (el.type === 'checkbox') el.checked = config[el.name];
    else el.value = config[el.name];
  }
  $('tempo').value = config.tempo;
  $('tempo-value').textContent = `${config.tempo} bpm`;
}

function readForm() {
  for (const el of form.elements) {
    if (!el.name || !(el.name in config)) continue;
    if (el.type === 'checkbox') config[el.name] = el.checked;
    else if (NUMBERS.includes(el.name)) config[el.name] = Number(el.value);
    else config[el.name] = el.value.trim();
  }
}

function showError(message) {
  const el = $('error');
  el.textContent = message;
  el.hidden = !message;
}

// --- Session -------------------------------------------------------------

async function reset() {
  stopAudio();
  showError('');
  ctx = new AudioContext();
  try {
    practice = new Practice(JSON.stringify(config), ctx.sampleRate);
  } catch (e) {
    practice = null;
    showError(String(e.message || e));
    return;
  }
  writeHash();
  timeline = [];
  current = -1;
  started = false;
  lastNote = null;
  $('score').innerHTML = '';
  systems = [];
  fillScore();
  setCurrent(0);
  place(systems[0], 0, 0);
  updatePlayButton();
  $('where').textContent = 'Press Play to start.';
}

function stopAudio() {
  playing = false;
  clearInterval(pumpTimer);
  if (ctx) ctx.close();
  ctx = null;
}

function scoreWidth() {
  const score = $('score');
  const style = getComputedStyle(score);
  return score.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
}

/** Engraves systems until the score area is full (and at least MIN_SYSTEMS). */
function fillScore() {
  const room = $('score').clientHeight;
  const height = () => systems.reduce((h, s) => h + s.data.height, 0);
  while (systems.length < MIN_SYSTEMS || height() < room) addSystem();
}

function addSystem() {
  const data = JSON.parse(practice.nextSystem(scoreWidth(), config.zoom));
  const el = document.createElement('div');
  el.className = 'system';
  el.innerHTML = data.svg;
  const cursor = document.createElement('div');
  cursor.className = 'cursor';
  cursor.style.top = `${(STAFF_TOP - 1) * config.zoom}px`;
  cursor.style.height = `${6 * config.zoom}px`;
  el.appendChild(cursor);
  $('score').appendChild(el);
  systems.push({ el, data, cursor });
}

function setCurrent(index) {
  if (index === current) return;
  if (systems[current]) systems[current].el.classList.remove('current');
  current = index;
  systems[current].el.classList.add('current');
}

// --- Audio ---------------------------------------------------------------

function pump() {
  if (!ctx || !practice) return;
  while (nextTime < ctx.currentTime + AHEAD) {
    const left = new Float32Array(BLOCK);
    const right = new Float32Array(BLOCK);
    const tick = practice.position();
    const tps = practice.ticksPerSecond();
    practice.render(left, right);
    const buffer = ctx.createBuffer(2, BLOCK, ctx.sampleRate);
    buffer.copyToChannel(left, 0);
    buffer.copyToChannel(right, 1);
    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.connect(ctx.destination);
    src.start(nextTime);
    timeline.push({ time: nextTime, tick, tps });
    nextTime += BLOCK / ctx.sampleRate;
  }
  // Only the blocks still playing are needed to find the position.
  while (timeline.length > 2 && timeline[1].time <= ctx.currentTime) timeline.shift();
}

function currentTick() {
  if (!timeline.length) return null;
  const t = ctx.currentTime;
  let e = timeline[0];
  for (const x of timeline) if (x.time <= t) e = x;
  return e.tick + Math.max(0, t - e.time) * e.tps;
}

async function togglePlay() {
  if (!practice) return;
  if (!started) {
    started = true;
    await ctx.resume();
    nextTime = ctx.currentTime + 0.08;
    pump();
    pumpTimer = setInterval(pump, 25);
    playing = true;
    requestAnimationFrame(frame);
  } else if (playing) {
    playing = false;
    await ctx.suspend();
  } else {
    await ctx.resume();
    playing = true;
    requestAnimationFrame(frame);
  }
  updatePlayButton();
}

function updatePlayButton() {
  $('play').textContent = playing ? 'Pause' : started ? 'Resume' : 'Play';
}

// --- Following the music -------------------------------------------------

function frame() {
  if (!playing || !ctx) return;
  const tick = currentTick();
  if (tick !== null) follow(tick);
  requestAnimationFrame(frame);
}

function follow(tick) {
  if (tick < 0) {
    const beat = practice.beatTicks();
    const left = Math.ceil(-tick / beat);
    $('where').textContent = `Count-in: ${left}`;
    place(systems[current], 0, 0);
    return;
  }
  // Move on to the system that holds this tick.
  let index = current;
  while (systems[index + 1] && tick >= systems[index].data.endTick) index++;
  if (index !== current) {
    turnTo(index);
    index = current;
  }
  const sys = systems[index];
  const notes = sys.data.notes;
  let i = 0;
  while (i + 1 < notes.length && notes[i + 1].start <= tick) i++;
  const n = notes[i];
  const frac = Math.min(1, Math.max(0, (tick - n.start) / n.dur));
  place(sys, i, frac);
  const bar = Math.floor(tick / practice.barTicks()) + 1;
  $('where').textContent = `Bar ${bar} · seed ${config.seed}`;
}

/** Puts the cursor `frac` of the way from note `i` to the next one, and
 *  highlights note `i`. */
function place(sys, i, frac) {
  const notes = sys.data.notes;
  const x0 = notes[i].x;
  const x1 = i + 1 < notes.length ? notes[i + 1].x : sys.data.endX;
  sys.cursor.style.left = `${x0 + (x1 - x0) * frac}px`;
  const g = sys.el.querySelector(`g[data-note="${i}"]`);
  if (g !== lastNote) {
    if (lastNote) lastNote.classList.remove('playing');
    if (g && !notes[i].rest) g.classList.add('playing');
    lastNote = g;
  }
}

/** Scrolls finished systems away and engraves new ones at the bottom. */
function turnTo(index) {
  if (lastNote) lastNote.classList.remove('playing');
  lastNote = null;
  const gone = systems.splice(0, index);
  for (const s of gone) {
    s.el.classList.add('leaving');
    s.el.style.marginTop = `-${s.el.offsetHeight}px`;
    setTimeout(() => s.el.remove(), 400);
  }
  current = -1;
  setCurrent(0);
  fillScore();
}

// --- Controls ------------------------------------------------------------

$('play').addEventListener('click', togglePlay);
$('restart').addEventListener('click', reset);
$('new').addEventListener('click', () => {
  config.seed = randomSeed();
  writeForm();
  reset();
});
$('tempo').addEventListener('input', (e) => {
  config.tempo = Number(e.target.value);
  $('tempo-value').textContent = `${config.tempo} bpm`;
  if (practice) practice.setTempo(config.tempo);
  writeHash();
});
form.addEventListener('change', () => {
  if (!form.checkValidity()) return;
  readForm();
  reset();
});
form.addEventListener('submit', (e) => e.preventDefault());
$('settings-toggle').addEventListener('click', () => {
  form.hidden = !form.hidden;
  $('settings-toggle').setAttribute('aria-expanded', String(!form.hidden));
});
$('share').addEventListener('click', async () => {
  writeHash();
  try {
    await navigator.clipboard.writeText(location.href);
    $('share').textContent = 'Link copied';
  } catch {
    $('share').textContent = 'Copy the address bar to share';
  }
  setTimeout(() => ($('share').textContent = 'Copy link to this exercise'), 2000);
});
document.addEventListener('keydown', (e) => {
  if (e.target.closest('input, select, textarea')) return;
  if (e.code === 'Space') {
    e.preventDefault();
    togglePlay();
  } else if (e.key === 'n' || e.key === 'N') {
    $('new').click();
  }
});
let resizeTimer = 0;
window.addEventListener('resize', () => {
  // Re-flow the score for the new width, but never interrupt playback.
  if (started) return;
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(reset, 200);
});

await init();
writeForm();
reset();
