<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatSpeed, formatDisplayVersion } from '$lib/types';
  import {
    Plus,
    Play,
    Pause,
    Trash2,
    FolderOpen,
    FileText,
    Settings,
    Search,
    Gauge,
    CheckCheck,
    Zap,
    Download
  } from '@lucide/svelte';

  const selected = $derived(store.selectedTask);
  const totalSpeed = $derived(store.totalSpeedBps);
</script>

<header class="w-full shrink-0 select-none bg-[#171c24]/95 backdrop-blur-xl border-b border-[#30353e]/80 shadow-[0_2px_12px_rgba(0,0,0,0.35)] z-30">
  <!-- Tier 1: Window Titlebar & Quick Search -->
  <div class="h-10 px-4 flex items-center justify-between border-b border-[#252a33]/80">
    <!-- Brand / Logo -->
    <div class="flex items-center gap-2.5">
      <img
        src="/favicon.png"
        alt="IDM Turbo"
        class="w-6 h-6 rounded-lg object-contain shadow-[0_0_12px_rgba(0,229,255,0.35)]"
      />
      <span class="font-bold text-xs sm:text-sm tracking-tight text-[#dee2ee]">
        IDM Turbo Desktop
      </span>
      <span class="px-1.5 py-0.5 rounded bg-[#252a33] text-[#4cd7f6] font-mono text-[9px] uppercase tracking-wider font-semibold">
        {formatDisplayVersion(store.appVersion)}
      </span>
    </div>

    <!-- Center Search Input -->
    <div class="flex items-center gap-2 flex-1 max-w-sm mx-4">
      <div class="relative w-full flex items-center">
        <Search class="w-3.5 h-3.5 absolute left-2.5 text-[#8c909f]" />
        <input
          type="text"
          bind:value={store.searchQuery}
          placeholder={store.t('toolbar.searchPlaceholder')}
          class="w-full h-7 pl-8 pr-3 bg-[#090e16] text-[#dee2ee] font-sans text-xs rounded-lg focus:outline-none focus:ring-1 focus:ring-[#4cd7f6] placeholder-[#8c909f] transition-all border border-[#252a33]"
        />
      </div>
    </div>

    <!-- Right Telemetry -->
    <div class="flex items-center gap-2">
      {#if totalSpeed > 0}
        <div class="flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[#10b981]/15 border border-[#10b981]/30 text-[#4edea3] text-[11px] font-mono">
          <Zap class="w-3 h-3 text-[#4edea3]" />
          <span class="font-semibold">{formatSpeed(totalSpeed)}</span>
        </div>
      {/if}
    </div>
  </div>

  <!-- Tier 2: IDM Operational Actions Toolbar -->
  <div class="h-11 px-4 bg-[#1b2028]/90 flex items-center justify-between gap-2 overflow-x-auto">
    <!-- Left Action Group -->
    <div class="flex items-center gap-1.5 shrink-0">
      <!-- + Tambah URL -->
      <button
        onclick={() => store.openAddModal()}
        class="h-7 px-3 rounded-lg bg-[#4d8eff] hover:bg-[#3b82f6] text-white font-sans text-xs font-semibold flex items-center gap-1.5 shadow-[0_0_14px_-2px_rgba(77,142,255,0.45)] active:scale-95 transition-all cursor-pointer"
      >
        <Plus class="w-3.5 h-3.5 stroke-[2.5]" />
        <span>{store.t('toolbar.addUrl')}</span>
      </button>

      <div class="h-4 w-[1px] bg-[#30353e] mx-1"></div>

      <!-- Mulai Semua -->
      <button
        onclick={() => store.resumeAll()}
        class="h-7 px-2.5 rounded-lg bg-[#252a33] text-[#dee2ee] hover:bg-[#343942] hover:text-[#4edea3] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
        title={store.t('toolbar.resumeAll')}
      >
        <Play class="w-3 h-3 text-[#4edea3] fill-[#4edea3]" />
        <span class="hidden sm:inline">{store.t('toolbar.resumeAll')}</span>
      </button>

      <!-- Jeda Semua -->
      <button
        onclick={() => store.pauseAll()}
        class="h-7 px-2.5 rounded-lg bg-[#252a33] text-[#dee2ee] hover:bg-[#343942] hover:text-[#ffb4ab] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
        title={store.t('toolbar.pauseAll')}
      >
        <Pause class="w-3 h-3 text-[#ffb4ab] fill-[#ffb4ab]" />
        <span class="hidden sm:inline">{store.t('toolbar.pauseAll')}</span>
      </button>

      <!-- Bersihkan Selesai -->
      <button
        onclick={() => store.clearCompleted()}
        class="h-7 px-2.5 rounded-lg bg-[#252a33] text-[#dee2ee] hover:bg-[#343942] hover:text-[#dee2ee] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
        title={store.t('toolbar.clearCompleted')}
      >
        <CheckCheck class="w-3 h-3 text-[#8c909f]" />
        <span class="hidden md:inline">{store.t('toolbar.clearCompleted')}</span>
      </button>

      <div class="h-4 w-[1px] bg-[#30353e] mx-1"></div>

      <!-- Batasi Kecepatan Toggle -->
      <button
        type="button"
        onclick={() => store.setGlobalSpeedLimit(!store.speedLimiterEnabled)}
        class="h-7 px-2.5 rounded-lg {store.speedLimiterEnabled ? 'bg-[#00e5ff]/15 text-[#00e5ff] border border-[#00e5ff]/30 shadow-[0_0_10px_rgba(0,229,255,0.2)]' : 'bg-[#252a33] text-[#dee2ee] hover:bg-[#343942]'} text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
        title={store.t('toolbar.speedLimiter')}
      >
        <Gauge class="w-3 h-3 {store.speedLimiterEnabled ? 'text-[#00e5ff]' : 'text-[#8c909f]'}" />
        <span class="hidden lg:inline">
          {store.speedLimiterEnabled ? store.t('toolbar.speedLimited', { value: `${store.globalSpeedLimitValue} ${store.globalSpeedLimitUnit}` }) : store.t('toolbar.speedLimiter')}
        </span>
      </button>

      <!-- Selected Task Actions (if selected) -->
      {#if selected}
        <div class="h-4 w-[1px] bg-[#30353e] mx-1"></div>

        <button
          onclick={() => store.openTransferWindow(selected.id)}
          class="h-7 px-2.5 rounded-lg bg-[#252a33] text-[#4cd7f6] hover:bg-[#343942] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
          title={store.t('transfer.windowTitle')}
        >
          <Zap class="w-3 h-3 text-[#4cd7f6]" />
          <span class="hidden xl:inline">Detail Transfer</span>
        </button>

        {#if selected.status === 'completed'}
          <button
            onclick={() => store.openFile(selected.file_path)}
            class="h-7 px-2 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4cd7f6] hover:bg-[#343942] text-xs transition-colors cursor-pointer"
            title={store.t('menu.openFile')}
          >
            <FileText class="w-3.5 h-3.5" />
          </button>
        {/if}

        <button
          onclick={() => store.openFolder(selected.file_path)}
          class="h-7 px-2 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4cd7f6] hover:bg-[#343942] text-xs transition-colors cursor-pointer"
          title={store.t('menu.openFolder')}
        >
          <FolderOpen class="w-3.5 h-3.5" />
        </button>

        <button
          onclick={() => store.cancelTask(selected.id, true)}
          class="h-7 px-2 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#ffb4ab] hover:bg-[#343942] text-xs transition-colors cursor-pointer"
          title={store.t('common.delete')}
        >
          <Trash2 class="w-3.5 h-3.5" />
        </button>
      {/if}
    </div>

    <!-- Right: Settings -->
    <div class="flex items-center gap-1.5 shrink-0">
      <button
        onclick={() => (store.isSettingsModalOpen = true)}
        class="h-7 px-2.5 rounded-lg bg-[#252a33] text-[#dee2ee] hover:bg-[#343942] hover:text-[#dee2ee] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
        title={store.t('toolbar.settings')}
      >
        <Settings class="w-3 h-3 text-[#8c909f]" />
        <span class="hidden sm:inline">{store.t('toolbar.settings')}</span>
      </button>
    </div>
  </div>
</header>
