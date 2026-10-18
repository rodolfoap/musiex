// Offline support: keeps every file of the practice screen, so once it has
// been opened (or installed) it starts and plays without a connection.
//
// pkg/version.js is written by build.sh from a hash of the files. A new build
// changes it, the browser installs this worker again with a fresh cache, and
// the old cache is dropped, so the files always come from the same build.
importScripts('pkg/version.js');

const CACHE = `musiex-${VERSION}`;
const FILES = [
  './',
  'index.html',
  'style.css',
  'app.js',
  'manifest.webmanifest',
  'icon.svg',
  'icon-192.png',
  'icon-512.png',
  'pkg/musiex_web.js',
  'pkg/musiex_web_bg.wasm',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(FILES.map((f) => new Request(f, { cache: 'reload' }))))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(
        keys.filter((k) => k.startsWith('musiex-') && k !== CACHE).map((k) => caches.delete(k)),
      ))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') return;
  event.respondWith(
    caches.match(event.request, { ignoreSearch: true }).then((hit) => hit || fetch(event.request)),
  );
});
