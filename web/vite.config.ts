import { defineConfig } from "vite";

// The web UI builds to ../static, which the axum server serves as the SPA root.
// During dev, /api and /ws are proxied to the running herdctl backend.
export default defineConfig({
  build: {
    outDir: "../static",
    emptyOutDir: true,
  },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8787",
      "/ws": { target: "ws://127.0.0.1:8787", ws: true },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
  },
});
