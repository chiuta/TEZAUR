/* The PWA used to live at the site root and now lives at /app/. Browsers that installed the old root service worker
   re-fetch this file, which removes the old worker and its caches so the deck at / is served fresh. */
self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', e => e.waitUntil((async () => {
  for (const k of await caches.keys()) await caches.delete(k);
  await self.registration.unregister();
  for (const c of await self.clients.matchAll({ type: 'window' })) c.navigate(c.url);
})()));
