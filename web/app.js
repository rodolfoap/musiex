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
  flats: false, instrument: 'concert', seed: 0, metronome: true, countIn: 1, zoom: 10,
  names: 'off', keyboard: true, speedUp: 0,
};
const NUMBERS = ['tempo', 'level', 'leap', 'seed', 'countIn', 'zoom', 'speedUp'];
const FLAGS = ['rests', 'triplets', 'dotted', 'ties', 'sharps', 'flats', 'metronome', 'keyboard'];
// Settings that only change what is shown, so the exercise keeps playing.
const VIEW = ['names', 'keyboard', 'speedUp'];
const SAVED = 'musiex-settings';

// One tap sets the difficulty; meter, key and clef stay as chosen.
const PRESETS = {
  beginner: { level: 1, leap: 2, rests: false, triplets: false, dotted: false, ties: false, sharps: false, flats: false, tempo: 60 },
  easy: { level: 2, leap: 2, rests: true, triplets: false, dotted: false, ties: false, sharps: false, flats: false, tempo: 70 },
  medium: { level: 3, leap: 3, rests: true, triplets: false, dotted: true, ties: true, sharps: false, flats: false, tempo: 80 },
  hard: { level: 4, leap: 5, rests: true, triplets: true, dotted: true, ties: true, sharps: true, flats: true, tempo: 90 },
};
// The speed trainer raises the tempo every this many bars.
const SPEED_UP_BARS = 4;
const MAX_TEMPO = 240;

const LETTERS = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
const SOLFEGE = ['Do', 'Re', 'Mi', 'Fa', 'Sol', 'La', 'Si'];
const WHITE_SEMITONES = [0, 2, 4, 5, 7, 9, 11];

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
let lastName = null;
let lastKey = null;
let lastBar = 0;
let wakeLock = null;

function randomSeed() {
  return Math.floor(Math.random() * 1000000);
}

function readHash() {
  const c = { ...DEFAULTS };
  const params = new URLSearchParams(location.hash.slice(1));
  // Opened without settings in the link (as the installed app is): use the
  // ones from last time.
  if (![...params.keys()].length) {
    try {
      const saved = JSON.parse(localStorage.getItem(SAVED) || '{}');
      for (const k of Object.keys(DEFAULTS)) {
        if (k !== 'seed' && typeof saved[k] === typeof DEFAULTS[k]) c[k] = saved[k];
      }
    } catch {
      // No storage (private window): start from the defaults.
    }
  }
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
  try {
    localStorage.setItem(SAVED, JSON.stringify(config));
  } catch {
    // Not saved; the link still carries the settings.
  }
}

function writeForm() {
  for (const el of form.elements) {
    if (!el.name || !(el.name in config)) continue;
    if (el.type === 'checkbox') el.checked = config[el.name];
    else el.value = config[el.name];
  }
  $('tempo').value = config.tempo;
  showTempo();
}

/** Shows the tempo next to the slider and fills the slider's track up to it. */
function showTempo() {
  const slider = $('tempo');
  const pct = ((config.tempo - slider.min) / (slider.max - slider.min)) * 100;
  slider.style.setProperty('--pct', `${pct}%`);
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
  lastName = null;
  lastKey = null;
  lastBar = 0;
  buildKeyboard();
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
  keepAwake(false);
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
  // Note names, shown below the staff when switched on.
  const names = data.notes.map((n) => {
    if (n.letter === null) return null;
    const span = document.createElement('span');
    span.className = 'name';
    span.dataset.letter = n.letter;
    span.dataset.alter = n.alter;
    span.textContent = noteName(n.letter, n.alter);
    span.style.left = `${n.x + 0.6 * config.zoom}px`;
    span.style.top = `${(STAFF_TOP + 7.4) * config.zoom}px`;
    span.style.fontSize = `${Math.max(11, 1.3 * config.zoom)}px`;
    el.appendChild(span);
    return span;
  });
  $('score').appendChild(el);
  systems.push({ el, data, cursor, names });
}

function noteName(letter, alter) {
  const base = (config.names === 'solfege' ? SOLFEGE : LETTERS)[letter];
  return base + (alter > 0 ? '♯' : alter < 0 ? '♭' : '');
}

/** Applies the settings that change only what is shown. */
function applyView() {
  $('score').classList.toggle('show-names', config.names !== 'off');
  for (const span of $('score').querySelectorAll('.name')) {
    span.textContent = noteName(Number(span.dataset.letter), Number(span.dataset.alter));
  }
  $('keyboard').hidden = !config.keyboard;
  for (const key of $('keyboard').querySelectorAll('.white')) {
    key.querySelector('span').textContent = noteName(Number(key.dataset.letter), 0);
  }
}

// --- Piano keyboard ------------------------------------------------------

/** Draws whole octaves of keys covering the exercise's range. */
function buildKeyboard() {
  const kb = $('keyboard');
  kb.innerHTML = '';
  const [low, high] = practice.keyRange();
  const first = Math.floor(low / 12) * 12;
  const last = Math.ceil((high + 1) / 12) * 12 - 1;
  const whites = [];
  for (let k = first; k <= last; k++) {
    const letter = WHITE_SEMITONES.indexOf(k % 12);
    if (letter >= 0) whites.push({ k, letter });
  }
  const width = 100 / whites.length;
  whites.forEach(({ k, letter }, i) => {
    const key = document.createElement('div');
    key.className = 'key white';
    key.dataset.key = k;
    key.dataset.letter = letter;
    key.style.left = `${i * width}%`;
    key.style.width = `${width}%`;
    const label = document.createElement('span');
    label.textContent = noteName(letter, 0);
    key.appendChild(label);
    if (letter === 0) key.classList.add('octave');
    kb.appendChild(key);
    // A black key after C, D, F, G and A.
    if ([0, 1, 3, 4, 5].includes(letter) && k + 1 <= last) {
      const black = document.createElement('div');
      black.className = 'key black';
      black.dataset.key = k + 1;
      black.style.left = `${(i + 1) * width - width * 0.3}%`;
      black.style.width = `${width * 0.6}%`;
      kb.appendChild(black);
    }
  });
}

function lightKey(midi) {
  const key = midi === null ? null : $('keyboard').querySelector(`[data-key="${midi}"]`);
  if (key === lastKey) return;
  if (lastKey) lastKey.classList.remove('lit');
  if (key) key.classList.add('lit');
  lastKey = key;
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
  keepAwake(playing);
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
  if (bar !== lastBar) {
    lastBar = bar;
    if (config.speedUp && bar > 1 && (bar - 1) % SPEED_UP_BARS === 0) speedUp();
  }
  $('where').textContent = `Bar ${bar} · seed ${config.seed}`;
}

/** The speed trainer: a little faster every few bars. */
function speedUp() {
  if (config.tempo >= MAX_TEMPO) return;
  config.tempo = Math.min(MAX_TEMPO, config.tempo + config.speedUp);
  $('tempo').value = config.tempo;
  showTempo();
  practice.setTempo(config.tempo);
  writeHash();
}

/** Keeps the screen on while the music plays, where the device allows it. */
async function keepAwake(on) {
  try {
    if (on && !wakeLock && 'wakeLock' in navigator) {
      wakeLock = await navigator.wakeLock.request('screen');
      wakeLock.addEventListener('release', () => (wakeLock = null));
    } else if (!on && wakeLock) {
      await wakeLock.release();
      wakeLock = null;
    }
  } catch {
    // Refused (battery saver, hidden page): the screen just follows its own timeout.
  }
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
  const name = sys.names[i];
  if (name !== lastName) {
    if (lastName) lastName.classList.remove('playing');
    if (name) name.classList.add('playing');
    lastName = name;
  }
  lightKey(notes[i].key);
}

/** Scrolls finished systems away and engraves new ones at the bottom. */
function turnTo(index) {
  if (lastNote) lastNote.classList.remove('playing');
  lastNote = null;
  if (lastName) lastName.classList.remove('playing');
  lastName = null;
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
  showTempo();
  if (practice) practice.setTempo(config.tempo);
  writeHash();
});
form.addEventListener('change', (e) => {
  if (!form.checkValidity()) return;
  readForm();
  if (VIEW.includes(e.target.name)) {
    applyView();
    writeHash();
    return;
  }
  reset();
});
for (const button of form.querySelectorAll('[data-preset]')) {
  button.addEventListener('click', () => {
    Object.assign(config, PRESETS[button.dataset.preset]);
    writeForm();
    reset();
  });
}
document.addEventListener('visibilitychange', () => {
  // The browser drops the wake lock when the page is hidden.
  if (document.visibilityState === 'visible' && playing) keepAwake(true);
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
// Installing on Android: Chrome offers the app once the offline cache is in
// place, and the page shows its own button for it.
let installPrompt = null;
window.addEventListener('beforeinstallprompt', (e) => {
  if (!/Android/i.test(navigator.userAgent)) return;
  e.preventDefault();
  installPrompt = e;
  $('install').hidden = false;
});
$('install').addEventListener('click', async () => {
  if (!installPrompt) return;
  installPrompt.prompt();
  await installPrompt.userChoice;
  installPrompt = null;
  $('install').hidden = true;
});
window.addEventListener('appinstalled', () => {
  installPrompt = null;
  $('install').hidden = true;
});
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('sw.js').catch(() => {});
}

let resizeTimer = 0;
window.addEventListener('resize', () => {
  // Re-flow the score for the new width, but never interrupt playback.
  if (started) return;
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(reset, 200);
});

await init();
writeForm();
applyView();
reset();
