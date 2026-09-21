<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { formatDisplayVersion, type AppSettings } from '$lib/types';
  import {
    X,
    Minus,
    Sliders,
    Monitor,
    Zap,
    FileText,
    FolderOpen,
    Globe,
    Check,
    RotateCcw,
    Folder,
    HardDrive,
    ShieldCheck,
    ExternalLink,
    RefreshCw,
    Layers,
    Clock,
    Flame
  } from '@lucide/svelte';

  type SettingsTab = 'general' | 'connection' | 'filetypes' | 'saveto' | 'extensions';
  let activeTab = $state<SettingsTab>('general');

  // Draft local copy of settings to allow Apply, OK, Cancel, and Reset
  let draft = $state<AppSettings>({ ...store.settings });
  let isSaving = $state(false);
  let saveSuccessMessage = $state<string | null>(null);
  let nativeHostRegisterStatus = $state<string | null>(null);

  $effect(() => {
    if (store.isSettingsModalOpen) {
      draft = { ...store.settings };
      saveSuccessMessage = null;
      nativeHostRegisterStatus = null;
    }
  });

  async function browseFolder(target: 'default' | 'temp') {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: (target === 'default' ? draft.defaultDownloadDir : draft.tempDir) || undefined,
        title: target === 'default' ? store.t('addModal.selectFolder') : store.t('settings.tempDirTitle'),
      });
      if (selected && typeof selected === 'string') {
        if (target === 'default') {
          draft.defaultDownloadDir = selected;
        } else {
          draft.tempDir = selected;
        }
      }
    } catch (e) {
      console.error('Folder selection error:', e);
    }
  }

  async function handleApply() {
    isSaving = true;
    try {
      await store.saveAppSettings(draft);
      saveSuccessMessage = store.t('settings.msgApplied');
      setTimeout(() => {
        saveSuccessMessage = null;
      }, 3000);
    } finally {
      isSaving = false;
    }
  }

  async function handleOk() {
    isSaving = true;
    try {
      await store.saveAppSettings(draft);
      store.isSettingsModalOpen = false;
    } finally {
      isSaving = false;
    }
  }

  function handleCancel() {
    store.isSettingsModalOpen = false;
  }

  async function handleResetDefaults() {
    if (confirm(store.t('settings.confirmReset'))) {
      await store.resetAppSettings();
      draft = { ...store.settings };
      saveSuccessMessage = store.t('settings.msgReset');
      setTimeout(() => {
        saveSuccessMessage = null;
      }, 3000);
    }
  }

  async function registerNativeHost() {
    nativeHostRegisterStatus = store.t('common.processing');
    try {
      await invoke('register_native_host_manifest');
      nativeHostRegisterStatus = store.t('settings.manifestRegistered');
      setTimeout(() => {
        nativeHostRegisterStatus = null;
      }, 4000);
    } catch (e) {
      nativeHostRegisterStatus = `${store.t('common.error')}: ${e}`;
      setTimeout(() => {
        nativeHostRegisterStatus = null;
      }, 4000);
    }
  }

  const connectionSpeeds = $derived([
    { label: store.t('settings.speedBroadband'), value: 'broadband' },
    { label: store.t('settings.speedStandard'), value: 'standard' },
    { label: store.t('settings.speedMetered'), value: 'metered' },
  ]);
</script>

{#if store.isSettingsModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/50 backdrop-blur-[3px] select-none animate-in fade-in duration-150">
    <!-- Acrylic Glass Container -->
    <div
      class="relative w-full max-w-4xl max-h-[92vh] bg-[#171c24]/95 backdrop-blur-2xl rounded-xl shadow-2xl border border-[#30353e] overflow-hidden flex flex-col transition-all duration-200"
      id="settings-options-modal"
    >
      <!-- Top Cyan Shimmer Bar -->
      <div class="absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r from-transparent via-[#00e5ff]/70 to-transparent"></div>

      <!-- Window Title Bar -->
      <div data-tauri-drag-region class="px-4 py-2.5 bg-[#252a33]/90 flex items-center justify-between border-b border-[#30353e]/80 cursor-move">
        <div class="flex items-center gap-2">
          <img
            src="/favicon.png"
            alt="IDM Turbo"
            class="w-5 h-5 rounded-md object-contain shadow-[0_0_8px_rgba(0,229,255,0.3)]"
          />
          <span class="font-sans text-xs sm:text-sm font-semibold text-[#dee2ee]">
            {store.t('settings.title')}
          </span>
        </div>
        <div class="flex items-center gap-1">
          <button
            onclick={() => (store.isSettingsModalOpen = false)}
            class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:bg-[#30353e] hover:text-[#dee2ee] transition-colors cursor-pointer"
            title={store.t('common.minimize')}
            type="button"
          >
            <Minus class="w-3.5 h-3.5" />
          </button>
          <button
            onclick={handleCancel}
            class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:bg-[#93000a] hover:text-white transition-colors cursor-pointer"
            title={store.t('common.close')}
            type="button"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <!-- Subheader with Badge & Status -->
      <div class="px-6 py-3.5 bg-[#171c24] border-b border-[#252a33] flex flex-wrap items-center justify-between gap-3">
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-lg bg-[#090e16] border border-[#30353e] flex items-center justify-center text-[#00e5ff] shadow-[0_0_12px_rgba(0,229,255,0.2)]">
            <Sliders class="w-4 h-4" />
          </div>
          <div>
            <h2 class="text-sm font-bold text-[#dee2ee] tracking-tight">{store.t('settings.configOptions')}</h2>
            <p class="text-[11px] text-[#8c909f]">{store.t('settings.configSub')}</p>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <span class="px-2 py-0.5 rounded bg-[#090e16] text-[#4edea3] font-mono text-[11px] font-semibold flex items-center gap-1.5 border border-[#4edea3]/20 shadow-[0_0_8px_rgba(78,222,163,0.15)]">
            <span class="w-1.5 h-1.5 rounded-full bg-[#4edea3] animate-pulse"></span>
            {store.t('settings.activeStatus', { version: formatDisplayVersion(store.appVersion) })}
          </span>
          <span class="px-2 py-0.5 rounded bg-[#252a33] text-[#bac9cc] font-mono text-[11px] border border-[#30353e]">
            Build 2026.09
          </span>
        </div>
      </div>

      <!-- 5-Tab Bar -->
      <div class="px-6 pt-3 bg-[#11161f] border-b border-[#252a33] flex items-center gap-2 overflow-x-auto no-scrollbar">
        <button
          type="button"
          onclick={() => (activeTab = 'general')}
          class={`px-3.5 py-2 rounded-t-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === 'general'
              ? 'bg-[#171c24] text-[#00e5ff] border-t-2 border-t-[#00e5ff] border-x border-[#30353e] shadow-[0_-2px_8px_rgba(0,229,255,0.1)]'
              : 'text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#171c24]/50'
          }`}
        >
          <Monitor class="w-3.5 h-3.5" />
          <span>{store.t('settings.tabGeneral')}</span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = 'connection')}
          class={`px-3.5 py-2 rounded-t-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === 'connection'
              ? 'bg-[#171c24] text-[#00e5ff] border-t-2 border-t-[#00e5ff] border-x border-[#30353e] shadow-[0_-2px_8px_rgba(0,229,255,0.1)]'
              : 'text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#171c24]/50'
          }`}
        >
          <Zap class="w-3.5 h-3.5" />
          <span>{store.t('settings.tabConnection')}</span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = 'filetypes')}
          class={`px-3.5 py-2 rounded-t-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === 'filetypes'
              ? 'bg-[#171c24] text-[#00e5ff] border-t-2 border-t-[#00e5ff] border-x border-[#30353e] shadow-[0_-2px_8px_rgba(0,229,255,0.1)]'
              : 'text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#171c24]/50'
          }`}
        >
          <FileText class="w-3.5 h-3.5" />
          <span>{store.t('settings.tabFileTypes')}</span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = 'saveto')}
          class={`px-3.5 py-2 rounded-t-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === 'saveto'
              ? 'bg-[#171c24] text-[#00e5ff] border-t-2 border-t-[#00e5ff] border-x border-[#30353e] shadow-[0_-2px_8px_rgba(0,229,255,0.1)]'
              : 'text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#171c24]/50'
          }`}
        >
          <FolderOpen class="w-3.5 h-3.5" />
          <span>{store.t('settings.tabSaveTo')}</span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = 'extensions')}
          class={`px-3.5 py-2 rounded-t-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === 'extensions'
              ? 'bg-[#171c24] text-[#00e5ff] border-t-2 border-t-[#00e5ff] border-x border-[#30353e] shadow-[0_-2px_8px_rgba(0,229,255,0.1)]'
              : 'text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#171c24]/50'
          }`}
        >
          <Globe class="w-3.5 h-3.5" />
          <span>{store.t('settings.tabExtensions')}</span>
        </button>
      </div>

      <!-- Tab Content (Scrollable Viewport) -->
      <div class="p-6 overflow-y-auto flex-1 text-xs space-y-6 max-h-[60vh]">
        <!-- Success Alert Bar -->
        {#if saveSuccessMessage}
          <div class="p-3 bg-[#00e5ff]/10 border border-[#00e5ff]/30 text-[#00e5ff] rounded-lg flex items-center gap-2 font-medium">
            <Check class="w-4 h-4 shrink-0" />
            <span>{saveSuccessMessage}</span>
          </div>
        {/if}

        <!-- TAB 1: UMUM (GENERAL) -->
        {#if activeTab === 'general'}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- Left: Bahasa & Integrasi Sistem -->
            <div class="space-y-6">
              <!-- Pilihan Bahasa / Language Selection -->
              <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-3">
                <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                  <Globe class="w-4 h-4 text-[#00e5ff]" />
                  <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.languageTitle')}</h3>
                </div>
                <p class="text-[11px] text-[#8c909f] leading-snug">
                  {store.t('settings.languageDesc')}
                </p>
                <div class="pt-1">
                  <select
                    bind:value={draft.language}
                    onchange={() => {
                      store.settings.language = draft.language;
                    }}
                    class="w-full bg-[#090e16] text-[#dee2ee] text-xs rounded-lg px-3 py-2 border border-[#30353e] focus:outline-none focus:border-[#00e5ff] cursor-pointer [&>option]:bg-[#171c24] [&>option]:text-[#dee2ee]"
                  >
                    <option value="id">🇮🇩 Bahasa Indonesia (Indonesian)</option>
                    <option value="en">🇺🇸 English (US / International)</option>
                  </select>
                </div>
              </div>

              <!-- Integrasi Sistem & Startup -->
              <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
                <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                  <Monitor class="w-4 h-4 text-[#00e5ff]" />
                  <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.systemStartup')}</h3>
                </div>

                <div class="space-y-3">
                  <label class="flex items-start gap-3 cursor-pointer group">
                    <input
                      type="checkbox"
                      bind:checked={draft.autoStartWindows}
                      class="mt-0.5 rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#090e16]"
                    />
                    <div>
                      <span class="text-xs font-semibold text-[#dee2ee] group-hover:text-[#00e5ff] transition-colors">
                        {store.t('settings.autoStart')}
                      </span>
                      <p class="text-[11px] text-[#8c909f] leading-snug">
                        {store.t('settings.autoStartDesc')}
                      </p>
                    </div>
                  </label>

                  <label class="flex items-start gap-3 cursor-pointer group">
                    <input
                      type="checkbox"
                      bind:checked={draft.mediaPanelOverlay}
                      class="mt-0.5 rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#090e16]"
                    />
                    <div>
                      <span class="text-xs font-semibold text-[#dee2ee] group-hover:text-[#00e5ff] transition-colors">
                        {store.t('settings.mediaPanel')}
                      </span>
                      <p class="text-[11px] text-[#8c909f] leading-snug">
                        {store.t('settings.mediaPanelDesc')}
                      </p>
                    </div>
                  </label>

                  <label class="flex items-start gap-3 cursor-pointer group">
                    <input
                      type="checkbox"
                      bind:checked={draft.clipboardAutoCapture}
                      class="mt-0.5 rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#090e16]"
                    />
                    <div>
                      <span class="text-xs font-semibold text-[#dee2ee] group-hover:text-[#00e5ff] transition-colors">
                        {store.t('settings.clipboardCapture')}
                      </span>
                      <p class="text-[11px] text-[#8c909f] leading-snug">
                        {store.t('settings.clipboardCaptureDesc')}
                      </p>
                    </div>
                  </label>

                  <label class="flex items-start gap-3 cursor-pointer group">
                    <input
                      type="checkbox"
                      bind:checked={draft.notifyOnComplete}
                      class="mt-0.5 rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#090e16]"
                    />
                    <div>
                      <span class="text-xs font-semibold text-[#dee2ee] group-hover:text-[#00e5ff] transition-colors">
                        {store.t('settings.notifyComplete')}
                      </span>
                      <p class="text-[11px] text-[#8c909f] leading-snug">
                        {store.t('settings.notifyCompleteDesc')}
                      </p>
                    </div>
                  </label>
                </div>
              </div>

              <!-- Integrasi Peramban Web -->
              <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
                <div class="flex items-center justify-between pb-2 border-b border-[#252a33]">
                  <div class="flex items-center gap-2">
                    <Globe class="w-4 h-4 text-[#00e5ff]" />
                    <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.browserTitle')}</h3>
                  </div>
                  <span class="px-1.5 py-0.5 rounded bg-[#00e5ff]/10 text-[#00e5ff] font-mono text-[10px] font-semibold border border-[#00e5ff]/30">
                    Native Messaging Host
                  </span>
                </div>

                <p class="text-[11px] text-[#8c909f]">
                  {store.t('settings.browserDesc')}
                </p>

                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <!-- Chrome -->
                  <div class="p-2.5 bg-[#090e16] border border-[#30353e] rounded-lg flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <div class="w-2 h-2 rounded-full bg-[#10b981]"></div>
                      <div>
                        <span class="font-semibold text-xs text-[#dee2ee] block">Google Chrome</span>
                        <span class="text-[10px] text-[#4edea3] font-mono">{store.t('settings.connected')}</span>
                      </div>
                    </div>
                    <input
                      type="checkbox"
                      bind:checked={draft.browserChrome}
                      class="rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#171c24]"
                    />
                  </div>

                  <!-- Edge -->
                  <div class="p-2.5 bg-[#090e16] border border-[#30353e] rounded-lg flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <div class="w-2 h-2 rounded-full bg-[#10b981]"></div>
                      <div>
                        <span class="font-semibold text-xs text-[#dee2ee] block">Microsoft Edge</span>
                        <span class="text-[10px] text-[#4edea3] font-mono">{store.t('settings.connected')}</span>
                      </div>
                    </div>
                    <input
                      type="checkbox"
                      bind:checked={draft.browserEdge}
                      class="rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#171c24]"
                    />
                  </div>

                  <!-- Firefox -->
                  <div class="p-2.5 bg-[#090e16] border border-[#30353e] rounded-lg flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <div class="w-2 h-2 rounded-full bg-[#10b981]"></div>
                      <div>
                        <span class="font-semibold text-xs text-[#dee2ee] block">Mozilla Firefox</span>
                        <span class="text-[10px] text-[#4edea3] font-mono">{store.t('settings.installedActive')}</span>
                      </div>
                    </div>
                    <input
                      type="checkbox"
                      bind:checked={draft.browserFirefox}
                      class="rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#171c24]"
                    />
                  </div>

                  <!-- Brave -->
                  <div class="p-2.5 bg-[#090e16] border border-[#30353e] rounded-lg flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <div class="w-2 h-2 rounded-full bg-[#10b981]"></div>
                      <div>
                        <span class="font-semibold text-xs text-[#dee2ee] block">Brave Browser</span>
                        <span class="text-[10px] text-[#4edea3] font-mono">{store.t('settings.connected')}</span>
                      </div>
                    </div>
                    <input
                      type="checkbox"
                      bind:checked={draft.browserBrave}
                      class="rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#171c24]"
                    />
                  </div>
                </div>

                <div class="flex items-center justify-between pt-1">
                  <button
                    type="button"
                    onclick={registerNativeHost}
                    class="px-2.5 py-1.5 rounded bg-[#252a33] hover:bg-[#30353e] text-[#4cd7f6] font-medium text-[11px] flex items-center gap-1.5 transition-colors cursor-pointer border border-[#4cd7f6]/20"
                  >
                    <RefreshCw class="w-3 h-3" />
                    <span>{store.t('settings.checkExtensionUpdates')}</span>
                  </button>
                  {#if nativeHostRegisterStatus}
                    <span class="text-[11px] font-mono text-[#4edea3]">{nativeHostRegisterStatus}</span>
                  {/if}
                </div>
              </div>
            </div>

            <!-- Right: Jalur Paralel & Jaringan + Kunci Tombol Cepat -->
            <div class="space-y-6">
              <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
                <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                  <Zap class="w-4 h-4 text-[#00e5ff]" />
                  <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.networkParallel')}</h3>
                </div>

                <div class="space-y-3">
                  <div>
                    <label for="connection-type-select" class="block text-xs font-semibold text-[#dee2ee] mb-1">
                      {store.t('settings.connectionTypeLabel')}
                    </label>
                    <select
                      id="connection-type-select"
                      bind:value={draft.connectionType}
                      class="w-full bg-[#090e16] border border-[#30353e] rounded-lg px-3 py-2 text-xs text-[#dee2ee] focus:outline-none focus:border-[#00e5ff]"
                    >
                      {#each connectionSpeeds as sp}
                        <option value={sp.value}>{sp.label}</option>
                      {/each}
                    </select>
                  </div>

                  <div>
                    <div class="flex items-center justify-between mb-1.5">
                      <span class="text-xs font-semibold text-[#dee2ee]">{store.t('settings.maxConnectionsLabel')}</span>
                      <span class="px-2 py-0.5 rounded bg-[#00e5ff]/15 text-[#00e5ff] font-mono text-xs font-bold border border-[#00e5ff]/30">
                        {store.t('settings.turboSuffix', { count: draft.defaultConnections })}
                      </span>
                    </div>
                    <p class="text-[11px] text-[#8c909f] mb-2">{store.t('settings.simultaneousSegmentsDesc')}</p>

                    <input
                      type="range"
                      min="4"
                      max="32"
                      step="4"
                      bind:value={draft.defaultConnections}
                      class="w-full accent-[#00e5ff] cursor-pointer"
                    />
                    <div class="flex justify-between text-[10px] font-mono text-[#8c909f] px-1 mt-1">
                      <span>4</span>
                      <span>8</span>
                      <span class="text-[#00e5ff] font-bold">{store.t('settings.turboPill')}</span>
                      <span>24</span>
                      <span>32</span>
                    </div>
                  </div>

                  <label class="flex items-start gap-3 cursor-pointer group pt-1">
                    <input
                      type="checkbox"
                      bind:checked={draft.tcpWindowAutoTuning}
                      class="mt-0.5 rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#090e16]"
                    />
                    <div>
                      <span class="text-xs font-semibold text-[#dee2ee] group-hover:text-[#00e5ff] transition-colors">
                        {store.t('settings.tcpTuningTitle')}
                      </span>
                      <p class="text-[11px] text-[#8c909f] leading-snug">
                        {store.t('settings.tcpTuningSub')}
                      </p>
                    </div>
                  </label>

                  <div class="p-2.5 rounded-lg bg-[#090e16] border border-[#30353e] flex items-center justify-between">
                    <span class="text-xs text-[#8c909f]">{store.t('settings.peakLimitEstimate')}</span>
                    <span class="text-xs font-mono font-bold text-[#4edea3]">{store.t('settings.uncapped')}</span>
                  </div>
                </div>
              </div>

              <!-- Kunci Tombol Cepat -->
              <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-3">
                <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                  <Layers class="w-4 h-4 text-[#00e5ff]" />
                  <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.hotkeysTitle')}</h3>
                </div>

                <div class="space-y-2">
                  <div class="flex items-center justify-between p-2 rounded-lg bg-[#090e16] border border-[#252a33]">
                    <span class="text-xs text-[#dee2ee]">{store.t('settings.preventDownload')}</span>
                    <kbd class="px-2 py-0.5 rounded bg-[#252a33] text-[#4cd7f6] font-mono text-[11px] font-bold border border-[#30353e]">
                      Alt
                    </kbd>
                  </div>
                  <div class="flex items-center justify-between p-2 rounded-lg bg-[#090e16] border border-[#252a33]">
                    <span class="text-xs text-[#dee2ee]">{store.t('settings.forceCapture')}</span>
                    <kbd class="px-2 py-0.5 rounded bg-[#252a33] text-[#4cd7f6] font-mono text-[11px] font-bold border border-[#30353e]">
                      Insert
                    </kbd>
                  </div>
                </div>
              </div>
            </div>
          </div>
        {/if}

        <!-- TAB 2: SAMBUNGAN (CONNECTION) -->
        {#if activeTab === 'connection'}
          <div class="space-y-6 max-w-2xl">
            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
              <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                <Zap class="w-4 h-4 text-[#00e5ff]" />
                <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.bandwidthLimiterTitle')}</h3>
              </div>

              <div class="space-y-3">
                <div class="flex items-center justify-between">
                  <div>
                    <span class="text-xs font-semibold text-[#dee2ee]">{store.t('settings.enableGlobalLimiter')}</span>
                    <p class="text-[11px] text-[#8c909f]">{store.t('settings.enableGlobalLimiterDesc')}</p>
                  </div>
                  <input
                    type="checkbox"
                    bind:checked={store.speedLimiterEnabled}
                    class="rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#090e16]"
                  />
                </div>

                {#if store.speedLimiterEnabled}
                  <div class="flex items-center gap-3 p-3 bg-[#090e16] rounded-lg border border-[#30353e]">
                    <label for="speed-limit-val" class="text-xs font-medium text-[#dee2ee]">{store.t('settings.maxLimitLabel')}</label>
                    <input
                      id="speed-limit-val"
                      type="number"
                      min="1"
                      bind:value={store.globalSpeedLimitValue}
                      class="w-24 bg-[#171c24] border border-[#30353e] rounded px-2.5 py-1 text-xs text-[#dee2ee] focus:outline-none focus:border-[#00e5ff]"
                    />
                    <select
                      bind:value={store.globalSpeedLimitUnit}
                      class="bg-[#171c24] border border-[#30353e] rounded px-2.5 py-1 text-xs text-[#dee2ee] focus:outline-none focus:border-[#00e5ff]"
                    >
                      <option value="KB/s">KB/s</option>
                      <option value="MB/s">MB/s</option>
                    </select>
                  </div>
                {/if}
              </div>
            </div>

            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
              <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                <Clock class="w-4 h-4 text-[#00e5ff]" />
                <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.networkTimeoutTitle')}</h3>
              </div>

              <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <label for="timeout-input" class="block text-xs font-semibold text-[#dee2ee] mb-1">
                    {store.t('settings.connTimeoutSeconds')}
                  </label>
                  <input
                    id="timeout-input"
                    type="number"
                    min="5"
                    max="300"
                    bind:value={draft.connectionTimeoutSec}
                    class="w-full bg-[#090e16] border border-[#30353e] rounded-lg px-3 py-2 text-xs text-[#dee2ee] focus:outline-none focus:border-[#00e5ff]"
                  />
                  <p class="text-[10px] text-[#8c909f] mt-1">{store.t('settings.connTimeoutDesc')}</p>
                </div>

                <div>
                  <label for="retries-input" class="block text-xs font-semibold text-[#dee2ee] mb-1">
                    {store.t('settings.maxAutoRetries')}
                  </label>
                  <input
                    id="retries-input"
                    type="number"
                    min="0"
                    max="50"
                    bind:value={draft.maxRetries}
                    class="w-full bg-[#090e16] border border-[#30353e] rounded-lg px-3 py-2 text-xs text-[#dee2ee] focus:outline-none focus:border-[#00e5ff]"
                  />
                  <p class="text-[10px] text-[#8c909f] mt-1">{store.t('settings.maxAutoRetriesDesc')}</p>
                </div>
              </div>
            </div>
          </div>
        {/if}

        <!-- TAB 3: JENIS BERKAS (FILE TYPES) -->
        {#if activeTab === 'filetypes'}
          <div class="space-y-6 max-w-2xl">
            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-3">
              <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                <FileText class="w-4 h-4 text-[#00e5ff]" />
                <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.autoCapturedExtTitle')}</h3>
              </div>
              <p class="text-[11px] text-[#8c909f]">
                {store.t('settings.autoCapturedExtDesc')}
              </p>

              <textarea
                rows="4"
                bind:value={draft.autoCaptureExtensions}
                class="w-full bg-[#090e16] border border-[#30353e] rounded-lg p-3 text-xs font-mono text-[#00e5ff] focus:outline-none focus:border-[#00e5ff] leading-relaxed resize-y"
                placeholder="ZIP RAR 7Z EXE ISO MP4 MKV ..."
              ></textarea>
              <p class="text-[10px] text-[#8c909f]">{store.t('settings.autoCapturedExtHelp')}</p>
            </div>

            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-3">
              <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                <ShieldCheck class="w-4 h-4 text-[#ffb4ab]" />
                <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.excludedSitesTitle')}</h3>
              </div>
              <p class="text-[11px] text-[#8c909f]">
                {store.t('settings.excludedSitesDesc')}
              </p>

              <textarea
                rows="3"
                bind:value={draft.excludedSites}
                class="w-full bg-[#090e16] border border-[#30353e] rounded-lg p-3 text-xs font-mono text-[#bac9cc] focus:outline-none focus:border-[#00e5ff] leading-relaxed resize-y"
                placeholder="*.bank.co.id intranet.local example.org"
              ></textarea>
              <p class="text-[10px] text-[#8c909f]">{store.t('settings.excludedSitesHelp')}</p>
            </div>
          </div>
        {/if}

        <!-- TAB 4: LOKASI SIMPAN (SAVE TO) -->
        {#if activeTab === 'saveto'}
          <div class="space-y-6 max-w-2xl">
            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
              <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                <FolderOpen class="w-4 h-4 text-[#00e5ff]" />
                <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.mainDownloadDir')}</h3>
              </div>

              <div>
                <label for="primary-save-dir" class="block text-xs font-semibold text-[#dee2ee] mb-1">
                  {store.t('settings.defaultFolderLabel')}
                </label>
                <div class="flex gap-2">
                  <input
                    id="primary-save-dir"
                    type="text"
                    bind:value={draft.defaultDownloadDir}
                    placeholder="C:\Users\Username\Downloads"
                    class="flex-1 bg-[#090e16] border border-[#30353e] rounded-lg px-3 py-2 text-xs font-mono text-[#dee2ee] focus:outline-none focus:border-[#00e5ff]"
                  />
                  <button
                    type="button"
                    onclick={() => browseFolder('default')}
                    class="px-3 py-2 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-[#dee2ee] font-medium text-xs flex items-center gap-1.5 transition-colors cursor-pointer border border-[#30353e]"
                  >
                    <Folder class="w-3.5 h-3.5 text-[#00e5ff]" />
                    <span>{store.t('common.browse')}</span>
                  </button>
                </div>
              </div>

              <label class="flex items-start gap-3 cursor-pointer group pt-1">
                <input
                  type="checkbox"
                  bind:checked={draft.categorySubfolders}
                  class="mt-0.5 rounded border-[#30353e] text-[#00e5ff] focus:ring-0 focus:ring-offset-0 bg-[#090e16]"
                />
                <div>
                  <span class="text-xs font-semibold text-[#dee2ee] group-hover:text-[#00e5ff] transition-colors">
                    {store.t('settings.autoCategorySubfolders')}
                  </span>
                  <p class="text-[11px] text-[#8c909f] leading-snug">
                    {store.t('settings.autoCategorySubfoldersDesc')}
                  </p>
                </div>
              </label>
            </div>

            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
              <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                <HardDrive class="w-4 h-4 text-[#00e5ff]" />
                <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.tempDirTitle')}</h3>
              </div>

              <div>
                <label for="temp-save-dir" class="block text-xs font-semibold text-[#dee2ee] mb-1">
                  {store.t('settings.tempDirLabel')}
                </label>
                <div class="flex gap-2">
                  <input
                    id="temp-save-dir"
                    type="text"
                    bind:value={draft.tempDir}
                    placeholder={store.t('settings.tempDirPlaceholder')}
                    class="flex-1 bg-[#090e16] border border-[#30353e] rounded-lg px-3 py-2 text-xs font-mono text-[#dee2ee] focus:outline-none focus:border-[#00e5ff]"
                  />
                  <button
                    type="button"
                    onclick={() => browseFolder('temp')}
                    class="px-3 py-2 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-[#dee2ee] font-medium text-xs flex items-center gap-1.5 transition-colors cursor-pointer border border-[#30353e]"
                  >
                    <Folder class="w-3.5 h-3.5 text-[#00e5ff]" />
                    <span>{store.t('common.browse')}</span>
                  </button>
                </div>
                <p class="text-[10px] text-[#8c909f] mt-1.5">
                  {store.t('settings.tempDirDesc')}
                </p>
              </div>
            </div>

            <!-- Tindakan Tautan Unduhan Duplikat -->
            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
              <div class="flex items-center gap-2 pb-2 border-b border-[#252a33]">
                <Layers class="w-4 h-4 text-[#00e5ff]" />
                <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.duplicateActionTitle')}</h3>
              </div>

              <div>
                <p class="text-[11px] text-[#8c909f] mb-2.5">
                  {store.t('settings.duplicateActionDesc')}
                </p>

                <select
                  bind:value={draft.duplicateAction}
                  onchange={() => {
                    draft.duplicateActionRemember = draft.duplicateAction !== 'ask';
                  }}
                  class="w-full bg-[#090e16] text-[#dee2ee] text-xs rounded-lg px-3 py-2 border border-[#30353e] focus:outline-none focus:border-[#00e5ff] cursor-pointer [&>option]:bg-[#171c24] [&>option]:text-[#dee2ee]"
                >
                  <option value="ask">{store.t('settings.duplicateActionAsk')}</option>
                  <option value="numbered">{store.t('settings.duplicateActionNumbered')}</option>
                  <option value="overwrite">{store.t('settings.duplicateActionOverwrite')}</option>
                  <option value="resume">{store.t('settings.duplicateActionResume')}</option>
                </select>
              </div>
            </div>
          </div>
        {/if}

        <!-- TAB 5: EKSTENSI (EXTENSIONS) -->
        {#if activeTab === 'extensions'}
          <div class="space-y-6 max-w-2xl">
            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-4">
              <div class="flex items-center justify-between pb-2 border-b border-[#252a33]">
                <div class="flex items-center gap-2">
                  <Globe class="w-4 h-4 text-[#00e5ff]" />
                  <h3 class="font-bold text-xs text-[#dee2ee] uppercase tracking-wider">{store.t('settings.browserIntegrationTitle')}</h3>
                </div>
                <span class="px-2 py-0.5 rounded bg-[#10b981]/15 text-[#10b981] font-mono text-[10px] font-bold border border-[#10b981]/30">
                  {store.t('settings.ipcPipeActive')}
                </span>
              </div>

              <p class="text-[11px] text-[#8c909f] leading-relaxed">
                {store.t('settings.browserIntegrationDesc')}
              </p>

              <div class="p-3 bg-[#090e16] rounded-lg border border-[#30353e] space-y-2 text-xs font-mono">
                <div class="flex justify-between">
                  <span class="text-[#8c909f]">Host Name:</span>
                  <span class="text-[#00e5ff]">com.myownidm.host</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-[#8c909f]">Windows Named Pipe:</span>
                  <span class="text-[#dee2ee]">\\.\pipe\myownidm-ipc</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-[#8c909f]">Localhost Fallback Port:</span>
                  <span class="text-[#4edea3]">127.0.0.1:17890</span>
                </div>
              </div>

              <div class="flex items-center gap-3 pt-2">
                <button
                  type="button"
                  onclick={registerNativeHost}
                  class="px-3 py-2 rounded-lg bg-[#00e5ff]/15 hover:bg-[#00e5ff]/25 text-[#00e5ff] font-semibold text-xs flex items-center gap-2 transition-all cursor-pointer border border-[#00e5ff]/40 shadow-[0_0_12px_rgba(0,229,255,0.15)]"
                >
                  <RefreshCw class="w-3.5 h-3.5" />
                  <span>{store.t('settings.reRegisterHost')}</span>
                </button>
                {#if nativeHostRegisterStatus}
                  <span class="text-xs font-mono text-[#4edea3]">{nativeHostRegisterStatus}</span>
                {/if}
              </div>
            </div>

            <div class="bg-[#1b2028] p-4 rounded-xl border border-[#30353e]/80 space-y-2">
              <span class="font-bold text-xs text-[#dee2ee] block">{store.t('settings.manualInstallGuideTitle')}</span>
              <p class="text-[11px] text-[#8c909f] leading-relaxed">
                {store.t('settings.manualInstallGuideDesc')}
              </p>
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer Action Bar -->
      <div class="px-6 py-3.5 bg-[#171c24] border-t border-[#252a33] flex items-center justify-between">
        <button
          type="button"
          onclick={handleResetDefaults}
          class="px-3 py-1.5 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-[#8c909f] hover:text-[#dee2ee] font-medium text-xs flex items-center gap-1.5 transition-colors cursor-pointer border border-[#30353e]"
        >
          <RotateCcw class="w-3.5 h-3.5" />
          <span>{store.t('settings.btnReset')}</span>
        </button>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            onclick={handleCancel}
            class="px-4 py-1.5 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-[#dee2ee] font-medium text-xs transition-colors cursor-pointer border border-[#30353e]"
          >
            {store.t('common.cancel')}
          </button>
          <button
            type="button"
            onclick={handleApply}
            disabled={isSaving}
            class="px-4 py-1.5 rounded-lg bg-[#252a33] hover:bg-[#30353e] text-[#00e5ff] hover:text-[#4cd7f6] font-semibold text-xs transition-colors cursor-pointer border border-[#00e5ff]/30 shadow-sm"
          >
            {store.t('common.apply')}
          </button>
          <button
            type="button"
            onclick={handleOk}
            disabled={isSaving}
            class="px-5 py-1.5 rounded-lg bg-[#00e5ff] hover:bg-[#4cd7f6] text-[#090e16] font-bold text-xs flex items-center gap-1.5 transition-all shadow-[0_0_12px_rgba(0,229,255,0.3)] cursor-pointer active:scale-95"
          >
            <Check class="w-3.5 h-3.5 stroke-[2.5]" />
            <span>{store.t('common.ok')}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
