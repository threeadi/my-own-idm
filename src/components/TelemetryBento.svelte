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
    ShieldCheck,
    ChevronUp,
    ChevronDown
  } from '@lucide/svelte';

  const totalSpeed = $derived(store.totalSpeedBps);
  const counts = $derived(store.categoryCounts);

  // Compute total downloaded volume across all tasks
  const totalVolumeBytes = $derived.by<number>(() => {
    return store.tasks.reduce((acc, t) => acc + (t.downloaded_bytes || 0), 0);
  });

  // Rolling 20-sample history for the 60-second fluctuation chart
  let speedHistory = $state<number[]>(new Array(20).fill(0));

  $effect(() => {
    const currentSpeed = totalSpeed;
    const interval = setInterval(() => {
      speedHistory = [...speedHistory.slice(1), currentSpeed];
    }, 1000);
    return () => clearInterval(interval);
  });

  const waveSvgPath = $derived.by(() => {
    const hasActivity = speedHistory.some((v) => v > 0);

    if (!hasActivity) {
      // Resting flat line near bottom
      return {
        fill: 'M0,55 L240,55 L240,60 L0,60 Z',
        stroke: 'M0,55 L240,55',
      };
    }

    const maxVal = Math.max(...speedHistory, 1024 * 100);
    const stepX = 240 / (speedHistory.length - 1);
    const points = speedHistory.map((val, idx) => {
      const x = idx * stepX;
      const ratio = Math.min(val / maxVal, 1);
      const y = 55 - ratio * 45;
      return { x, y };
    });

    let stroke = `M${points[0].x.toFixed(1)},${points[0].y.toFixed(1)}`;
    for (let i = 1; i < points.length; i++) {
      stroke += ` L${points[i].x.toFixed(1)},${points[i].y.toFixed(1)}`;
    }
    const fill = `${stroke} L240,60 L0,60 Z`;
    return { fill, stroke };
  });

  // Determine display speed and unit
  const speedDisplay = $derived.by<{ value: string; unit: string }>(() => {
    if (totalSpeed <= 0) {
      return { value: '0.0', unit: 'KB/s' };
    }
    if (totalSpeed >= 1024 * 1024) {
      return { value: (totalSpeed / (1024 * 1024)).toFixed(1), unit: store.t('bento.unitMbS') };
    }
    return { value: (totalSpeed / 1024).toFixed(0), unit: store.t('bento.unitKbS') };
  });
  let isCollapsed = $state(false);
</script>

{#if isCollapsed}
  <section class="relative overflow-hidden rounded-xl bg-[#171c24] border border-[#252a33] shadow-md px-3.5 py-2.5 shrink-0 select-none mb-3 flex items-center justify-between gap-3">
    <!-- Left: Realtime Status & Speed -->
    <div class="flex items-center gap-3 min-w-0">
      <div class="flex items-center gap-2">
        <span class="w-2 h-2 rounded-full {totalSpeed > 0 ? 'bg-[#4edea3] animate-pulse' : 'bg-[#8c909f]'}"></span>
        <div class="flex items-baseline gap-1 font-mono">
          <span class="font-bold text-base text-[#dee2ee]">{speedDisplay.value}</span>
          <span class="text-xs text-[#4cd7f6] font-medium">{speedDisplay.unit}</span>
        </div>
      </div>
      <div class="hidden sm:flex items-center gap-2.5 text-xs font-mono text-[#8c909f] border-l border-[#252a33] pl-3">
        <span>DL: <strong class="text-[#dee2ee]">{counts.downloading || 0}</strong></span>
        <span>•</span>
        <span>Paused: <strong class="text-[#dee2ee]">{counts.paused || 0}</strong></span>
        <span>•</span>
        <span>Done: <strong class="text-[#4edea3]">{counts.completed || 0}</strong></span>
        <span>•</span>
        <span>Total: <strong class="text-[#adc6ff]">{formatBytes(totalVolumeBytes)}</strong></span>
      </div>
    </div>

    <!-- Right: Quick Limiter / Expand -->
    <div class="flex items-center gap-2 shrink-0">
      {#if store.speedLimiterEnabled}
        <span class="hidden md:inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-[#00e5ff]/10 text-[#00e5ff] font-mono text-[11px] border border-[#00e5ff]/30">
          <Zap class="w-3 h-3" />
          {store.globalSpeedLimitValue} {store.globalSpeedLimitUnit}
        </span>
      {/if}
      <button
        type="button"
        onclick={() => (isCollapsed = false)}
        class="flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#4cd7f6] hover:bg-[#30353e] transition-colors cursor-pointer border border-[#30353e]"
        title={store.t('bento.expandTelemetry')}
      >
        <span class="text-[11px] font-sans font-medium">{store.t('common.expand')}</span>
        <ChevronDown class="w-3.5 h-3.5" />
      </button>
    </div>
  </section>
{:else}
<section class="relative overflow-hidden rounded-xl bg-[#171c24] border border-[#252a33] shadow-xl p-4 sm:p-5 shrink-0 select-none mb-3">
  <!-- Glowing Ambient Accents -->
  <div class="absolute -right-20 -top-20 w-72 h-72 rounded-full bg-[#4cd7f6]/5 blur-3xl pointer-events-none"></div>
  <div class="absolute -left-16 -bottom-16 w-64 h-64 rounded-full bg-[#4d8eff]/10 blur-3xl pointer-events-none"></div>

  <!-- Top Header Row: Status Pills + Collapse Toggle -->
  <div class="relative z-10 flex items-center justify-between gap-2 pb-3 mb-3 border-b border-[#252a33]/70">
    <div class="flex items-center gap-2 flex-wrap">
      {#if store.activeView === 'telegram'}
        <span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[#4cd7f6]/15 text-[#4cd7f6] font-sans text-xs border border-[#4cd7f6]/30">
          <span class="w-1.5 h-1.5 rounded-full bg-[#4cd7f6] animate-pulse"></span>
          Telegram MTProto Telemetry
        </span>
        <span class="inline-flex items-center gap-1 text-[#8c909f] font-mono text-xs">
          <HardDrive class="w-3.5 h-3.5 text-[#4cd7f6]" />
          <span>Channel: <strong class="text-[#4cd7f6] font-sans font-semibold">{store.telegramActiveChatInput}</strong></span>
        </span>
      {:else}
        <span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[#10b981]/15 text-[#4edea3] font-sans text-xs border border-[#10b981]/30">
          <span class="w-1.5 h-1.5 rounded-full bg-[#4edea3] animate-pulse"></span>
          {store.t('bento.connStable')}
        </span>
        <span class="inline-flex items-center gap-1 text-[#8c909f] font-mono text-xs">
          <HardDrive class="w-3.5 h-3.5 text-[#4cd7f6]" />
          <span>{store.t('bento.driveSpace')} <strong class="text-[#dee2ee] font-sans font-semibold">{store.t('bento.driveAvailable')}</strong></span>
        </span>
      {/if}
    </div>

    <!-- Integrated Collapse Toggle -->
    <button
      type="button"
      onclick={() => (isCollapsed = true)}
      class="flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#4cd7f6] hover:bg-[#30353e] transition-colors cursor-pointer border border-[#30353e] shrink-0"
      title={store.t('bento.collapseTelemetry')}
    >
      <span class="text-[11px] font-sans font-medium">{store.t('common.collapse')}</span>
      <ChevronUp class="w-3.5 h-3.5" />
    </button>
  </div>

  <div class="relative z-10 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-4">
    <!-- Left: Telemetry Highlights -->
    <div class="flex flex-col gap-1 max-w-xl">

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
          {store.activeView === 'telegram' ? 'MTProto 24 Stream Pool' : store.t('bento.turboMultiPart')}
        </span>
      </div>

      <p class="font-sans text-xs text-[#8c909f] line-clamp-1">
        {store.activeView === 'telegram' ? 'Integrasi MTProto Client Telethon dengan engine unduh multi-thread IDM Turbo (Bypass Batas 4GB Telegram Premium)' : store.t('bento.engineDescription')}
      </p>
    </div>

    <!-- Center: Wave Fluctuation Graphic (SVG) -->
    <div class="w-full lg:w-64 flex flex-col gap-1 bg-[#090e16]/80 p-2.5 rounded-lg border border-[#252a33] backdrop-blur-md">
      <div class="flex items-center justify-between text-[11px] text-[#8c909f]">
        <span class="flex items-center gap-1">
          <Activity class="w-3 h-3 text-[#4cd7f6]" />
          {store.t('bento.fluctuation60s')}
        </span>
        <span class="text-[#4cd7f6] font-mono text-[10px] font-semibold">
          {totalSpeed > 0 ? store.t('bento.activeConnections', { count: store.settings.defaultConnections }) : store.t('bento.ready')}
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
          <path d={waveSvgPath.fill} fill="url(#waveGrad)"></path>
          <path d={waveSvgPath.stroke} stroke="currentColor" stroke-linecap="round" stroke-width="2" vector-effect="non-scaling-stroke"></path>
        </svg>
      </div>
    </div>

    <!-- Right: Quick Limiter / Priority Controls -->
    <div class="flex flex-col gap-1.5 w-full sm:w-auto min-w-[220px] bg-[#1b2028] p-2.5 rounded-lg border border-[#252a33]">
      <div class="flex items-center justify-between">
        <span class="font-sans text-[10px] text-[#8c909f] uppercase tracking-wider font-semibold">
          {store.t('bento.speedLimit')}
        </span>
        <button
          type="button"
          onclick={() => store.setGlobalSpeedLimit(!store.speedLimiterEnabled)}
          class="inline-flex items-center gap-1 text-[11px] font-medium transition-colors cursor-pointer {store.speedLimiterEnabled ? 'text-[#00e5ff] font-semibold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
        >
          <span class="w-1.5 h-1.5 rounded-full {store.speedLimiterEnabled ? 'bg-[#00e5ff] shadow-[0_0_8px_#00e5ff]' : 'bg-[#30353e]'}"></span>
          {store.speedLimiterEnabled ? store.t('common.active') : store.t('common.off')}
        </button>
      </div>

      {#if store.speedLimiterEnabled}
        <!-- Input row with unit selector -->
        <div class="flex items-center gap-1.5 mt-0.5">
          <input
            type="number"
            min="1"
            step="any"
            value={store.globalSpeedLimitValue}
            oninput={(e) => {
              const val = parseFloat((e.target as HTMLInputElement).value);
              if (!isNaN(val) && val > 0) store.setGlobalSpeedLimit(true, val);
            }}
            class="w-20 h-7 px-2 bg-[#090e16] text-[#dee2ee] font-mono text-xs font-semibold rounded border border-[#30353e] focus:border-[#00e5ff] focus:outline-none"
            placeholder="1"
          />

          <!-- Unit Selector Toggle -->
          <div class="flex rounded bg-[#090e16] p-0.5 border border-[#30353e]">
            <button
              type="button"
              onclick={() => store.setGlobalSpeedLimit(true, undefined, 'KB/s')}
              class="px-2 py-0.5 text-[10px] font-mono font-medium rounded transition-colors {store.globalSpeedLimitUnit === 'KB/s' ? 'bg-[#00e5ff] text-slate-950 font-bold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
            >
              KB/s
            </button>
            <button
              type="button"
              onclick={() => store.setGlobalSpeedLimit(true, undefined, 'MB/s')}
              class="px-2 py-0.5 text-[10px] font-mono font-medium rounded transition-colors {store.globalSpeedLimitUnit === 'MB/s' ? 'bg-[#00e5ff] text-slate-950 font-bold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
            >
              MB/s
            </button>
          </div>
        </div>

        <!-- Quick Presets -->
        <div class="flex items-center gap-1 mt-0.5 flex-wrap">
          <button
            type="button"
            onclick={() => store.setGlobalSpeedLimit(true, 500, 'KB/s')}
            class="px-1.5 py-0.5 rounded bg-[#252a33] hover:bg-[#30353e] text-[10px] font-mono text-[#8c909f] hover:text-[#4cd7f6] transition-colors"
          >
            500K
          </button>
          <button
            type="button"
            onclick={() => store.setGlobalSpeedLimit(true, 1, 'MB/s')}
            class="px-1.5 py-0.5 rounded bg-[#252a33] hover:bg-[#30353e] text-[10px] font-mono text-[#8c909f] hover:text-[#4cd7f6] transition-colors"
          >
            1M
          </button>
          <button
            type="button"
            onclick={() => store.setGlobalSpeedLimit(true, 2, 'MB/s')}
            class="px-1.5 py-0.5 rounded bg-[#252a33] hover:bg-[#30353e] text-[10px] font-mono text-[#8c909f] hover:text-[#4cd7f6] transition-colors"
          >
            2M
          </button>
          <button
            type="button"
            onclick={() => store.setGlobalSpeedLimit(true, 5, 'MB/s')}
            class="px-1.5 py-0.5 rounded bg-[#252a33] hover:bg-[#30353e] text-[10px] font-mono text-[#8c909f] hover:text-[#4cd7f6] transition-colors"
          >
            5M
          </button>
        </div>
      {:else}
        <button
          onclick={() => store.setGlobalSpeedLimit(true)}
          class="w-full h-7 px-2.5 rounded bg-[#252a33] text-[#dee2ee] font-sans text-xs flex items-center justify-between hover:bg-[#343942] transition-colors cursor-pointer border border-[#30353e]"
          type="button"
        >
          <span class="flex items-center gap-1.5 truncate">
            <Zap class="w-3 h-3 text-[#4edea3]" />
            {store.t('bento.unlimitedMax')}
          </span>
          <span class="text-[10px] text-[#4cd7f6] font-medium">{store.t('bento.limitBtn')}</span>
        </button>
        <span class="font-sans text-[11px] text-[#8c909f] flex items-center gap-1 mt-0.5">
          <ShieldCheck class="w-3.5 h-3.5 text-[#4edea3]" />
          {store.t('bento.fullBandwidth')}
        </span>
      {/if}
    </div>
  </div>

  <!-- Bottom Mini Telemetry Badges -->
  <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 mt-3 pt-3 border-t border-[#252a33]/80">
    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#4cd7f6]/10 flex items-center justify-center text-[#4cd7f6]">
        <Download class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">{store.t('sidebar.statusDownloading')}</span>
        <span class="font-sans text-xs font-bold text-[#dee2ee]">{store.t('table.filesCount', { count: counts.downloading || 0 })}</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#252a33] flex items-center justify-center text-[#8c909f]">
        <PauseCircle class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">{store.t('sidebar.statusPaused')}</span>
        <span class="font-sans text-xs font-bold text-[#dee2ee]">{store.t('table.filesCount', { count: counts.paused || 0 })}</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#10b981]/15 flex items-center justify-center text-[#4edea3]">
        <CheckCircle2 class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">{store.t('sidebar.statusCompleted')}</span>
        <span class="font-sans text-xs font-bold text-[#4edea3]">{store.t('table.filesCount', { count: counts.completed || 0 })}</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="w-7 h-7 rounded-lg bg-[#4d8eff]/15 flex items-center justify-center text-[#adc6ff]">
        <Database class="w-3.5 h-3.5" />
      </div>
      <div class="flex flex-col">
        <span class="font-sans text-[10px] text-[#8c909f]">{store.t('bento.totalVolume')}</span>
        <span class="font-sans text-xs font-bold text-[#dee2ee]">{formatBytes(totalVolumeBytes)}</span>
      </div>
    </div>
  </div>
</section>
{/if}
