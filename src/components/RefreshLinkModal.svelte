<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes, getPercent } from '$lib/types';
  import {
    RotateCw,
    X,
    Globe,
    ExternalLink,
    CheckCircle2,
    AlertCircle,
    Copy,
    Check,
    Radio,
    FileText,
    Play
  } from 'lucide-svelte';
  import { onDestroy } from 'svelte';

  let manualUrl = $state('');
  let isManualOpen = $state(false);
  let isCopied = $state(false);
  let countdownTimer: any = null;
  let countdown = $state(3);

  const task = $derived(store.refreshTask);
  const targetUrl = $derived(task?.referer || task?.url || '');

  // Watch for detected URL to start 3-second countdown
  $effect(() => {
    if (store.refreshDetectedUrl && !countdownTimer) {
      countdown = 3;
      countdownTimer = setInterval(() => {
        if (countdown > 1) {
          countdown -= 1;
        } else {
          clearInterval(countdownTimer);
          countdownTimer = null;
          handleApply();
        }
      }, 1000);
    } else if (!store.refreshDetectedUrl && countdownTimer) {
      clearInterval(countdownTimer);
      countdownTimer = null;
    }
  });

  onDestroy(() => {
    if (countdownTimer) {
      clearInterval(countdownTimer);
    }
  });

  async function handleApply() {
    if (countdownTimer) {
      clearInterval(countdownTimer);
      countdownTimer = null;
    }
    const urlToUse = store.refreshDetectedUrl || manualUrl.trim();
    if (!urlToUse) return;
    try {
      await store.applyRefreshedUrl(urlToUse);
    } catch (e) {
      // Error handled in store.refreshError
    }
  }

  function handleManualApply() {
    if (!manualUrl.trim()) return;
    store.refreshDetectedUrl = manualUrl.trim();
  }

  function copySourceUrl() {
    if (!targetUrl) return;
    navigator.clipboard.writeText(targetUrl);
    isCopied = true;
    setTimeout(() => {
      isCopied = false;
    }, 2000);
  }

  function cancelAutoResume() {
    if (countdownTimer) {
      clearInterval(countdownTimer);
      countdownTimer = null;
    }
    store.refreshDetectedUrl = null;
  }
</script>

{#if store.isRefreshModalOpen && task}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-[2px] p-4 animate-in fade-in duration-150"
    role="dialog"
    aria-modal="true"
  >
    <div
      class="w-full max-w-lg bg-[#171c24] border border-[#30353e] rounded-2xl shadow-2xl shadow-black/80 overflow-hidden text-slate-200 flex flex-col scale-in-95 duration-150"
    >
      <!-- Titlebar / Draggable Header -->
      <div
        data-tauri-drag-region
        class="flex items-center justify-between px-5 py-3.5 bg-[#0f141c] border-b border-[#252a33] select-none"
      >
        <div class="flex items-center gap-2.5">
          <div class="p-1.5 rounded-lg bg-[#00e5ff]/10 text-[#00e5ff] border border-[#00e5ff]/20">
            <RotateCw class="w-4 h-4 {store.refreshDetectedUrl ? '' : 'animate-spin'}" style="animation-duration: 3s;" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-sm font-semibold tracking-wide text-white">Perbarui Alamat Unduhan</h2>
              <span class="text-[10px] px-1.5 py-0.5 rounded font-mono font-bold bg-[#00e5ff]/15 text-[#00e5ff] border border-[#00e5ff]/30">
                REFRESH LINK
              </span>
            </div>
            <p class="text-[11px] text-slate-400">IDM Turbo Desktop &bull; Resumption Invariant</p>
          </div>
        </div>

        <button
          onclick={() => store.cancelRefreshLink()}
          class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-[#252a33] transition"
          title="Tutup (Esc)"
          aria-label="Tutup"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Task Context Summary Strip -->
      <div class="px-5 py-3 bg-[#090e16]/60 border-b border-[#252a33] flex items-center justify-between text-xs">
        <div class="flex items-center gap-2.5 min-w-0 flex-1 pr-3">
          <FileText class="w-4 h-4 text-slate-400 shrink-0" />
          <div class="min-w-0">
            <p class="font-medium text-slate-200 truncate font-mono text-[12px]">{task.filename}</p>
            <p class="text-[11px] text-slate-400">
              Progres Tersimpan: <span class="font-mono text-emerald-400 font-medium">{formatBytes(task.downloaded_bytes)}</span>
              {#if task.total_bytes}
                / {formatBytes(task.total_bytes)} ({getPercent(task).toFixed(1)}%)
              {/if}
            </p>
          </div>
        </div>
        <span class="px-2 py-0.5 rounded text-[11px] font-mono font-medium bg-[#252a33] text-slate-300 border border-[#30353e] shrink-0">
          {task.category.toUpperCase()}
        </span>
      </div>

      <!-- Error Alert (if any) -->
      {#if store.refreshError}
        <div class="mx-5 mt-4 p-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-300 text-xs flex items-start gap-2.5">
          <AlertCircle class="w-4 h-4 text-rose-400 shrink-0 mt-0.5" />
          <div>
            <p class="font-medium">Validasi Tautan Gagal</p>
            <p class="text-[11px] text-rose-300/80 mt-0.5">{store.refreshError}</p>
          </div>
        </div>
      {/if}

      <!-- Main Body -->
      <div class="p-5 space-y-4">
        {#if !store.refreshDetectedUrl}
          <!-- STATE 1: LISTENING RADAR MODE -->
          <div class="flex flex-col items-center text-center py-3">
            <!-- Animated Concentric Radar Pulse -->
            <div class="relative flex items-center justify-center w-20 h-20 mb-3">
              <div class="absolute inset-0 rounded-full bg-[#00e5ff]/10 animate-ping opacity-75" style="animation-duration: 2s;"></div>
              <div class="absolute inset-2 rounded-full bg-[#00e5ff]/15 animate-pulse"></div>
              <div class="relative w-12 h-12 rounded-full bg-[#090e16] border border-[#00e5ff]/50 flex items-center justify-center shadow-lg shadow-[#00e5ff]/20">
                <Radio class="w-6 h-6 text-[#00e5ff] animate-pulse" />
              </div>
            </div>

            <h3 class="text-sm font-semibold text-white tracking-wide">
              Menunggu Tautan Baru dari Browser...
            </h3>
            <p class="text-xs text-slate-400 mt-1 max-w-sm leading-relaxed">
              Buka halaman web yang telah dimuat di browser Anda, lalu klik <span class="text-[#00e5ff] font-medium">Download</span> kembali. IDM Turbo akan langsung menangkap link baru dan melanjutkan unduhan.
            </p>
          </div>

          <!-- Web Source Action Card -->
          <div class="bg-[#0f141c] border border-[#252a33] rounded-xl p-3 text-xs space-y-2">
            <div class="flex items-center justify-between text-slate-400 text-[11px]">
              <span class="flex items-center gap-1.5 font-medium text-slate-300">
                <Globe class="w-3.5 h-3.5 text-[#00e5ff]" />
                Halaman Sumber Unduhan
              </span>
              <button
                onclick={copySourceUrl}
                class="hover:text-white flex items-center gap-1 transition"
                title="Salin URL Halaman"
              >
                {#if isCopied}
                  <Check class="w-3.5 h-3.5 text-emerald-400" />
                  <span class="text-emerald-400">Tersalin</span>
                {:else}
                  <Copy class="w-3.5 h-3.5" />
                  <span>Salin URL</span>
                {/if}
              </button>
            </div>

            <div class="p-2 rounded bg-[#090e16] border border-[#252a33] font-mono text-[11px] text-slate-300 truncate select-all">
              {targetUrl || 'Tidak ada referer URL yang tercatat'}
            </div>

            <div class="flex items-center justify-end gap-2 pt-1">
              {#if targetUrl}
                <button
                  onclick={() => store.openExternalUrl(targetUrl)}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-slate-200 font-medium text-xs border border-[#30353e] transition"
                >
                  <ExternalLink class="w-3.5 h-3.5 text-[#00e5ff]" />
                  <span>Buka Ulang Halaman di Browser</span>
                </button>
              {/if}
            </div>
          </div>

          <!-- Collapsible Manual URL Paste Option -->
          <div class="border-t border-[#252a33] pt-3">
            <button
              onclick={() => (isManualOpen = !isManualOpen)}
              class="text-xs text-slate-400 hover:text-[#00e5ff] flex items-center gap-1.5 transition font-medium"
            >
              <span>{isManualOpen ? '▼ Sembunyikan input URL manual' : '▶ Atau tempel URL unduhan baru secara manual'}</span>
            </button>

            {#if isManualOpen}
              <div class="mt-2.5 flex gap-2">
                <input
                  type="text"
                  bind:value={manualUrl}
                  placeholder="https://server.com/download/fresh-token-url..."
                  class="flex-1 bg-[#090e16] border border-[#30353e] rounded-lg px-3 py-1.5 text-xs text-slate-200 placeholder-slate-500 font-mono focus:outline-none focus:border-[#00e5ff]"
                />
                <button
                  onclick={handleManualApply}
                  disabled={!manualUrl.trim()}
                  class="px-3 py-1.5 rounded-lg bg-[#00e5ff]/20 hover:bg-[#00e5ff]/30 text-[#00e5ff] border border-[#00e5ff]/40 text-xs font-semibold disabled:opacity-50 disabled:cursor-not-allowed transition"
                >
                  Terapkan
                </button>
              </div>
            {/if}
          </div>
        {:else}
          <!-- STATE 2: LINK DETECTED CONFIRMATION -->
          <div class="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-300 flex items-start gap-3">
            <div class="p-2 rounded-full bg-emerald-500/20 border border-emerald-500/40 text-emerald-400 shrink-0 mt-0.5">
              <CheckCircle2 class="w-5 h-5" />
            </div>
            <div class="min-w-0 flex-1">
              <h3 class="text-sm font-semibold text-white">Tautan Baru Berhasil Ditangkap!</h3>
              <p class="text-xs text-emerald-200/90 mt-1 leading-relaxed">
                Alamat unduhan baru dari browser siap diterapkan. Progres sebesar <span class="font-mono font-bold text-white">{formatBytes(task.downloaded_bytes)}</span> akan langsung dilanjutkan tanpa mengulang dari 0%.
              </p>

              <div class="mt-2.5 p-2 bg-[#090e16] border border-[#252a33] rounded font-mono text-[11px] text-slate-300 truncate">
                {store.refreshDetectedUrl}
              </div>

              <div class="mt-3 flex items-center justify-between text-xs text-slate-400">
                <span class="flex items-center gap-1.5">
                  <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
                  Lanjut otomatis dalam <strong class="text-white font-mono text-sm">{countdown}s</strong>
                </span>
                <button
                  onclick={cancelAutoResume}
                  class="hover:text-white underline text-[11px] transition"
                >
                  Batal / Ganti Tautan
                </button>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer Buttons -->
      <div class="px-5 py-3.5 bg-[#0f141c] border-t border-[#252a33] flex items-center justify-between">
        <button
          onclick={() => store.cancelRefreshLink()}
          class="px-4 py-2 rounded-xl text-xs font-medium text-slate-400 hover:text-white hover:bg-[#252a33] transition"
        >
          Batal
        </button>

        {#if store.refreshDetectedUrl}
          <button
            onclick={handleApply}
            class="flex items-center gap-2 px-5 py-2 rounded-xl text-xs font-bold bg-[#00e5ff] hover:bg-[#4cd7f6] text-[#090e16] shadow-lg shadow-[#00e5ff]/25 transition active:scale-95"
          >
            <Play class="w-3.5 h-3.5 fill-current" />
            <span>Lanjutkan Unduhan Sekarang</span>
          </button>
        {:else}
          <span class="text-[11px] text-slate-500 font-mono flex items-center gap-1.5">
            <span class="w-2 h-2 rounded-full bg-[#00e5ff] animate-ping"></span>
            Listening on IPC Port 18888
          </span>
        {/if}
      </div>
    </div>
  </div>
{/if}
