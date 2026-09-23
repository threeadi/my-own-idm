<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes } from '$lib/types';
  import type { TelegramMediaItem } from '$lib/types';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    Send,
    ShieldCheck,
    Search,
    Video,
    FileText,
    Music,
    Image as ImageIcon,
    FolderOpen,
    Download,
    Loader2,
    CheckSquare,
    Square,
    Sparkles,
    UserCheck,
    Shield,
    ChevronDown,
    FolderPlus
  } from '@lucide/svelte';

  let chatInput = $state(store.telegramActiveChatInput || '');
  let selectedFilter = $state<'all' | 'video' | 'document' | 'audio' | 'photo'>('all');
  let selectedMediaIds = $state<number[]>([]);
  let customSaveDir = $state(store.settings.defaultDownloadDir || 'C:\\Downloads');
  let useChannelSubfolder = $state(false);

  $effect(() => {
    chatInput = store.telegramActiveChatInput || '';
  });

  $effect(() => {
    if (store.settings.defaultDownloadDir && !customSaveDir) {
      customSaveDir = store.settings.defaultDownloadDir;
    }
  });

  const effectiveTargetDir = $derived(
    useChannelSubfolder
      ? `${customSaveDir.replace(/[\\/]+$/, '')}\\Telegram\\${chatInput.replace(/^@/, '')}`
      : customSaveDir
  );

  async function handleBrowseFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: customSaveDir || undefined,
        title: 'Pilih Direktori Penyimpanan Telegram',
      });
      if (selected && typeof selected === 'string') {
        customSaveDir = selected;
      }
    } catch (e) {
      console.warn('Folder picker error:', e);
    }
  }

  async function handleScan() {
    if (!chatInput.trim()) return;
    await store.scanTelegramChat(chatInput.trim(), selectedFilter);
  }

  function toggleFilter(filter: 'all' | 'video' | 'document' | 'audio' | 'photo') {
    selectedFilter = filter;
    store.scanTelegramChat(chatInput, filter);
  }

  function toggleSelectMedia(id: number) {
    if (selectedMediaIds.includes(id)) {
      selectedMediaIds = selectedMediaIds.filter(i => i !== id);
    } else {
      selectedMediaIds = [...selectedMediaIds, id];
    }
  }

  function toggleSelectAll() {
    if (selectedMediaIds.length === store.telegramScannedMedia.length) {
      selectedMediaIds = [];
    } else {
      selectedMediaIds = store.telegramScannedMedia.map(m => m.message_id);
    }
  }

  async function handleDownloadSelected() {
    const items = store.telegramScannedMedia.filter(m => selectedMediaIds.includes(m.message_id));
    const targetItems = items.length > 0 ? items : store.telegramScannedMedia;
    await store.downloadTelegramMediaItems(targetItems, effectiveTargetDir);
  }

  function getMediaIcon(mediaType: string) {
    switch (mediaType) {
      case 'video': return Video;
      case 'audio': return Music;
      case 'document': return FileText;
      case 'photo': return ImageIcon;
      default: return FileText;
    }
  }
</script>

<div class="space-y-4 animate-in fade-in duration-200">
  <!-- Channel Header Banner & Controls -->
  <div class="p-4 rounded-xl bg-[#171c24] border border-[#30353e] space-y-3 shadow-xl">
    <div class="flex flex-wrap items-center justify-between gap-3 border-b border-[#30353e]/80 pb-3">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-[#4cd7f6]/15 flex items-center justify-center text-[#4cd7f6] shadow-[0_0_16px_rgba(76,215,246,0.3)]">
          <Send class="w-5 h-5" />
        </div>
        <div>
          <h2 class="font-sans text-sm sm:text-base font-bold text-[#dee2ee] flex items-center gap-2">
            {store.t('telegram.title')}
            {#if store.telegramAuthStatus?.is_authenticated}
              <span class="px-2 py-0.5 rounded-full bg-[#10b981]/15 text-[#4edea3] font-mono text-[10px] font-semibold flex items-center gap-1 border border-[#10b981]/30">
                <span class="w-1.5 h-1.5 rounded-full bg-[#10b981] animate-pulse"></span>
                {store.t('telegram.connected')}
              </span>
            {:else}
              <span class="px-2 py-0.5 rounded-full bg-[#f59e0b]/15 text-[#f59e0b] font-mono text-[10px] font-semibold flex items-center gap-1 border border-[#f59e0b]/30">
                <span class="w-1.5 h-1.5 rounded-full bg-[#f59e0b]"></span>
                {store.t('telegram.notLoggedIn')}
              </span>
            {/if}
          </h2>
          <p class="text-xs text-[#8c909f]">{store.t('telegram.subtitle')}</p>
        </div>
      </div>

      <!-- Account Switcher Dropdown & Auth Badge -->
      <div class="flex items-center gap-2 bg-[#1b2028] px-3 py-1.5 rounded-xl border border-[#30353e]">
        <div class="flex items-center gap-2">
          <ShieldCheck class="w-4 h-4 {store.telegramAuthStatus?.is_authenticated ? 'text-[#4edea3]' : 'text-[#f59e0b]'}" />
          <div class="flex flex-col">
            <span class="text-[9px] font-semibold text-[#8c909f] uppercase tracking-wider">{store.t('telegram.activeAccount')}</span>
            {#if store.telegramAccounts.length > 1}
              <select
                value={store.activeAccountId || ''}
                onchange={(e) => store.switchTelegramAccount((e.target as HTMLSelectElement).value)}
                class="bg-transparent font-mono text-xs font-semibold text-[#4cd7f6] focus:outline-none cursor-pointer"
              >
                {#each store.telegramAccounts as acc}
                  <option value={acc.account_id} class="bg-[#1b2028] text-[#dee2ee]">
                    {acc.phone_number || acc.username || acc.account_id} ({acc.account_type})
                  </option>
                {/each}
              </select>
            {:else if store.telegramAuthStatus?.is_authenticated}
              <span class="font-mono text-xs font-semibold text-[#dee2ee]">
                {store.telegramAuthStatus?.phone_number || store.telegramAuthStatus?.username || store.t('telegram.activeAccount')}
              </span>
            {:else}
              <span class="font-mono text-xs font-semibold text-[#f59e0b]">
                {store.t('telegram.noAccount')}
              </span>
            {/if}
          </div>
        </div>
        <div class="h-5 w-[1px] bg-[#30353e]"></div>
        <button
          onclick={() => store.openSettingsModal('telegram')}
          class="text-xs font-semibold text-[#4cd7f6] hover:text-[#acedff] flex items-center gap-1 cursor-pointer px-1 py-0.5 rounded hover:bg-[#252a33]"
          type="button"
        >
          <UserCheck class="w-3.5 h-3.5" />
          <span>{store.telegramAuthStatus?.is_authenticated ? store.t('telegram.manageAccounts') : store.t('telegram.loginAccount')}</span>
        </button>
      </div>
    </div>

    <!-- Search & Channel Input Row -->
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-3 items-center">
      <div class="lg:col-span-8 flex flex-col sm:flex-row gap-2">
        <div class="relative flex-1">
          <span class="absolute left-3 top-1/2 -translate-y-1/2 text-[#4cd7f6] text-xs font-bold font-mono">@</span>
          <input
            type="text"
            bind:value={chatInput}
            placeholder={store.t('telegram.searchPlaceholder')}
            class="w-full h-10 pl-8 pr-3.5 bg-[#090e16] text-[#dee2ee] text-xs font-sans rounded-xl border border-[#30353e] focus:outline-none focus:border-[#4cd7f6] placeholder-[#8c909f] transition-all"
          />
        </div>
        <button
          onclick={handleScan}
          disabled={store.telegramIsScanning}
          class="h-10 px-5 rounded-xl bg-[#4cd7f6] text-[#090e16] font-semibold text-xs flex items-center justify-center gap-2 hover:bg-[#acedff] transition-all cursor-pointer shadow-md shadow-[#4cd7f6]/20 disabled:opacity-50 shrink-0"
          type="button"
        >
          {#if store.telegramIsScanning}
            <Loader2 class="w-4 h-4 animate-spin" />
            <span>{store.t('telegram.scanning')}</span>
          {:else}
            <Search class="w-4 h-4" />
            <span>{store.t('telegram.btnScanMedia')}</span>
          {/if}
        </button>
      </div>

      <div class="lg:col-span-4 flex flex-col gap-1 bg-[#252a33] px-3.5 py-2 rounded-xl border border-[#30353e]">
        <div class="flex items-center justify-between gap-2 min-w-0">
          <div class="flex flex-col min-w-0 flex-1">
            <span class="text-[10px] font-semibold text-[#8c909f] uppercase tracking-wider">{store.t('telegram.targetSaveDir')}</span>
            <span class="font-mono text-[11px] text-[#adc6ff] truncate" title={effectiveTargetDir}>
              {effectiveTargetDir}
            </span>
          </div>
          <button
            onclick={handleBrowseFolder}
            class="p-1.5 rounded-lg bg-[#1b2028] text-[#8c909f] hover:text-[#4cd7f6] hover:bg-[#30353e] transition-colors cursor-pointer shrink-0 flex items-center gap-1 border border-[#30353e]"
            title={store.t('telegram.changeFolder')}
            type="button"
          >
            <FolderOpen class="w-3.5 h-3.5 text-[#4cd7f6]" />
            <span class="text-[10px] font-semibold text-[#dee2ee]">{store.t('telegram.changeFolder')}</span>
          </button>
        </div>

        <label class="flex items-center gap-1.5 text-[10px] text-[#8c909f] hover:text-[#dee2ee] cursor-pointer pt-1 border-t border-[#30353e]/40 select-none">
          <input
            type="checkbox"
            bind:checked={useChannelSubfolder}
            class="rounded border-[#30353e] bg-[#171c24] text-[#4cd7f6] focus:ring-0 w-3 h-3 cursor-pointer"
          />
          <span>{store.t('telegram.createSubfolder')} (<strong class="font-mono text-[#4cd7f6]">\Telegram\{chatInput.replace(/^@/, '')}\</strong>)</span>
        </label>
      </div>
    </div>

    <!-- Channel Scan Info Bar -->
    <div class="flex items-center justify-between text-xs text-[#8c909f] pt-1 border-t border-[#30353e]/40">
      <span class="text-[#adc6ff]">{store.t('telegram.scanHelpNotice')}</span>
      <div class="flex items-center gap-2">
        <span class="flex items-center gap-1">
          <Shield class="w-3.5 h-3.5 text-[#4edea3]" />
          {store.t('telegram.antiFloodActive')}
        </span>
        <span>•</span>
        <span class="font-mono text-[11px]">{store.t('telegram.antiFloodMetrics')}</span>
      </div>
    </div>
  </div>

  <!-- Media Items Queue Table / List -->
  <div class="space-y-2">
    <div class="flex items-center justify-between px-1">
      <div class="flex items-center gap-2">
        <h3 class="font-sans text-sm font-bold text-[#dee2ee]">{store.t('telegram.queueTitle')}</h3>
        <span class="px-2 py-0.5 rounded bg-[#252a33] text-[#4cd7f6] font-mono text-[11px] font-semibold">
          {store.t('telegram.filesFound', { count: store.telegramScannedMedia.length })}
        </span>
      </div>

      <div class="flex items-center gap-2">
        <button
          onclick={toggleSelectAll}
          class="h-7 px-2.5 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#dee2ee] text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer"
          type="button"
        >
          {#if selectedMediaIds.length === store.telegramScannedMedia.length && store.telegramScannedMedia.length > 0}
            <CheckSquare class="w-3.5 h-3.5 text-[#4cd7f6]" />
          {:else}
            <Square class="w-3.5 h-3.5" />
          {/if}
          <span>{store.t('telegram.selectAll')}</span>
        </button>

        <button
          onclick={handleDownloadSelected}
          disabled={store.telegramScannedMedia.length === 0}
          class="h-7 px-3.5 rounded-lg bg-[#10b981] text-white hover:bg-[#059669] text-xs font-semibold flex items-center gap-1.5 transition-all cursor-pointer shadow-md shadow-[#10b981]/20 disabled:opacity-50"
          type="button"
        >
          <Download class="w-3.5 h-3.5" />
          <span>{store.t('telegram.downloadSelected', { count: selectedMediaIds.length || store.telegramScannedMedia.length })}</span>
        </button>
      </div>
    </div>

    <!-- Media Cards Stream List -->
    <div class="space-y-2">
      {#each store.telegramScannedMedia as item (item.message_id)}
        {@const IconComp = getMediaIcon(item.media_type)}
        {@const isSelected = selectedMediaIds.includes(item.message_id)}
        <div
          onclick={() => toggleSelectMedia(item.message_id)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggleSelectMedia(item.message_id); }}
          role="button"
          tabindex="0"
          class={`p-3.5 rounded-xl bg-[#1b2028] border transition-all cursor-pointer flex items-center justify-between gap-3 shadow-md hover:bg-[#252a33] ${isSelected ? 'border-[#4cd7f6] bg-[#4cd7f6]/5' : 'border-[#30353e]'}`}
        >
          <div class="flex items-center gap-3 min-w-0 flex-1">
            <button
              type="button"
              class="text-[#4cd7f6] shrink-0 cursor-pointer"
              onclick={(e) => { e.stopPropagation(); toggleSelectMedia(item.message_id); }}
            >
              {#if isSelected}
                <CheckSquare class="w-4 h-4 text-[#4cd7f6]" />
              {:else}
                <Square class="w-4 h-4 text-[#8c909f]" />
              {/if}
            </button>

            <div class="w-10 h-10 rounded-xl bg-[#252a33] flex items-center justify-center text-[#4cd7f6] shrink-0">
              <IconComp class="w-5 h-5" />
            </div>

            <div class="flex flex-col min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-sans text-xs font-semibold text-[#dee2ee] truncate" title={item.filename}>
                  {item.filename}
                </span>
                <span class="px-1.5 py-0.2 rounded bg-[#4cd7f6]/10 text-[#4cd7f6] font-mono text-[10px] font-semibold">
                  Telegram Media
                </span>
                {#if item.resolution}
                  <span class="px-1.5 py-0.2 rounded bg-[#252a33] text-[#8c909f] font-mono text-[10px]">
                    {item.resolution}
                  </span>
                {/if}
              </div>

              <div class="flex flex-wrap items-center gap-x-3 gap-y-0.5 font-mono text-[11px] text-[#8c909f] pt-1">
                <span class="text-[#4cd7f6] font-semibold">{formatBytes(item.file_size)}</span>
                <span>Msg ID: #{item.message_id}</span>
                <span>{item.created_at}</span>
                {#if item.crc32_hash}
                  <span class="text-[#c2c6d6]">CRC32: {item.crc32_hash}</span>
                {/if}
              </div>
            </div>
          </div>

          <button
            onclick={(e) => { e.stopPropagation(); store.startTelegramDownloadDirect(item); }}
            class="h-8 px-3 rounded-lg bg-[#4cd7f6]/10 text-[#4cd7f6] hover:bg-[#4cd7f6] hover:text-[#090e16] text-xs font-semibold flex items-center gap-1.5 transition-all shrink-0 cursor-pointer shadow-sm"
            title={store.t('telegram.singleDownload')}
            type="button"
          >
            <Download class="w-3.5 h-3.5" />
            <span>{store.t('telegram.singleDownload')}</span>
          </button>
        </div>
      {/each}

      {#if store.telegramScannedMedia.length === 0}
        <div class="p-10 text-center rounded-xl bg-[#1b2028] border border-[#30353e] space-y-2">
          <Send class="w-10 h-10 text-[#8c909f] mx-auto opacity-40" />
          <p class="text-xs text-[#8c909f]">{store.t('telegram.emptyMedia')}</p>
        </div>
      {/if}
    </div>
  </div>
</div>
