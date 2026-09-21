# AGENTS.md — Developer & AI Agent Context Guide

> This document serves as the authoritative operational manual for AI coding assistants (Antigravity, Cursor, Claude Code, Copilot, etc.) and human contributors working on the **My Own IDM (IDM Turbo Desktop)** codebase.

---

## 1. Project Overview & Technology Stack

| Layer | Technologies & Libraries | Key Responsibilities |
| :--- | :--- | :--- |
| **Desktop Shell** | **Tauri v2** (`src-tauri`) | OS integration, multi-window management, frameless drag regions, system tray, native file dialogs |
| **Backend Core** | **Rust 2021** (Tokio, Reqwest, Rusqlite, Parking_lot, AES-128, CBC, Tracing) | Multi-thread Range download engine, concurrent pre-allocating file writer, M3U8/HLS decryption, yt-dlp remuxing, token bucket rate limiter, IPC server |
| **Crash Reporting** | **GlitchTip / Sentry SDK v0.34** (`crash_reporter.rs`) | Sentry protocol integration, fatal panic hooks, sensitive credential & user profile sanitization, multi-tier failover |
| **Frontend UI** | **Svelte 5** (Runes `$state`, `$derived`), **SvelteKit**, **TailwindCSS v4**, **Lucide Svelte** | Stitch "Kinetic Telemetry" dark UI, reactive event listeners, bento telemetry speedometer & waveform, modals, bilingual i18n (`id`/`en`) |
| **Browser Extension** | **Manifest V3** (`extension/`) | Floating video grabber button overlay, media traffic sniffer, Windows Named Pipe & HTTP IPC sender, GlitchTip direct store fallback |
| **Storage & Logs** | **SQLite** (`rusqlite`), **Hybrid Logger** (`logger.rs`) | Persistent tasks, segment progress checkpoints, crash-resilient restart, in-memory ring buffer (1,000 lines) + daily rotating diagnostic logs |
| **Test Suites** | **Vitest (v8)**, **Cargo llvm-cov** | Automated unit & integration testing with mandatory **> 80% coverage** threshold |

---

## 2. Codebase Directory Structure

```
d:/Development/my-own-idm/
├── docs/
│   └── screenshots/              # High-resolution UI captures for documentation
├── src/                          # Svelte 5 Single-Page Application
│   ├── app.css                   # Stitch "Kinetic Telemetry" Tokens & Shimmer Animations
│   ├── app.html                  # HTML Shell & Google Fonts (Plus Jakarta Sans, JetBrains Mono)
│   ├── lib/
│   │   ├── idmStore.svelte.ts    # Central Reactive State Store (Runes $state, $derived)
│   │   ├── idmStore.test.ts      # Frontend Unit Tests (Vitest)
│   │   ├── i18n.ts               # Complete Bilingual Dictionaries (Indonesian & English)
│   │   ├── types.ts              # TypeScript Interfaces & Utility Formatters
│   │   ├── types.test.ts         # Utility Formatter Unit Tests
│   │   └── version.ts            # Application Version & Build Metadata
│   ├── components/               # Modular UI Components
│   │   ├── Toolbar.svelte        # Two-Tier Application Toolbar
│   │   ├── Sidebar.svelte        # Categorized Status & Navigation Sidebar
│   │   ├── TelemetryBento.svelte # Speedometer Gauge & 60s SVG Waveform Visualizer
│   │   ├── DownloadTable.svelte  # Cards & Table Task Queue with Segment Bars
│   │   ├── SegmentVisualizer.svelte # Discrete Multi-Thread Segment Monitor
│   │   ├── AddDownloadModal.svelte  # Acrylic Glass Shell New Download Dialog
│   │   ├── DownloadProgressModal.svelte # Dual Progress In-Flight Transfer Dialog
│   │   ├── DownloadOutcomeModal.svelte  # Completion & Error Alert Notification Dialog
│   │   ├── RefreshLinkModal.svelte  # Expired URL Refresh & Browser Sniffer Dialog
│   │   ├── PropertiesModal.svelte   # Detailed Task & Checksum Inspection Dialog
│   │   └── SettingsModal.svelte  # Application Preferences & Extension Integrator
│   └── routes/
│       ├── +page.svelte          # Main Application Viewport
│       └── transfer/
│           └── +page.svelte      # Dedicated Native Floating Window for In-Flight Transfers
├── src-tauri/                    # Rust Backend (Tauri v2)
│   ├── Cargo.toml                # Dependencies & Build Profiles (Package: idm-turbo-desktop)
│   ├── tauri.conf.json           # Tauri Window & Security Policy
│   └── src/
│       ├── lib.rs                # App Entry Point, Plugins & Window Events
│       ├── commands.rs           # Tauri IPC Command Handlers
│       ├── crash_reporter.rs     # GlitchTip / Sentry Integration & URL/Path Sanitizer
│       ├── db/                   # SQLite Database CRUD & In-Memory Unit Tests
│       ├── logger.rs             # Hybrid Thread-Safe Ring Buffer & Rotating File Logger
│       ├── native_messaging.rs   # Browser Native Messaging Protocol
│       ├── ipc_server.rs         # Windows Named Pipe & Localhost HTTP Server
│       ├── tray.rs               # System Tray Context Menu & Window Toggling
│       └── engine/
│           ├── manager.rs        # Download Orchestrator & Task Evaluator
│           ├── worker.rs         # HTTP Range Worker Execution
│           ├── writer.rs         # Pre-allocated Concurrent File Writer
│           ├── probe.rs          # HTTP HEAD/GET Prober & Filename Parser
│           ├── limiter.rs        # Token Bucket Rate Limiter (Global & Per-Task)
│           ├── hls.rs            # M3U8 Playlist Parser & AES-128 Decryption
│           ├── ytdlp.rs          # yt-dlp Process Runner & Output Stream Parser
│           └── types.rs          # Rust Types, Enums & Structs
└── extension/                    # Browser Extension (Chrome/Firefox/Edge)
    ├── manifest.json             # Extension Declaration (V3)
    ├── background.js             # Media Traffic Sniffer, IPC Forwarder & GlitchTip Direct Fallback
    ├── content.js                # Floating Video Grabber Overlay Injector
    ├── content.css               # Obsidian Glass & Cyan Glowing Button Styling
    └── content.test.js           # Extension Unit Tests (Vitest)
```

---

## 3. Core Architectural Patterns

### A. IPC Communication Flow
1. **Browser Extension to Rust Core**:
   - Primary: Windows Named Pipe `\\.\pipe\myownidm-ipc` (4-byte little-endian length-prefixed JSON).
   - Fallback: HTTP POST `http://127.0.0.1:18888/download` (with CORS and Private Network Access headers).
   - Error Dispatch: HTTP POST `http://127.0.0.1:18888/error-report` for browser extension diagnostics.
2. **Rust Core to Frontend Svelte Store**:
   - `emitter.emit_event("download-progress", SpeedMetrics)`: Sent every 150ms during active downloads.
   - `emitter.emit_event("download-completed", taskId)`: Sent on successful completion.
   - `emitter.emit_event("download-paused", taskId)`: Sent on pause or cancel.
   - `emitter.emit_event("download-failed", taskId)`: Sent on unrecoverable failure.
   - `emitter.emit_event("browser-download-requested", payload)`: Triggers the new download dialog or URL refresh.

### B. Download Engine Execution & Finalization
- **Range vs Stream Detection**:
  - Regular files: probed via HTTP `Range: bytes=0-0` request; if `206 Partial Content`, chunks are distributed across 1 to 32 worker threads.
  - Video streams (YouTube, Instagram, TikTok, HLS): routed through `YtDlpRunner` or `HlsDownloader` to avoid byte-range corruption and ensure proper video+audio remuxing into `.mp4`.
- **Completion Synchronization (Crucial)**:
  - When `outcome` evaluates to `Ok(final_bytes)`, `file_len` on disk is verified.
  - `task.total_bytes` and `task.downloaded_bytes` MUST be synchronized to `actual_final_bytes`.
  - SQLite database `mark_task_completed` records `downloaded_bytes = actual_final_bytes` and `total_bytes = actual_final_bytes`.
  - When `task.status === 'completed'`, `getPercent(task)` MUST evaluate to `100.0%`.

### C. Dedicated Floating Window & In-Place Lifecycle Transitions
- Active transfers open in a dedicated native Tauri floating window ([transfer/+page.svelte](file:///d:/Development/my-own-idm/src/routes/transfer/+page.svelte) via `commands::open_transfer_window`).
- **In-Place Lifecycle Transitions**:
  - The window does NOT close upon finish; instead, it morphs smoothly in-place from transfer progress to the completion view (**Buka Berkas**, **Buka Folder**, **Tutup**) with Emerald glow, or to the failure view (**Perbarui Tautan Unduhan...**, **Laporkan Masalah**, **Coba Lagi**, **Tutup**) with Rose Red glow.
  - Multi-window permission is granted via `"windows": ["*"]` in [default.json](file:///d:/Development/my-own-idm/src-tauri/capabilities/default.json).
  - Windows are frameless with Stitch "Kinetic Telemetry" custom titlebars, draggable via `data-tauri-drag-region`, and support independent minimize/close.
  - In web preview environments, `store.openTransferWindow` automatically falls back to [DownloadProgressModal.svelte](file:///d:/Development/my-own-idm/src/components/DownloadProgressModal.svelte).

### D. Refresh Download Address Flow (Expiring URLs & Resumption)
- **Problem**: Temporary CDN tokens, signed URLs (YouTube, GDrive, video hosts), or long pauses can cause URL expiration (`HTTP 403`, `HTTP 410`, or timeout).
- **Core Invariant**: A download must NEVER lose partially downloaded bytes when its URL expires or times out.
- **Interaction Points**:
  1. In [DownloadOutcomeModal.svelte](file:///d:/Development/my-own-idm/src/components/DownloadOutcomeModal.svelte): Provide a "Perbarui Tautan" (Refresh Link) action when a download fails due to auth/expiration/timeout.
  2. In [DownloadTable.svelte](file:///d:/Development/my-own-idm/src/components/DownloadTable.svelte): Add "Perbarui Tautan Unduhan..." in the contextual right-click menu and action bar for `paused` and `failed` tasks.
- **Workflow**:
  1. Triggering "Refresh Link" opens the original `referer` or source URL in the user's default browser and opens [RefreshLinkModal.svelte](file:///d:/Development/my-own-idm/src/components/RefreshLinkModal.svelte) ("Menunggu Tautan Baru dari Browser...").
  2. When the user re-initiates the download on the webpage, the browser extension captures the fresh URL/cookies and emits `browser-download-requested`.
  3. The desktop core performs a fast probe to validate `supports_range` and `total_bytes` parity, then updates `task.url` in SQLite and in-memory engine, keeping existing `.part` files, downloaded byte counts, and segment checkpoints intact.
  4. The download immediately resumes from the existing byte offset.

### E. GlitchTip / Sentry Crash & Diagnostic Error Reporting
- **Project DSN**: `https://2e56fa98dc824baaa843dff62a18b3d2@app.glitchtip.com/28025`.
- **Rust Core ([crash_reporter.rs](file:///d:/Development/my-own-idm/src-tauri/src/crash_reporter.rs))**:
  - Sentry SDK v0.34 initialized with `sentry::release_name!()` (auto crate version `idm-turbo-desktop@0.1.0-dev`), `traces_sample_rate: 0.01` (1% transaction sampling), and long-lived client guard stored in static `SENTRY_GUARD` to prevent early drop and ensure event flushing.
  - Automatic panic hook captures uncaught fatal panics and backtraces.
  - Sensitive data sanitization:
    - `sanitize_url`: Masks query tokens (`token`, `auth`, `api_key`, `secret`, `signature`, `password`) with `***` and strips HTTP basic auth credentials.
    - `sanitize_path`: Replaces user home/profile paths (`C:\Users\username\` or `/home/user/`) with generic placeholders (`C:\Users\***\`).
    - `sanitize_log_line`: Scans arbitrary log strings and scrubs all embedded URLs and system user paths.
  - `report_download_failure`: Attaches task metadata (category, thread connections, byte offsets, range support) and the last 30 log lines from the in-memory ring buffer.
- **Browser Extension ([background.js](file:///d:/Development/my-own-idm/extension/background.js))**:
  - Multi-tier failover:
    1. Primary: Localhost HTTP IPC `POST http://127.0.0.1:18888/error-report`.
    2. Fallback 1: Native Messaging `type: "error_report"`.
    3. Fallback 2 (Desktop Offline): Direct HTTP POST to GlitchTip Store API (`https://app.glitchtip.com/api/28025/store/`) with Sentry protocol JSON and `X-Sentry-Auth` header.
- **Frontend UI ([DownloadOutcomeModal.svelte](file:///d:/Development/my-own-idm/src/components/DownloadOutcomeModal.svelte))**:
  - Provides reactive "Laporkan Masalah" / "Report Issue" action on download failure with instant feedback (Idle -> Sending -> Sent).

### F. Hybrid Logging Architecture
- **In-Memory Ring Buffer**: Stores the last 1,000 log lines with timestamp and target component, instantly queryable by the UI via `commands::get_recent_logs` without disk I/O bottlenecks.
- **Daily Rotating File Logger**: Writes formatted diagnostic logs to `%APPDATA%\com.myownidm.app\logs\idm.log` with automatic daily rotation.
- **Tracing Subscriber**: Integrated with `tracing` and `tracing-subscriber` for development console diagnostics.

### G. Internationalization (i18n)
- **Bilingual Dictionary ([src/lib/i18n.ts](file:///d:/Development/my-own-idm/src/lib/i18n.ts))**: Complete coverage for Indonesian (`id`) and English (`en`).
- **Reactive Runes Store**: State bound via `store.language` (`'id' | 'en'`) with persistent SQLite storage and instant UI reactivity through `store.t(key)`.

---

## 4. Design System Tokens (Stitch "Kinetic Telemetry")

Always adhere to these styling tokens when modifying or creating frontend components:
- **Canvas / Backgrounds**:
  - `#0f141c` (Deep Obsidian Canvas)
  - `#090e16` (Obsidian Lowest Surface)
  - `#171c24` (Surface Low / Cards)
  - `#252a33` (Surface Medium / Hover)
  - `#30353e` (Surface High / Borders)
- **Accents**:
  - `#00e5ff` (Electric Cyan Accent / Speedometer / Primary Glow)
  - `#4cd7f6` (Secondary Cyan)
  - `#4d8eff` (Primary Blue)
  - `#10b981` (Emerald Success)
  - `#ff5252` / `#ffb4ab` (Rose Red Error / Warning)
- **Typography**:
  - Font Sans: `'Plus Jakarta Sans'`, `'Inter'`, sans-serif.
  - Font Mono: `'JetBrains Mono'`, monospace (for metrics, byte sizes, speeds, ETA, and thread IDs).

---

## 5. Development & Testing Commands

### Running Locally
```powershell
# Run desktop app in development mode
npm run tauri dev

# Run Vite frontend dev server in browser preview mode
npm run dev
```

### Running Test Suites (Mandatory > 80% Coverage)
```powershell
# Run frontend unit tests with v8 coverage (Current: 99 passed, 97.04% lines)
npm run test:coverage

# Run backend Rust unit tests (Current: 125 passed)
npm run test:backend

# Run backend Rust coverage summary (Current: 125 passed, 80.82% lines)
npm run test:backend:coverage
```

### Type Checking & Build
```powershell
# SvelteKit and TypeScript validation (0 errors, 0 warnings)
npm run check

# Compile production desktop bundle
npm run tauri build
```

---

## 6. Guidelines for AI Agents

1. **Maintain Code Coverage**: Never introduce code changes that lower Frontend or Backend coverage below **80%**. Always add corresponding tests in `src/lib/idmStore.test.ts` or `src-tauri` when modifying state logic or engine methods.
2. **Preserve Thread Safety**: Use `parking_lot::Mutex` or `tokio::sync::RwLock` for shared mutable state in Rust. Ensure file handles are closed properly and workers respect the atomic `cancel_flag`.
3. **Keep Design Fidelity**: Do not introduce generic colors (plain red, plain blue). Follow the Stitch Kinetic Telemetry tokens defined above.
4. **Sanitize Sensitive Data**: Always pass URLs and log lines through `crash_reporter::sanitize_url` and `crash_reporter::sanitize_log_line` before attaching to crash reports or diagnostic payloads.
5. **Never Block Dev Server**: Do not terminate long-running processes like `npm run tauri dev` unless explicitly instructed by the user.
