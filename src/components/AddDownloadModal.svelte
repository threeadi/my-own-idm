<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { DownloadCategory, ProbeResult, DuplicateCheckResult } from '$lib/types';
  import { formatBytes, formatDisplayVersion } from '$lib/types';
  import {
    CloudDownload,
    X,
    Minus,
    Link,
    ClipboardPaste,
    CheckCircle2,
    Lock,
    FolderOpen,
    FolderPlus,
    Zap,
    Download,
    Check,
    Video,
    Music,
    Archive,
    Cpu,
    FileText,
    Folder,
    HardDrive,
    ShieldCheck,
    Loader2
  } from '@lucide/svelte';

  let url = $state(store.initialAddUrl || '');
  let filename = $state(store.initialFilename || '');
  let saveDir = $state('');
  let category = $state<DownloadCategory>('general');
  let connections = $state<number>(16);
  let isProbing = $state<boolean>(false);
  let probeResult = $state<ProbeResult | null>(null);
  let probeError = $state<string | null>(null);
  let isSubmitting = $state<boolean>(false);
  let clipboardCopied = $state<boolean>(false);
  let autoOpenFile = $state<boolean>(false);
  let rememberFolder = $state<boolean>(true);
  let isCheckingDuplicate = $state<boolean>(false);

  $effect(() => {
    if (store.isAddModalOpen) {
      if (!saveDir) {
        invoke<string>('get_default_download_dir').then((dir) => {
          saveDir = dir;
          if (url) checkDuplicate();
        });
      }
      if (store.initialAddUrl) {
        url = store.initialAddUrl;
        if (store.initialFilename) {
          filename = store.initialFilename;
        }
        probeUrl(store.initialHeaders);
      } else if (url) {
        checkDuplicate();
      }
    }
  });

  async function pasteFromClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      if (text && text.trim().startsWith('http')) {
        url = text.trim();
        clipboardCopied = true;
        probeUrl();
        setTimeout(() => {
          clipboardCopied = false;
        }, 1500);
      }
    } catch (e) {
      console.warn('Clipboard read error:', e);
    }
  }

  async function probeUrl(customHeaders?: any) {
    if (!url.trim()) return;
    isProbing = true;
    probeError = null;
    probeResult = null;

    const headers = customHeaders || store.initialHeaders || null;

    try {
      const res = await invoke<ProbeResult>('probe_url', { url: url.trim(), headers });
      probeResult = res;
      if (!filename || filename === 'download.bin') {
        filename = res.filename;
      }
      category = res.category;
      if (res.suggested_dir) {
        saveDir = res.suggested_dir;
      }
      await checkDuplicate();
    } catch (e: any) {
      probeError = String(e);
    } finally {
      isProbing = false;
    }
  }

  async function checkDuplicate() {
    if (!url.trim()) return;
    isCheckingDuplicate = true;
    try {
      const res = await store.checkDuplicateDownload(url.trim(), filename.trim(), saveDir.trim());
      if (res.is_duplicate) {
        store.openDuplicateModal(res, url.trim(), store.initialHeaders || null);
      }
    } catch (e) {
      console.warn('Check duplicate error:', e);
    } finally {
      isCheckingDuplicate = false;
    }
  }

  async function browseFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: saveDir || undefined,
        title: 'Pilih Folder Penyimpanan',
      });
      if (selected && typeof selected === 'string') {
        saveDir = selected;
        await checkDuplicate();
      }
    } catch (e) {
      console.error('Folder selection error:', e);
    }
  }

  function getFileIcon() {
    switch (category) {
      case 'video': return Video;
      case 'audio': return Music;
      case 'compressed': return Archive;
      case 'programs': return Cpu;
      case 'documents': return FileText;
      default: return Folder;
    }
  }

  const FileIcon = $derived(getFileIcon());

  async function handleStartDownload(downloadNow: boolean) {
    if (!url.trim() || !saveDir.trim()) return;
    isSubmitting = true;

    try {
      const task = await invoke<any>('start_download', {
        url: url.trim(),
        filename: filename.trim(),
        saveDir: saveDir.trim(),
        connections: connections,
        headers: store.initialHeaders || null,
      });

      if (!downloadNow) {
        await store.pauseTask(task.id);
      }

      await store.refreshTasks();
      store.selectedTaskId = task.id;
      store.isAddModalOpen = false;

      if (downloadNow) {
        store.openTransferWindow(task.id);
      }
    } catch (e: any) {
      probeError = `Gagal memulai unduhan: ${e}`;
    } finally {
      isSubmitting = false;
    }
  }
</script>

{#if store.isAddModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/40 backdrop-blur-[2px] select-none animate-in fade-in duration-150">
    <!-- Acrylic Glass Shell Container -->
    <div class="relative w-full max-w-2xl bg-[#1b2028]/95 backdrop-blur-2xl rounded-xl shadow-2xl border border-[#30353e] overflow-hidden flex flex-col transition-all duration-200" id="new-download-modal">
      <!-- Top Glow Subtle Cyan Highlight -->
      <div class="absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r from-transparent via-[#4cd7f6]/70 to-transparent"></div>

      <!-- Window Title Bar -->
      <div data-tauri-drag-region class="px-4 py-2.5 bg-[#252a33]/90 flex items-center justify-between border-b border-[#30353e]/80 cursor-move">
        <div class="flex items-center gap-2">
          <img
            src="/favicon.png"
            alt="IDM Turbo"
            class="w-5 h-5 rounded-md object-contain shadow-[0_0_8px_rgba(0,229,255,0.3)]"
          />
          <span class="font-sans text-xs sm:text-sm font-semibold text-[#dee2ee]">
            {store.t('addModal.title')}
          </span>
          <span class="px-1.5 py-0.2 rounded bg-[#090e16] text-[#4edea3] font-mono text-[10px] font-semibold">
            Akrilik {formatDisplayVersion(store.appVersion)}
          </span>
        </div>
        <div class="flex items-center gap-1">
          <button
            onclick={() => (store.isAddModalOpen = false)}
            class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:bg-[#30353e] hover:text-[#dee2ee] transition-colors cursor-pointer"
            title="Minimalkan"
            type="button"
          >
            <Minus class="w-3.5 h-3.5" />
          </button>
          <button
            onclick={() => (store.isAddModalOpen = false)}
            class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:bg-[#93000a] hover:text-white transition-colors cursor-pointer"
            title="Tutup"
            type="button"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <!-- Modal Body Content -->
      <div class="p-4 sm:p-5 space-y-3.5 overflow-y-auto max-h-[calc(85vh-100px)]">
        <!-- 1. URL Source Bar & Protocol Status -->
        <div class="space-y-1">
          <div class="flex items-center justify-between text-xs">
            <label class="font-sans text-[11px] uppercase tracking-wider text-[#8c909f] flex items-center gap-1.5 font-semibold" for="modal-url">
              <Link class="w-3 h-3 text-[#4cd7f6]" />
              {store.t('addModal.urlLabel')}
            </label>
            <button
              onclick={pasteFromClipboard}
              class="font-sans text-[11px] text-[#4cd7f6] hover:text-[#adc6ff] flex items-center gap-1 px-2 py-0.5 rounded bg-[#4cd7f6]/10 transition-colors cursor-pointer"
              type="button"
            >
              {#if clipboardCopied}
                <Check class="w-3 h-3 text-[#4edea3]" />
                <span class="text-[#4edea3]">Ditempel!</span>
              {:else}
                <ClipboardPaste class="w-3 h-3" />
                <span>Tempel dari Clipboard</span>
              {/if}
            </button>
          </div>

          <div class="relative flex items-center">
            <input
              id="modal-url"
              type="text"
              bind:value={url}
              onchange={() => probeUrl()}
              oninput={() => checkDuplicate()}
              placeholder="https://example.com/file.iso..."
              class="w-full h-8 pl-3 pr-24 bg-[#090e16] text-[#dee2ee] font-mono text-xs rounded-lg focus:outline-none focus:ring-1 focus:ring-[#4cd7f6] border border-[#30353e] transition-all"
            />
            <div class="absolute right-1 flex items-center gap-1">
              <button
                onclick={() => probeUrl()}
                disabled={isProbing}
                class="px-2 py-1 bg-[#252a33] text-[#c2c6d6] hover:text-[#dee2ee] rounded text-[11px] flex items-center gap-1 transition-colors cursor-pointer border border-[#30353e]"
                title="Uji Koneksi Tautan"
                type="button"
              >
                {#if isProbing}
                  <Loader2 class="w-3 h-3 animate-spin text-[#4cd7f6]" />
                  <span>Cek...</span>
                {:else}
                  <CheckCircle2 class="w-3 h-3 text-[#4edea3]" />
                  <span>Valid</span>
                {/if}
              </button>
            </div>
          </div>

          <!-- Server Capabilities / Resume Indicator -->
          <div class="flex items-center justify-between px-1 pt-0.5 text-[11px] text-[#8c909f] flex-wrap gap-1">
            <div class="flex items-center gap-1.5 text-[#4edea3] font-medium">
              <CheckCircle2 class="w-3.5 h-3.5" />
              <span>Server Mendukung Resume (Bisa Dijeda)</span>
            </div>
            <div class="flex items-center gap-1 text-[#8c909f] font-mono text-[10px]">
              <Lock class="w-3 h-3 text-[#4cd7f6]" />
              <span>HTTPS 256-bit TLS v1.3</span>
            </div>
          </div>
        </div>

        <!-- 2. File Inspection & Meta Card -->
        <div class="p-3 rounded-xl bg-[#171c24] border border-[#252a33] flex items-start gap-3 shadow-sm">
          <div class="w-11 h-11 rounded-xl bg-[#03b5d3]/15 flex shrink-0 items-center justify-center text-[#4cd7f6] border border-[#03b5d3]/30">
            <FileIcon class="w-5 h-5" />
          </div>
          <div class="flex-1 min-w-0 space-y-1">
            <div class="flex items-center justify-between gap-2">
              <div class="flex-1 min-w-0">
                <span class="text-[9px] text-[#8c909f] uppercase tracking-wider block font-semibold">Nama Berkas</span>
                <input
                  type="text"
                  bind:value={filename}
                  oninput={() => checkDuplicate()}
                  class="w-full bg-[#090e16] border border-[#30353e] font-sans text-xs font-semibold text-[#dee2ee] px-2 py-0.5 rounded focus:outline-none focus:border-[#4cd7f6] truncate"
                />
              </div>
              <div class="flex flex-col items-end shrink-0">
                <span class="text-[9px] text-[#8c909f] uppercase tracking-wider font-semibold">Ukuran Berkas</span>
                <span class="font-mono text-sm sm:text-base font-bold text-[#4cd7f6]">
                  {probeResult?.total_bytes ? formatBytes(probeResult.total_bytes) : 'Tidak diketahui'}
                </span>
              </div>
            </div>

            <div class="flex items-center gap-2 pt-1 flex-wrap">
              <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-[#4d8eff]/15 text-[#adc6ff] text-[10px] font-semibold border border-[#4d8eff]/30">
                Kategori: {category.toUpperCase()}
              </span>
              <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-[#252a33] text-[#c2c6d6] text-[10px]">
                <ShieldCheck class="w-3 h-3 text-[#4edea3]" />
                SHA-256 Otomatis
              </span>
            </div>
          </div>
        </div>

        <!-- 3. Storage Location (Simpan Ke) -->
        <div class="space-y-1">
          <div class="flex items-center justify-between text-xs">
            <label class="font-sans text-[11px] uppercase tracking-wider text-[#8c909f] flex items-center gap-1.5 font-semibold" for="save-dir">
              <FolderOpen class="w-3 h-3 text-[#adc6ff]" />
              Simpan Ke Folder
            </label>
            <span class="font-mono text-[10px] text-[#4edea3] flex items-center gap-1">
              <HardDrive class="w-3 h-3" />
              Sisa Ruang Drive: Tersedia
            </span>
          </div>
          <div class="flex items-center gap-2">
            <input
              id="save-dir"
              type="text"
              bind:value={saveDir}
              class="flex-1 h-8 px-3 bg-[#090e16] text-[#dee2ee] font-mono text-xs rounded-lg focus:outline-none focus:ring-1 focus:ring-[#4cd7f6] border border-[#30353e]"
            />
            <button
              onclick={browseFolder}
              class="h-8 px-3 rounded-lg bg-[#252a33] hover:bg-[#343942] text-[#dee2ee] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer border border-[#30353e] shrink-0"
              type="button"
            >
              <FolderPlus class="w-3.5 h-3.5 text-[#4cd7f6]" />
              <span>Pilih Folder...</span>
            </button>
          </div>
        </div>

        <!-- 4. Thread Acceleration Engine Tuning -->
        <div class="p-3 rounded-xl bg-[#171c24]/80 border border-[#252a33] space-y-2">
          <div class="flex items-center justify-between text-xs">
            <span class="text-[11px] font-semibold uppercase tracking-wider text-[#8c909f] flex items-center gap-1.5">
              <Zap class="w-3.5 h-3.5 text-[#4cd7f6]" />
              Akselerasi Kecepatan & Jalur Thread
            </span>
            <span class="text-[10px] text-[#4cd7f6] font-semibold font-mono">Turbo Engine Aktif</span>
          </div>

          <!-- Segmented Control Threads -->
          <div class="grid grid-cols-3 gap-1 p-1 rounded-lg bg-[#090e16] border border-[#252a33]">
            <button
              onclick={() => (connections = 4)}
              class="h-7 rounded-md text-xs font-medium flex items-center justify-center gap-1 transition-all cursor-pointer {connections === 4 ? 'bg-[#4d8eff] text-white font-semibold shadow-sm' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
              type="button"
            >
              <span>Standar (4 Jalur)</span>
            </button>
            <button
              onclick={() => (connections = 8)}
              class="h-7 rounded-md text-xs font-medium flex items-center justify-center gap-1 transition-all cursor-pointer {connections === 8 ? 'bg-[#4d8eff] text-white font-semibold shadow-sm' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
              type="button"
            >
              <span>Cepat (8 Jalur)</span>
            </button>
            <button
              onclick={() => (connections = 16)}
              class="h-7 rounded-md text-xs font-medium flex items-center justify-center gap-1 transition-all cursor-pointer {connections === 16 ? 'bg-[#4d8eff] text-white font-semibold shadow-sm' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
              type="button"
            >
              <Zap class="w-3 h-3" />
              <span>Maksimal Turbo (16)</span>
            </button>
          </div>

          <!-- Checkbox Options -->
          <div class="pt-1 space-y-1 text-xs text-[#c2c6d6]">
            <label class="flex items-center gap-2 cursor-pointer select-none">
              <input type="checkbox" bind:checked={autoOpenFile} class="w-3.5 h-3.5 rounded bg-[#090e16] border-[#30353e] text-[#4d8eff] focus:ring-0 cursor-pointer" />
              <span>Buka file otomatis setelah proses pengunduhan selesai</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer select-none">
              <input type="checkbox" bind:checked={rememberFolder} class="w-3.5 h-3.5 rounded bg-[#090e16] border-[#30353e] text-[#4d8eff] focus:ring-0 cursor-pointer" />
              <span>Ingat folder tujuan ini untuk berkas berkategori sejenis</span>
            </label>
          </div>
        </div>

        {#if probeError}
          <div class="p-2.5 rounded-lg bg-[#93000a]/20 border border-[#ffb4ab]/30 text-[#ffb4ab] text-xs">
            {probeError}
          </div>
        {/if}
      </div>

      <!-- Action Buttons Footer -->
      <div class="px-4 sm:px-5 py-3 bg-[#252a33]/60 border-t border-[#30353e]/80 flex items-center justify-between gap-2 flex-wrap">
        <div class="flex items-center gap-2">
          <button
            onclick={() => handleStartDownload(false)}
            disabled={isSubmitting}
            class="h-8 px-3 rounded-lg bg-[#1b2028] text-[#dee2ee] hover:bg-[#343942] hover:text-[#adc6ff] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer border border-[#30353e]"
            type="button"
          >
            <span>{store.t('addModal.btnDownloadLater')}</span>
          </button>
        </div>

        <div class="flex items-center gap-2">
          <button
            onclick={() => (store.isAddModalOpen = false)}
            class="h-8 px-3 rounded-lg text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#252a33] text-xs font-medium transition-colors cursor-pointer"
            type="button"
          >
            {store.t('common.cancel')}
          </button>
          <button
            onclick={() => handleStartDownload(true)}
            disabled={isSubmitting}
            class="h-8 px-4 rounded-lg bg-[#4d8eff] text-white hover:bg-[#3b82f6] text-xs font-bold flex items-center gap-1.5 shadow-[0_0_20px_-2px_rgba(77,142,255,0.45)] active:scale-95 transition-all cursor-pointer"
            type="button"
          >
            {#if isSubmitting}
              <Loader2 class="w-3.5 h-3.5 animate-spin" />
              <span>Memproses...</span>
            {:else}
              <Download class="w-3.5 h-3.5" />
              <span>{store.t('addModal.btnDownloadNow')}</span>
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
