// Damit die installierte Web-App auch ohne Netz startet: Was einmal geladen wurde, liegt im Cache.
// Seite, Programm und WebAssembly kommen bevorzugt frisch aus dem Netz, damit Updates sofort ankommen;
// Bilder und Töne kommen aus dem Cache und werden im Hintergrund aufgefrischt.
const CACHE = "bunny-skylines";

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) {
    return;
  }
  const isAsset = new URL(request.url).pathname.includes("/assets/");
  event.respondWith(isAsset ? cacheFirst(event, request) : networkFirst(request));
});

async function networkFirst(request) {
  const cache = await caches.open(CACHE);
  try {
    const response = await fetch(request);
    if (response.ok) {
      await cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    const cached = await cache.match(request);
    if (cached) {
      return cached;
    }
    throw error;
  }
}

async function cacheFirst(event, request) {
  const cache = await caches.open(CACHE);
  const cached = await cache.match(request);
  const fresh = fetch(request).then(async (response) => {
    if (response.ok) {
      await cache.put(request, response.clone());
    }
    return response;
  });
  if (cached) {
    event.waitUntil(fresh.catch(() => {}));
    return cached;
  }
  return fresh;
}
