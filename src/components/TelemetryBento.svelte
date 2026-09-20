<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes, formatSpeed } from '$lib/types';
  import {
    Activity,
    ArrowUp,
    HardDrive,
    Zap,
    Download,
    PauseCircle,
    CheckCircle2,
    Database,
    ShieldCheck
  } from '@lucide/svelte';

  const totalSpeed = $derived(store.totalSpeedBps);
  const counts = $derived(store.categoryCounts);

  // Compute total downloaded volume across all tasks
  const totalVolumeBytes = $derived.by<number>(() => {
    return store.tasks.reduce((acc, t) => acc + (t.downloaded_bytes || 0), 0);
  });

  // Determine display speed and unit
  const speedDisplay = $derived.by<{ value: string; unit: string }>(() => {
    if (totalSpeed <= 0) {
      return { value: '0.0', unit: 'KB/s' };
    }
    if (totalSpeed >= 1024 * 1024) {
      return { value: (totalSpeed / (1024 * 1024)).toFixed(1), unit: 'MB/detik' };
    }
    return { value: (totalSpeed / 1024).toFixed(0), unit: 'KB/detik' };
  });
</script>

<section class="relative overflow-hidden rounded-xl bg-[#171c24] border border-[#252a33] shadow-xl p-4 sm:p-5 shrink-0 select-none mb-3">
  <!-- Glowing Ambient Accents -->
  <div class="absolute -right-20 -top-20 w-72 h-72 rounded-full bg-[#4cd7f6]/5 blur-3xl pointer-events-none"></div>
  <div class="absolute -left-16 -bottom-16 w-64 h-64 rounded-full bg-[#4d8eff]/10 blur-3xl pointer-events-none"></div>

  <div class="relative z-10 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-4">
    <!-- Left: Telemetry Highlights -->
    <div class="flex flex-col gap-1 max-w-xl">
      <!-- Top Pills -->
      <div class="flex items-center gap-2 flex-wrap">
        <span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[#10b981]/15 text-[#4edea3] font-sans text-xs border border-[#10b981]/30">
          <span class="w-1.5 h-1.5 rounded-full bg-[#4edea3] animate-pulse"></span>
          Koneksi Sangat Cepat & Stabil
        </span>
        <span class="inline-flex items-center gap-1 text-[#8c909f] font-mono text-xs">
          <HardDrive class="w-3.5 h-3.5 text-[#4cd7f6]" />
          <span>Ruang Drive: <strong class="text-[#dee2ee] font-sans font-semibold">Tersedia</strong></span>
        </span>
      </div>

      <!-- Real-time Speed readout -->
      <div class="flex items-baseline gap-2 mt-1">
        <h2 class="font-sans font-bold text-3xl sm:text-4xl text-[#dee2ee] tracking-tight font-mono">
          {speedDisplay.value}
        </h2>
        <span class="font-sans text-base sm:text-lg text-[#4cd7f6] font-semibold">
          {speedDisplay.unit}
        </span>
        <span class="ml-2 text-[#8c909f] font-sans text-xs flex items-center gap-1">
          <ArrowUp class="w-3.5 h-3.5 text-[#4edea3]" />
          Turbo Multi-Part
        </span>
      </div>

      <p class="font-sans text-xs text-[#8c909f] line-clamp-1">
        Mesin multi-thread mengoptimalkan pembagian berkas secara paralel. Bebas lag tanpa membebani browsing.
      </p>
    </div>

    <!-- Center: Wave Fluctuation Graphic (SVG) -->
    <div class="w-full lg:w-64 flex flex-col gap-1 bg-[#090e16]/80 p-2.5 rounded-lg border border-[#252a33] backdrop-blur-md">
      <div class="flex items-center justify-between text-[11px] text-[#8c909f]">
        <span class="flex items-center gap-1">
          <Activity class="w-3 h-3 text-[#4cd7f6]" />
          Fluktuasi 60 Detik
        </span>
        <span class="text-[#4cd7f6] font-mono text-[10px] font-semibold">
          {totalSpeed > 0 ? 'Aktif (32 Node)' : 'Siap'}
        </span>
      </div>
      <div class="h-10 w-full flex items-end">
        <svg class="w-full h-full text-[#4cd7f6]" fill="none" preserveAspectRatio="none" viewBox="0 0 240 60">
          <defs>
            <linearGradient id="waveGrad" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stop-color="currentColor" stop-opacity="0.38"></stop>
              <stop offset="100%" stop-color="currentColor" stop-opacity="0.0"></stop>
            </linearGradient>
          </defs>
          <path d="M0,45 C20,40 35,50 55,30 C75,12 90,38 115,22 C140,8 160,25 185,15 C210,5 225,20 240,12 L240,60 L0,60 Z" fill="url(#waveGrad)"></path>
          <path d="M0,45 C20,40 35,50 55,30 C75,12 90,38 115,22 C140,8 160,25 185,15 C210,5 225,20 240,12" stroke="currentColor" stroke-linecap="round" stroke-width="2" vector-effect="non-scaling-stroke"></path>
        </svg>
      </div>
    </div>

    <!-- Right: Quick Limiter / Priority Controls -->
    <div class="flex flex-col gap-1 w-full sm:w-auto min-w-[190px] bg-[#1b2028] p-2.5 rounded-lg border border-[#252a33]">
      <span class="font-sans text-[10px] text-[#8c909f] uppercase tracking-wider font-semibold">
        Batas Kecepatan
      </span>
      <button
        onclick={() => (store.speedLimiterEnabled = !store.speedLimiterEnabled)}
        class="w-full h-7 px-2.5 rounded bg-[#252a33] text-[#dee2ee] font-sans text-xs flex items-center justify-between hover:bg-[#343942] transition-colors cursor-pointer border border-[#30353e]"
        type="button"
      >
        <span class="flex items-center gap-1.5 truncate">
          <Zap class="w-3 h-3 text-[#4edea3]" />
          {store.speedLimiterEnabled ? 'Dibatasi (Hemat Kuota)' : 'Tak Terbatas (Maksimal)'}
        </span>
      </button>
      <span class="font-sans text-[11px] text-[#8c909f] flex items-center gap-1 mt-0.5">
        <ShieldCheck class="w-3.5 h-3.5 text-[#4edea3]" />
        Prioritas Bandwidth Penuh
      </span>
    </div>
  </div>

  <!-- Bottom Mini Telemetry Badges -->
  <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 mt-3 pt-3 border-t border-[#252a33]/80">
    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#4cd7f6]/10 flex items-center justify-center text-[#4cd7f6]">
        <Download class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">Sedang Mengunduh</span>
        <span class="font-sans text-xs font-bold text-[#dee2ee]">{counts.downloading || 0} Berkas</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#252a33] flex items-center justify-center text-[#8c909f]">
        <PauseCircle class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">Dijeda</span>
        <span class="font-sans text-xs font-bold text-[#dee2ee]">{counts.paused || 0} Berkas</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#10b981]/15 flex items-center justify-center text-[#4edea3]">
        <CheckCircle2 class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">Selesai</span>
        <span class="font-sans text-xs font-bold text-[#4edea3]">{counts.completed || 0} Berkas</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#4d8eff]/15 flex items-center justify-center text-[#adc6ff]">
        <Database class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">Total Volume Unduhan</span>
        <span class="font-sans text-xs font-bold text-[#dee2ee]">{formatBytes(totalVolumeBytes)}</span>
      </div>
    </div>
  </div>
</section>
