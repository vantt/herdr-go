# Architecture

`systemd → herdctl → herdr → coding agents`. The Rust backend supervises herdr and exposes authenticated HTTP/WebSocket endpoints. The TypeScript/xterm.js frontend observes terminals and forwards operator input. Herdr remains authoritative for sessions and agents.
