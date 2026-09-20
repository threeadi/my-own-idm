# My Own IDM — IDM Turbo Desktop ⚡

> **A High-Performance Desktop Internet Download Manager** built with **Tauri v2**, **Rust**, **Svelte 5 (Runes)**, and styled after Google Stitch's **"Kinetic Telemetry"** dark-mode design system.

[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-blue?logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-orange?logo=rust)](https://www.rust-lang.org)
[![Svelte 5](https://img.shields.io/badge/Svelte-5.x%20Runes-red?logo=svelte)](https://svelte.dev)
[![Vitest](https://img.shields.io/badge/Vitest%20Frontend%20Coverage-98.88%25-brightgreen?logo=vitest)](https://vitest.dev)
[![Cargo llvm-cov](https://img.shields.io/badge/Rust%20Backend%20Coverage-81.28%25-brightgreen?logo=rust)](https://github.com/taiki-e/cargo-llvm-cov)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## 🌟 Fitur Utama (Key Features)

### 1. 🚀 Mesin Akselerasi Multi-Thread (Turbo Multi-Part Engine)
- **Dynamic Chunk Splitting**: Membagi berkas secara byte-exact ke dalam 1 hingga 32 koneksi thread paralel melalui HTTP `Range` request.
- **Concurrent File Writer**: Pre-alokasi berkas sparse di disk dan penulisan chunk konkuren non-blocking dari beberapa worker thread tanpa race condition.
- **Auto-Resume & Crash Safety**: Merekam progres per segmen secara berkala ke SQLite database. Jika koneksi terputus atau aplikasi ditutup, unduhan dapat dilanjutkan (*resumed*) tanpa mengulang dari awal.

### 2. 🎬 Universal Media & Stream Grabber
- **YouTube, Instagram Reels, TikTok, & X (Twitter)**: Terintegrasi dengan `yt-dlp` dan `ffmpeg` untuk memproses video, audio, dan remuxing otomatis ke format `.mp4` utuh berkualitas tinggi tanpa korup.
- **Native HLS / M3U8 Stream Downloader**: Mengunduh dan menggabungkan segmen `.ts` secara mandiri, lengkap dengan dekripsi roundtrip AES-128-CBC.

### 3. 🎨 Desain Sistem Google Stitch ("Kinetic Telemetry")
- **Two-Tier Toolbar**: Header atas untuk navigasi & pencarian global; toolbar bawah untuk aksi instan (*+ Tambah URL*, *Mulai Semua*, *Jeda Semua*, *Bersihkan Selesai*, *Batasi Kecepatan*).
- **Bento Telemetry Dashboard**:
  - Speedometer real-time dengan angka monospaced JetBrains Mono.
  - Grafik SVG live waveform 60-detik dengan efek cyan glow.
  - 4 kartu metrik telemetri ringkas (Sedang Mengunduh, Dijeda, Selesai, Total Volume).
- **Tabel & Kartu Antrean Fleksibel**: Beralih antara *Cards View* (bento kartu tugas) dan *Table View* (tabel data dense), dilengkapi bar segmen thread beranimasi *striped shimmer*.
- **Dialog Modal Translucent & Draggable**:
  - **Tambah Unduhan Baru**: Shell akrilik dengan deteksi clipboard instan, indikator HTTPS TLS v1.3, dan pemilih 4, 8, atau 16 jalur thread turbo.
  - **Progress In-Flight**: Dual progress bar, tabel status 16-thread live, tombol kontrol cepat, serta header yang dapat digeser (*data-tauri-drag-region*).
  - **Download Outcome Modal**: Otomatis muncul saat unduhan selesai (dengan tombol *Buka Berkas*, *Buka Folder*, dan *Tutup*) atau gagal (dengan rincian pesan error dan tombol *Coba Lagi*).

### 4. 🧩 Ekstensi Browser & Inter-Process Communication (IPC)
- **Floating Video Grabber Button**: Muncul otomatis di sisi video YouTube, Instagram Reels, TikTok, atau Shorts saat kursor melintas atau saat video diputar.
- **Dual-Channel IPC**:
  - Windows Named Pipe (`\\.\pipe\myownidm-ipc`) dengan protokol 4-byte LE length-prefixed JSON.
  - Localhost HTTP Server fallback di `http://127.0.0.1:17890/download` dengan CORS & Private Network Access (PNA) headers.
  - Auto-register Chrome & Firefox Native Messaging Host manifest di Windows Registry.

### 5. 🛡️ SQLite Database & Logging Thread-Safe
- Database SQLite lokal tersimpan aman di direktori `%APPDATA%\com.myownidm.app\idm.db`.
- Log harian terpusat dengan rotasi file dan ring buffer di `%APPDATA%\com.myownidm.app\logs\idm.log`.

---

## 🏗️ Arsitektur Sistem (Architecture)

```mermaid
graph TD
    Browser[Browser / YouTube / Instagram] -->|Click Floating Grabber| Ext[Chrome / Firefox Extension]
    Ext -->|Named Pipe / HTTP IPC| IPC[src-tauri / ipc_server.rs]
    
    subgraph Rust Core (src-tauri)
        IPC --> Manager[Download Manager]
        Manager --> Probe[HTTP Probe / yt-dlp]
        Manager --> Worker[Segment Workers 1..32]
        Manager --> HLS[HLS / M3U8 AES-128 Engine]
        Manager --> Writer[Concurrent FileWriter]
        Manager --> DB[(SQLite Database)]
        Manager --> Logger[AppLogger]
    end

    subgraph Desktop UI (Svelte 5 / TailwindCSS)
        Manager -->|Tauri Events| Store[idmStore.svelte.ts]
        Store --> Toolbar[Toolbar.svelte]
        Store --> Bento[TelemetryBento.svelte]
        Store --> Table[DownloadTable.svelte]
        Store --> SegmentVis[SegmentVisualizer.svelte]
        Store --> Modals[Add / Progress / Outcome Modals]
    end
```

---

## 📂 Struktur Proyek (Directory Layout)

```
my-own-idm/
├── src/                          # Frontend Svelte 5 SPA
│   ├── app.css                   # Stitch "Kinetic Telemetry" CSS Tokens & Animations
│   ├── app.html                  # Google Fonts (Plus Jakarta Sans, JetBrains Mono)
│   ├── lib/
│   │   ├── idmStore.svelte.ts    # Central Reactive State Store (Runes $state/$derived)
│   │   ├── idmStore.test.ts      # Store & Event Unit Tests
│   │   ├── types.ts              # TypeScript Interfaces & Utility Formatters
│   │   └── types.test.ts         # Types & Formatters Unit Tests
│   ├── components/               # Modular UI Components
│   │   ├── Toolbar.svelte        # Two-Tier Navigation Toolbar
│   │   ├── Sidebar.svelte        # Category & Engine Status Sidebar
│   │   ├── TelemetryBento.svelte # Speedometer & SVG Waveform Dashboard
│   │   ├── DownloadTable.svelte  # Cards & Table Task Queue
│   │   ├── SegmentVisualizer.svelte # Multi-Thread Live Segment Monitor
│   │   ├── AddDownloadModal.svelte  # Acrylic Glass Shell New Download Dialog
│   │   ├── DownloadProgressModal.svelte # Dual Progress In-Flight Transfer Dialog
│   │   ├── DownloadOutcomeModal.svelte  # Finished & Error Notification Dialog
│   │   └── SettingsModal.svelte  # Application Preferences Dialog
│   └── routes/
│       └── +page.svelte          # Main Application Viewport
├── src-tauri/                    # Backend Rust / Tauri Core
│   ├── Cargo.toml                # Rust Dependencies & Optimizations
│   ├── tauri.conf.json           # Tauri Window & Security Config
│   └── src/
│       ├── lib.rs                # Tauri App Entry Point & Window Event Handler
│       ├── commands.rs           # Tauri IPC Commands
│       ├── db/                   # SQLite Storage Engine & CRUD Tests
│       ├── logger.rs             # Thread-Safe Rotating File Logger
│       ├── native_messaging.rs   # Browser Native Messaging Host IO
│       ├── ipc_server.rs         # Windows Named Pipe & HTTP Server
│       ├── tray.rs               # System Tray Icon & Context Menu
│       └── engine/
│           ├── manager.rs        # Download Orchestrator & Segment Allocator
│           ├── worker.rs         # HTTP Range Worker Threads
│           ├── writer.rs         # Concurrent Block Pre-allocating File Writer
│           ├── probe.rs          # HTTP HEAD/GET Prober & Filename Parser
│           ├── hls.rs            # M3U8 Playlist Parser & AES-128 Decryptor
│           ├── ytdlp.rs          # Stream Runner & Process Output Parser
│           └── types.rs          # Rust Enums, Structs, & Serialization
├── extension/                    # Browser Extension (Manifest V3)
│   ├── manifest.json             # Extension Permissions & Declarations
│   ├── background.js             # Media Sniffer & Native Messaging Sender
│   ├── content.js                # Floating Video Grabber Injector
│   ├── content.css               # Obsidian Glass & Cyan Glowing Button Styling
│   └── content.test.js           # Extension Sanitization Unit Tests
├── package.json                  # Scripts & NPM Dependencies
├── svelte.config.js              # SvelteKit Static Adapter Config
├── vite.config.js                # Vite Bundler Setup
└── vitest.config.ts              # Vitest Runner & v8 Coverage Thresholds
```

---

## ⚡ Memulai (Getting Started)

### Prasyarat (Prerequisites)
Pastikan lingkungan pengembangan Anda telah terinstal:
1. **Node.js**: v18+ atau v20+ ([nodejs.org](https://nodejs.org))
2. **Rust & Cargo**: Versi stabil terbaru ([rustup.rs](https://rustup.rs))
3. **yt-dlp & ffmpeg** *(opsional untuk video YouTube / medsos)*:
   ```powershell
   winget install yt-dlp
   winget install ffmpeg
   ```

### Instalasi & Menjalankan Dev Mode
1. Clone repositori ini:
   ```bash
   git clone https://github.com/username/my-own-idm.git
   cd my-own-idm
   ```
2. Instal dependensi Node.js:
   ```bash
   npm install
   ```
3. Jalankan aplikasi dalam mode dev (Vite + Tauri):
   ```bash
   npm run tauri dev
   ```

### Membangun Versi Produksi (Production Build)
Untuk mengompilasi installer `.msi` atau `.exe` desktop siap pakai:
```bash
npm run tauri build
```
Berkas installer hasil kompilasi akan berada di `src-tauri/target/release/bundle/`.

---

## 🧩 Memasang Ekstensi Browser

1. Buka browser berbasis Chromium (Google Chrome, Microsoft Edge, Brave) atau Firefox.
2. Masuk ke halaman ekstensi:
   - **Chrome**: `chrome://extensions/`
   - **Edge**: `edge://extensions/`
3. Aktifkan **Developer mode** (Mode pengembang) di sudut kanan atas.
4. Klik tombol **Load unpacked** (Muat yang belum dibongkar).
5. Arahkan ke folder `my-own-idm/extension`.
6. Ekstensi **My Own IDM Integration Module** siap digunakan! Saat memutar video di YouTube, Instagram Reels, atau TikTok, tombol mengambang `⚡ Download this video [TURBO]` akan muncul di samping video.

---

## 🧪 Pengujian & Coverage (> 80%)

Proyek ini dilengkapi dengan suite pengujian otomatis komprehensif pada lapisan Frontend maupun Backend dengan ambang batas minimal **80% coverage**:

| Layer | Framework | Jumlah Test | Line Coverage | Status |
| :--- | :--- | :---: | :---: | :---: |
| **Frontend & Extension** | Vitest (v8) | **42 / 42** (100%) | **98.88%** | ✅ Passed |
| **Backend Core** | Cargo llvm-cov | **86 / 86** (100%) | **81.28%** | ✅ Passed |

### Menjalankan Seluruh Pengujian:

1. **Frontend Unit Tests + Coverage Report**:
   ```bash
   npm run test:coverage
   ```
2. **Backend Rust Tests**:
   ```bash
   npm run test:backend
   ```
3. **Backend Rust Coverage (Summary Only)**:
   ```bash
   npm run test:backend:coverage
   ```

---

## 📄 Lisensi (License)

Proyek ini dilisensikan di bawah lisensi **MIT License** — silakan gunakan, pelajari, dan kembangkan secara bebas.
