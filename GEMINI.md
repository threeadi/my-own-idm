# GEMINI.md — Antigravity & AI Agent Guidelines

See the complete developer and agent context guide in [AGENTS.md](file:///d:/Development/my-own-idm/AGENTS.md).

## Quick Reference
- **Stack**: Tauri v2, Rust 2021, Svelte 5 (Runes), TailwindCSS v4, SQLite, Vitest, Cargo llvm-cov.
- **Design System**: Google Stitch "Kinetic Telemetry" dark-mode (`#0f141c`, `#00e5ff`, `#10b981`, `#ff5252`).
- **Download Engine**: Multi-threaded Range chunking (1–32 threads), HLS/M3U8 decryption, yt-dlp/ffmpeg remuxing.
- **Test Coverage Requirement**: Minimum **> 80% coverage** on both Frontend (`npm run test:coverage`) and Backend (`npm run test:backend:coverage`).
- **Transfer Window & In-Place Lifecycle**: Active transfers open in a dedicated native Tauri floating window (`/transfer?id=...`), seamlessly morphing in-place into completion ("Buka Berkas", "Buka Folder", "Tutup") or failure ("Perbarui Tautan Unduhan...", "Coba Lagi", "Tutup") states.
- **Window Styling**: Modals use lightweight translucent overlays (`bg-black/40 backdrop-blur-[2px]`) with draggable headers (`data-tauri-drag-region`).
- **URL Expiration & Refresh Link**: Supports IDM-style "Perbarui Tautan" (Refresh Download Address) via context menu on paused/failed tasks or failure modal; preserves existing downloaded byte progress when replacing expired URLs.
- **GlitchTip & Crash Reporting**: Sentry SDK v0.34 integration connected to GlitchTip (`crash_reporter.rs`), automatic panic capture, multi-tier extension failover, and sensitive query token/user path sanitization.
- **Hybrid Logging**: In-memory ring buffer for live UI diagnostics (1,000 lines) + daily rotating file logger (`%APPDATA%\com.myownidm.app\logs\idm.log`).
- **Internationalization (i18n)**: Full bilingual support (`id` & `en`) via `src/lib/i18n.ts` and reactive Svelte 5 store.

