self.onmessage = (event) => {
  if (event.data === "start") {
    self.postMessage("Worker started")
  }
}

self.addEventListener("install", (event) => {
  console.log("[SW] Installed");
  self.skipWaiting(); // Activate immediately
});

self.addEventListener("activate", (event) => {
  console.log("[SW] Activated");
  event.waitUntil(clients.claim()); // Take control of all pages
});

self.addEventListener("fetch", (event) => {
  self.postMessage("Found download")
  if (event.request.url.endsWith("/download")) {
    self.postMessage("Found download")
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("Hello from SW!"));
        controller.close();
      },
    });
    event.respondWith(new Response(stream, {
      headers: {
        "Content-Type": "application/octet-stream",
        "Content-Disposition": "attachment; filename=file.txt"
      }
    }));
  }
});

export default {}
