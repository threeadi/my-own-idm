# GEMINI.md — Antigravity & AI Agent Guidelines

See the complete developer and agent context guide in [AGENTS.md](file:///d:/Development/my-own-idm/AGENTS.md).

## Quick Reference
- **Stack**: Tauri v2, Rust 2021, Svelte 5 (Runes), TailwindCSS v4, SQLite, Vitest, Cargo llvm-cov.
- **Design System**: Google Stitch "Kinetic Telemetry" dark-mode (`#0f141c`, `#00e5ff`, `#10b981`, `#ff5252`).
- **Download Engine**: Multi-threaded Range chunking (1–32 threads), HLS/M3U8 decryption, yt-dlp/ffmpeg remuxing.
- **Test Coverage Requirement**: Minimum **> 80% coverage** on both Frontend (`npm run test:coverage`) and Backend (`npm run test:backend:coverage`).
- **Modal Lifecycle**: Progress dialog auto-closes upon completion or failure, displaying `DownloadOutcomeModal` with direct file & folder opening triggers.
- **Window Styling**: Modals use lightweight translucent overlays (`bg-black/40 backdrop-blur-[2px]`) with draggable headers (`data-tauri-drag-region`).
