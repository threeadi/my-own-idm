<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes, formatEta, formatSpeed, bpsToUnit, type DownloadTask, type SortCriterion } from '$lib/types';
  import {
    Play,
    Pause,
    FolderOpen,
    Trash2,
    Video,
    Music,
    Archive,
    Cpu,
    FileText,
    Folder,
    Zap,
    ArrowUpDown,
    ArrowUp,
    ArrowDown,
    AlertCircle,
    CheckCircle2,
    Info,
    RotateCw,
    Copy,
    Check,
    LayoutGrid,
    List,
    Gauge
  } from '@lucide/svelte';

  const tasks = $derived(store.filteredTasks);
  const viewMode = $derived(store.viewMode);

  let isContextMenuOpen = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let contextTaskId = $state<string | null>(null);
  let copiedContextUrl = $state(false);

  const contextTask = $derived(tasks.find((t) => t.id === contextTaskId) || null);

  function handleContextMenu(e: MouseEvent, task: DownloadTask) {
    e.preventDefault();
    contextTaskId = task.id;
    // Keep context menu on screen
    contextMenuX = Math.min(e.clientX, window.innerWidth - 230);
    contextMenuY = Math.min(e.clientY, window.innerHeight - 270);
    isContextMenuOpen = true;
    store.selectedTaskId = task.id;
  }

  function closeContextMenu() {
    isContextMenuOpen = false;
    contextTaskId = null;
    copiedContextUrl = false;
  }

  function copyContextUrl() {
    if (!contextTask) return;
    navigator.clipboard.writeText(contextTask.url);
    copiedContextUrl = true;
    setTimeout(() => {
      closeContextMenu();
    }, 600);
  }

  function getFileIcon(category: string) {
    switch (category) {
      case 'video': return Video;
      case 'audio': return Music;
      case 'compressed': return Archive;
      case 'programs': return Cpu;
      case 'documents': return FileText;
      default: return Folder;
    }
  }

  function getCategoryLabel(category: string): string {
    switch (category) {
      case 'video': return store.t('sidebar.catVideo');
      case 'audio': return store.t('sidebar.catAudio');
      case 'compressed': return store.t('sidebar.catCompressed');
      case 'programs': return store.t('sidebar.catPrograms');
      case 'documents': return store.t('sidebar.catDocuments');
      default: return store.t('sidebar.catGeneral');
    }
  }

  function getPercent(task: DownloadTask): number {
    if (task.status === 'completed') return 100;
    if (!task.total_bytes || task.total_bytes === 0) return 0;
    return Math.min(100, Math.max(0, (task.downloaded_bytes / task.total_bytes) * 100));
  }

  function handleTaskClick(task: DownloadTask) {
    store.selectedTaskId = task.id;
  }

  function handleTaskDoubleClick(task: DownloadTask) {
    if (task.status === 'downloading' || task.status === 'paused') {
      store.openTransferWindow(task.id);
    } else {
      store.openPropertiesModal(task.id);
    }
  }

</script>

<svelte:window onclick={closeContextMenu} onkeydown={(e) => e.key === 'Escape' && closeContextMenu()} />

<div class="w-full flex flex-col min-w-0 select-none bg-[#0f141c]">
  <!-- Control & Filter Header Bar -->
  <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2 mb-3 shrink-0">
    <div class="flex items-center gap-2">
      <h3 class="font-sans font-bold text-sm text-[#dee2ee]">
        {store.t('table.queueTitle')}
      </h3>
      <span class="px-2 py-0.5 rounded-full bg-[#252a33] text-[#8c909f] font-mono text-[11px]">
        {store.t('table.filesCount', { count: tasks.length })}
      </span>
    </div>

    <div class="flex items-center gap-2 w-full sm:w-auto justify-end flex-wrap">
      <!-- Quick Status Filters -->
      <div class="flex items-center bg-[#171c24] p-0.5 rounded-lg border border-[#252a33]">
        <button
          onclick={() => (store.activeCategory = 'all')}
          class="px-2.5 py-1 rounded text-xs transition-colors cursor-pointer {store.activeCategory === 'all' ? 'bg-[#252a33] text-[#dee2ee] font-semibold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
        >
          {store.t('table.filterAll')}
        </button>
        <button
          onclick={() => (store.activeCategory = 'downloading')}
          class="px-2.5 py-1 rounded text-xs transition-colors cursor-pointer {store.activeCategory === 'downloading' ? 'bg-[#252a33] text-[#4cd7f6] font-semibold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
        >
          {store.t('table.filterActive')}
        </button>
        <button
          onclick={() => (store.activeCategory = 'completed')}
          class="px-2.5 py-1 rounded text-xs transition-colors cursor-pointer {store.activeCategory === 'completed' ? 'bg-[#252a33] text-[#4edea3] font-semibold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
        >
          {store.t('table.filterCompleted')}
        </button>
      </div>

      <!-- Sorting Controls: Criterion Select + Order Toggle -->
      <div class="flex items-center bg-[#171c24] rounded-lg border border-[#252a33] p-0.5">
        <div class="flex items-center pl-1.5 pr-0.5 gap-1 text-[#8c909f]">
          <ArrowUpDown class="w-3 h-3 text-[#00e5ff]" />
          <span class="text-[11px] text-[#8c909f] font-sans">{store.t('table.sortLabel')}</span>
        </div>
        <select
          value={store.sortBy}
          onchange={(e) => store.setSort(e.currentTarget.value as SortCriterion, store.sortOrder)}
          class="bg-transparent text-xs text-[#dee2ee] font-medium py-1 pr-1.5 pl-0.5 border-none outline-none cursor-pointer focus:ring-0 [&>option]:bg-[#171c24] [&>option]:text-[#dee2ee]"
          title={store.t('table.sortBy')}
        >
          <option value="date">{store.t('table.sortDate')}</option>
          <option value="size">{store.t('table.sortSize')}</option>
          <option value="name">{store.t('table.sortName')}</option>
          <option value="progress">{store.t('table.sortProgress')}</option>
          <option value="speed">{store.t('table.sortSpeed')}</option>
          <option value="status">{store.t('table.sortStatus')}</option>
        </select>
        <div class="w-[1px] h-3.5 bg-[#252a33] mx-0.5"></div>
        <button
          onclick={() => store.toggleSortOrder()}
          class="h-6 px-1.5 text-xs font-mono flex items-center gap-1 text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#252a33] rounded transition-colors cursor-pointer"
          title={store.sortOrder === 'asc' ? store.t('table.orderAscTooltip') : store.t('table.orderDescTooltip')}
        >
          {#if store.sortOrder === 'asc'}
            <ArrowUp class="w-3 h-3 text-[#10b981]" />
            <span class="text-[10px] font-bold text-[#10b981]">{store.t('table.orderAsc')}</span>
          {:else}
            <ArrowDown class="w-3 h-3 text-[#00e5ff]" />
            <span class="text-[10px] font-bold text-[#00e5ff]">{store.t('table.orderDesc')}</span>
          {/if}
        </button>
      </div>

      <!-- View Switcher (Cards vs Table) -->
      <div class="flex items-center bg-[#171c24] rounded-lg p-0.5 border border-[#252a33]">
        <button
          onclick={() => (store.viewMode = 'cards')}
          class="w-6 h-6 flex items-center justify-center rounded transition-colors cursor-pointer {viewMode === 'cards' ? 'bg-[#252a33] text-[#4cd7f6]' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
          title={store.t('table.viewCards')}
        >
          <LayoutGrid class="w-3.5 h-3.5" />
        </button>
        <button
          onclick={() => (store.viewMode = 'table')}
          class="w-6 h-6 flex items-center justify-center rounded transition-colors cursor-pointer {viewMode === 'table' ? 'bg-[#252a33] text-[#4cd7f6]' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
          title={store.t('table.viewTable')}
        >
          <List class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  </div>

  <!-- Empty State -->
  {#if tasks.length === 0}
    <div class="h-64 flex flex-col items-center justify-center text-[#8c909f] gap-3 rounded-xl border border-dashed border-[#252a33] bg-[#171c24]/40 my-4">
      <div class="w-12 h-12 rounded-xl bg-[#252a33] flex items-center justify-center text-[#8c909f]">
        <Folder class="w-6 h-6" />
      </div>
      <div class="text-center">
        <p class="text-xs font-semibold text-[#dee2ee]">{store.t('table.emptyTitle')}</p>
        <p class="text-[11px] text-[#8c909f] mt-0.5">{store.t('table.emptySubtitle')}</p>
      </div>
      <button
        onclick={() => store.openAddModal()}
        class="mt-1 px-3 py-1.5 rounded-lg bg-[#4d8eff] hover:bg-[#3b82f6] text-white font-semibold text-xs transition-all shadow-[0_0_12px_rgba(77,142,255,0.3)] cursor-pointer"
      >
        {store.t('table.emptyBtn')}
      </button>
    </div>
  {:else if viewMode === 'cards'}
    <!-- CARDS VIEW (Iconic Stitch Industrial Telemetry) -->
    <div class="space-y-2.5 pb-6">
      {#each tasks as task (task.id)}
        {@const isSelected = store.selectedTaskId === task.id}
        {@const Icon = getFileIcon(task.category)}
        {@const pct = getPercent(task)}
        {@const isDownloading = task.status === 'downloading'}
        {@const isCompleted = task.status === 'completed'}
        {@const isPaused = task.status === 'paused'}

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <article
          onclick={() => handleTaskClick(task)}
          ondblclick={() => handleTaskDoubleClick(task)}
          oncontextmenu={(e) => handleContextMenu(e, task)}
          class="p-3 sm:p-3.5 rounded-xl bg-[#171c24] hover:bg-[#1b2028] transition-all border {isSelected ? 'border-[#00e5ff] shadow-[0_0_12px_rgba(0,229,255,0.25)]' : 'border-[#252a33]'} shadow-md flex flex-col gap-2 group cursor-pointer"
        >
          <!-- Top Row: Icon + Meta + Actions -->
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-start gap-3 min-w-0 flex-1">
              <!-- Category Icon -->
              <div class="w-9 h-9 rounded-lg bg-[#252a33] flex items-center justify-center shrink-0 {isDownloading ? 'text-[#4cd7f6]' : isCompleted ? 'text-[#4edea3]' : 'text-[#adc6ff]'} border border-[#30353e]">
                <Icon class="w-4 h-4" />
              </div>

              <!-- Title & Telemetry -->
              <div class="flex flex-col min-w-0 flex-1">
                <div class="flex items-center gap-2 flex-wrap">
                  <h4 class="font-sans text-xs sm:text-sm text-[#dee2ee] font-semibold truncate group-hover:text-[#4cd7f6] transition-colors" title={task.filename}>
                    {task.filename}
                  </h4>

                  <!-- Status Badge -->
                  {#if isDownloading}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#03b5d3]/15 text-[#4cd7f6] font-sans text-[10px] font-semibold flex items-center gap-1 border border-[#03b5d3]/30">
                      <span class="w-1.5 h-1.5 rounded-full bg-[#4cd7f6] animate-ping"></span>
                      {store.t('sidebar.statusDownloading')}
                    </span>
                  {:else if isCompleted}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#10b981]/15 text-[#4edea3] font-sans text-[10px] font-semibold border border-[#10b981]/30">
                      {store.t('sidebar.statusCompleted')}
                    </span>
                  {:else if isPaused}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#f59e0b]/15 text-[#f59e0b] font-sans text-[10px] font-semibold border border-[#f59e0b]/30">
                      {store.t('sidebar.statusPaused')}
                    </span>
                  {:else}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#93000a]/20 text-[#ffb4ab] font-sans text-[10px] font-semibold border border-[#ffb4ab]/30">
                      {store.t('common.failed')}
                    </span>
                  {/if}

                  {#if task.speed_limit_bps && task.speed_limit_bps > 0}
                    {@const limitParsed = bpsToUnit(task.speed_limit_bps)}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#00e5ff]/10 text-[#00e5ff] font-mono text-[10px] font-semibold border border-[#00e5ff]/30 flex items-center gap-1" title={store.t('table.limitFileTooltip')}>
                      <Gauge class="w-3 h-3 text-[#00e5ff]" />
                      {limitParsed.value} {limitParsed.unit}
                    </span>
                  {/if}

                  {#if task.quality}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#4cd7f6]/10 text-[#4cd7f6] font-mono text-[10px] font-semibold border border-[#4cd7f6]/30 uppercase">
                      {task.quality}
                    </span>
                  {/if}
                </div>

                <!-- Telemetry Row -->
                <div class="flex items-center gap-3 mt-1 flex-wrap font-mono text-[11px] text-[#8c909f]">
                  <span>
                    {formatBytes(task.downloaded_bytes)}
                    {#if task.total_bytes}
                      <span class="text-[#8c909f]/60">/ {formatBytes(task.total_bytes)}</span>
                    {/if}
                  </span>
                  {#if task.total_bytes}
                    <span class="text-[#4cd7f6] font-bold">{pct.toFixed(1)}%</span>
                  {:else if isDownloading}
                    <span class="text-[#4cd7f6] font-bold flex items-center gap-1">
                      <span class="w-1.5 h-1.5 rounded-full bg-[#4cd7f6] animate-pulse"></span>
                      Stream
                    </span>
                  {/if}
                  {#if isDownloading}
                    <span class="text-[#4edea3] font-semibold flex items-center gap-1">
                      <Zap class="w-3 h-3 text-[#4edea3]" />
                      {formatSpeed(task.speed_bps)}
                    </span>
                    <span class="text-[#8c909f]">ETA: {formatEta(task.eta_seconds)}</span>
                  {/if}
                </div>
              </div>
            </div>

            <!-- Task Quick Actions -->
            <div class="flex items-center gap-1 shrink-0">
              {#if isDownloading}
                <button
                  onclick={(e) => { e.stopPropagation(); store.pauseTask(task.id); }}
                  class="w-7 h-7 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#ffb4ab] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                  title={store.t('menu.pause')}
                >
                  <Pause class="w-3.5 h-3.5 fill-[#dee2ee]" />
                </button>
              {:else if !isCompleted}
                <button
                  onclick={(e) => { e.stopPropagation(); store.resumeTask(task.id); }}
                  class="w-7 h-7 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4edea3] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                  title={store.t('menu.resume')}
                >
                  <Play class="w-3.5 h-3.5 fill-[#dee2ee]" />
                </button>
                <button
                  onclick={(e) => { e.stopPropagation(); store.startRefreshLink(task.id); }}
                  class="w-7 h-7 rounded-lg bg-[#252a33] text-[#00e5ff] hover:bg-[#00e5ff]/20 flex items-center justify-center transition-colors cursor-pointer"
                  title={store.t('menu.refreshLink')}
                >
                  <RotateCw class="w-3.5 h-3.5" />
                </button>
              {/if}

              <button
                onclick={(e) => { e.stopPropagation(); store.openTransferWindow(task.id); }}
                class="w-7 h-7 rounded-lg bg-[#252a33] text-[#4cd7f6] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                title={store.t('table.openTransferWindow')}
              >
                <Zap class="w-3.5 h-3.5" />
              </button>

              <button
                onclick={(e) => { e.stopPropagation(); store.openPropertiesModal(task.id); }}
                class="w-7 h-7 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#00e5ff] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                title={store.t('menu.properties')}
              >
                <Info class="w-3.5 h-3.5" />
              </button>

              <button
                onclick={(e) => { e.stopPropagation(); store.openFolder(task.file_path); }}
                class="w-7 h-7 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4cd7f6] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                title={store.t('menu.openFolder')}
              >
                <FolderOpen class="w-3.5 h-3.5" />
              </button>

              <button
                onclick={(e) => { e.stopPropagation(); store.cancelTask(task.id, true); }}
                class="w-7 h-7 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#ffb4ab] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                title={store.t('menu.delete')}
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <!-- Segmented Multi-Thread Parallel Buffer Bar -->
          <div class="flex flex-col gap-1 pt-1">
            <div class="w-full bg-[#090e16] rounded-full h-2 overflow-hidden border border-[#252a33]/60 relative">
              <div
                class="h-full rounded-full transition-all duration-300 {isCompleted ? 'bg-[#10b981]' : isDownloading ? 'bg-gradient-to-r from-[#00e5ff] to-[#4d8eff]' : 'bg-[#30353e]'} {!task.total_bytes && isDownloading ? 'w-full animate-pulse opacity-80' : ''}"
                style="width: {!task.total_bytes && isDownloading ? '100%' : `${pct}%`}"
              ></div>
            </div>

            <div class="flex items-center justify-between text-[10px] text-[#8c909f] px-0.5">
              <span class="flex items-center gap-1.5 font-sans">
                <span class="w-1.5 h-1.5 rounded-full {isDownloading ? 'bg-[#4cd7f6] animate-pulse' : isCompleted ? 'bg-[#4edea3]' : 'bg-[#8c909f]'}"></span>
                {store.t('table.connectionsActive', { count: task.connections || 1 })}
              </span>
              <span class="font-mono text-[9px] text-[#8c909f]">
                {task.is_hls ? store.t('table.hlsStream') : store.t('table.multiPart')}
              </span>
            </div>
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <!-- TABLE VIEW -->
    <div class="border border-[#252a33] rounded-xl overflow-x-auto bg-[#171c24] mb-6">
      <table class="w-full min-w-[850px] text-left border-collapse text-xs">
        <thead class="bg-[#1b2028] border-b border-[#252a33] text-[#8c909f] font-semibold uppercase tracking-wider text-[10px]">
          <tr>
            <th
              class="py-2.5 px-3 w-12 text-center cursor-pointer hover:text-[#dee2ee] transition-colors select-none"
              onclick={() => store.setSort('date')}
              title={store.t('table.sortDate')}
            >
              <div class="flex items-center justify-center gap-1">
                <span>{store.t('table.colIndex')}</span>
                {#if store.sortBy === 'date'}
                  {#if store.sortOrder === 'asc'}
                    <ArrowUp class="w-2.5 h-2.5 text-[#10b981]" />
                  {:else}
                    <ArrowDown class="w-2.5 h-2.5 text-[#00e5ff]" />
                  {/if}
                {/if}
              </div>
            </th>
            <th
              class="py-2.5 px-3 cursor-pointer hover:text-[#dee2ee] transition-colors select-none"
              onclick={() => store.setSort('name')}
              title={store.t('table.sortName')}
            >
              <div class="flex items-center gap-1">
                <span>{store.t('table.colName')}</span>
                {#if store.sortBy === 'name'}
                  {#if store.sortOrder === 'asc'}
                    <ArrowUp class="w-3 h-3 text-[#10b981]" />
                  {:else}
                    <ArrowDown class="w-3 h-3 text-[#00e5ff]" />
                  {/if}
                {/if}
              </div>
            </th>
            <th
              class="py-2.5 px-3 w-28 cursor-pointer hover:text-[#dee2ee] transition-colors select-none"
              onclick={() => store.setSort('size')}
              title={store.t('table.sortSize')}
            >
              <div class="flex items-center gap-1">
                <span>{store.t('table.colSize')}</span>
                {#if store.sortBy === 'size'}
                  {#if store.sortOrder === 'asc'}
                    <ArrowUp class="w-3 h-3 text-[#10b981]" />
                  {:else}
                    <ArrowDown class="w-3 h-3 text-[#00e5ff]" />
                  {/if}
                {/if}
              </div>
            </th>
            <th
              class="py-2.5 px-3 w-44 cursor-pointer hover:text-[#dee2ee] transition-colors select-none"
              onclick={() => store.setSort('progress')}
              title={store.t('table.sortProgress')}
            >
              <div class="flex items-center gap-1">
                <span>{store.t('table.colProgress')}</span>
                {#if store.sortBy === 'progress'}
                  {#if store.sortOrder === 'asc'}
                    <ArrowUp class="w-3 h-3 text-[#10b981]" />
                  {:else}
                    <ArrowDown class="w-3 h-3 text-[#00e5ff]" />
                  {/if}
                {/if}
              </div>
            </th>
            <th
              class="py-2.5 px-3 w-24 cursor-pointer hover:text-[#dee2ee] transition-colors select-none"
              onclick={() => store.setSort('speed')}
              title={store.t('table.sortSpeed')}
            >
              <div class="flex items-center gap-1">
                <span>{store.t('table.colSpeed')}</span>
                {#if store.sortBy === 'speed'}
                  {#if store.sortOrder === 'asc'}
                    <ArrowUp class="w-3 h-3 text-[#10b981]" />
                  {:else}
                    <ArrowDown class="w-3 h-3 text-[#00e5ff]" />
                  {/if}
                {/if}
              </div>
            </th>
            <th class="py-2.5 px-3 w-20">{store.t('table.colEta')}</th>
            <th
              class="py-2.5 px-3 w-24 cursor-pointer hover:text-[#dee2ee] transition-colors select-none"
              onclick={() => store.setSort('status')}
              title={store.t('table.sortStatus')}
            >
              <div class="flex items-center gap-1">
                <span>{store.t('table.colStatus')}</span>
                {#if store.sortBy === 'status'}
                  {#if store.sortOrder === 'asc'}
                    <ArrowUp class="w-3 h-3 text-[#10b981]" />
                  {:else}
                    <ArrowDown class="w-3 h-3 text-[#00e5ff]" />
                  {/if}
                {/if}
              </div>
            </th>
            <th class="py-2.5 px-3 w-48 text-right">{store.t('table.colActions')}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[#252a33]/60 font-sans">
          {#each tasks as task, index (task.id)}
            {@const isSelected = store.selectedTaskId === task.id}
            {@const Icon = getFileIcon(task.category)}
            {@const pct = getPercent(task)}
            {@const isDownloading = task.status === 'downloading'}
            {@const isCompleted = task.status === 'completed'}
            {@const isPaused = task.status === 'paused'}
            <tr
              onclick={() => handleTaskClick(task)}
              ondblclick={() => handleTaskDoubleClick(task)}
              oncontextmenu={(e) => handleContextMenu(e, task)}
              class="group transition-colors cursor-pointer {isSelected ? 'bg-[#00e5ff]/10 text-[#dee2ee] border-l-2 border-l-[#00e5ff]' : 'hover:bg-[#1b2028] text-[#c2c6d6]'}"
            >
              <td class="py-2 px-3 text-center text-[#8c909f] font-mono text-[11px]">{index + 1}</td>
              <td class="py-2 px-3">
                <div class="flex items-center gap-2 min-w-0">
                  <div class="w-6 h-6 rounded bg-[#252a33] flex items-center justify-center shrink-0 text-[#4cd7f6]">
                    <Icon class="w-3.5 h-3.5" />
                  </div>
                  <span class="font-medium text-xs truncate max-w-md" title={task.filename}>
                    {task.filename}
                  </span>
                </div>
              </td>
              <td class="py-2 px-3 font-mono text-[11px] text-[#8c909f]">
                {formatBytes(task.downloaded_bytes)}
              </td>
              <td class="py-2 px-3">
                <div class="space-y-1">
                  <div class="w-full bg-[#090e16] rounded-full h-1.5 overflow-hidden">
                    <div
                      class="h-full rounded-full transition-all {isCompleted ? 'bg-[#10b981]' : isDownloading ? 'bg-[#00e5ff]' : 'bg-[#30353e]'} {!task.total_bytes && isDownloading ? 'w-full animate-pulse opacity-80' : ''}"
                      style="width: {!task.total_bytes && isDownloading ? '100%' : `${pct}%`}"
                    ></div>
                  </div>
                  <span class="text-[10px] font-mono text-[#8c909f]">
                    {#if task.total_bytes}
                      {pct.toFixed(1)}%
                    {:else if isDownloading}
                      Stream
                    {:else}
                      --
                    {/if}
                  </span>
                </div>
              </td>
              <td class="py-2 px-3 font-mono text-[11px] text-[#4edea3]">
                {isDownloading ? formatSpeed(task.speed_bps) : '--'}
              </td>
              <td class="py-2 px-3 font-mono text-[11px] text-[#8c909f]">
                {isDownloading ? formatEta(task.eta_seconds) : '--'}
              </td>
              <td class="py-2 px-3">
                <div class="flex items-center gap-1 flex-wrap">
                  <span class="text-[10px] px-1.5 py-0.5 rounded font-medium {isCompleted ? 'bg-[#10b981]/20 text-[#4edea3]' : isDownloading ? 'bg-[#03b5d3]/20 text-[#4cd7f6]' : isPaused ? 'bg-[#f59e0b]/20 text-[#f59e0b]' : 'bg-[#93000a]/20 text-[#ffb4ab]'}">
                    {isDownloading ? store.t('sidebar.statusDownloading') : isCompleted ? store.t('sidebar.statusCompleted') : isPaused ? store.t('sidebar.statusPaused') : store.t('common.failed')}
                  </span>
                  {#if task.quality}
                    <span class="text-[9px] font-mono px-1.5 py-0.5 rounded bg-[#4cd7f6]/10 text-[#4cd7f6] border border-[#4cd7f6]/30 uppercase font-semibold">
                      {task.quality}
                    </span>
                  {/if}
                  {#if task.speed_limit_bps && task.speed_limit_bps > 0}
                    {@const lp = bpsToUnit(task.speed_limit_bps)}
                    <span class="text-[9px] font-mono px-1.5 py-0.5 rounded bg-[#00e5ff]/10 text-[#00e5ff] border border-[#00e5ff]/30 flex items-center gap-0.5" title="{store.t('bento.speedLimit')}: {lp.value} {lp.unit}">
                      <Gauge class="w-2.5 h-2.5 text-[#00e5ff]" />
                      {lp.value} {lp.unit}
                    </span>
                  {/if}
                </div>
              </td>
              <td class="py-2 px-3 text-right">
                <div class="flex items-center justify-end gap-1">
                  {#if isDownloading}
                    <button
                      onclick={(e) => { e.stopPropagation(); store.pauseTask(task.id); }}
                      class="w-6.5 h-6.5 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#ffb4ab] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                      title={store.t('menu.pause')}
                    >
                      <Pause class="w-3 h-3 fill-[#dee2ee]" />
                    </button>
                  {:else if !isCompleted}
                    <button
                      onclick={(e) => { e.stopPropagation(); store.resumeTask(task.id); }}
                      class="w-6.5 h-6.5 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4edea3] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                      title={store.t('menu.resume')}
                    >
                      <Play class="w-3 h-3 fill-[#dee2ee]" />
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); store.startRefreshLink(task.id); }}
                      class="w-6.5 h-6.5 rounded-lg bg-[#252a33] text-[#00e5ff] hover:bg-[#00e5ff]/20 flex items-center justify-center transition-colors cursor-pointer"
                      title={store.t('menu.refreshLink')}
                    >
                      <RotateCw class="w-3 h-3" />
                    </button>
                  {/if}

                  <button
                    onclick={(e) => { e.stopPropagation(); store.openTransferWindow(task.id); }}
                    class="w-6.5 h-6.5 rounded-lg bg-[#252a33] text-[#4cd7f6] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                    title={store.t('table.openTransferWindow')}
                  >
                    <Zap class="w-3 h-3" />
                  </button>

                  <button
                    onclick={(e) => { e.stopPropagation(); store.openPropertiesModal(task.id); }}
                    class="w-6.5 h-6.5 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#00e5ff] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                    title={store.t('menu.properties')}
                  >
                    <Info class="w-3 h-3" />
                  </button>

                  <button
                    onclick={(e) => { e.stopPropagation(); store.openFolder(task.file_path); }}
                    class="w-6.5 h-6.5 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4cd7f6] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                    title={store.t('menu.openFolder')}
                  >
                    <FolderOpen class="w-3 h-3" />
                  </button>

                  <button
                    onclick={(e) => { e.stopPropagation(); store.cancelTask(task.id, true); }}
                    class="w-6.5 h-6.5 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#ffb4ab] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                    title={store.t('menu.delete')}
                  >
                    <Trash2 class="w-3 h-3" />
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  <!-- Context Menu (Obsidian & Cyan Floating Shell) -->
  {#if isContextMenuOpen && contextTask}
    <div
      class="fixed z-50 w-56 bg-[#171c24] border border-[#30353e] rounded-xl shadow-2xl shadow-black/90 p-1 text-xs text-slate-200 animate-in fade-in zoom-in-95 duration-100 select-none"
      style="top: {contextMenuY}px; left: {contextMenuX}px;"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => { if (e.key === 'Escape') closeContextMenu(); }}
      role="menu"
      tabindex="-1"
    >
      <div class="px-3 py-1.5 border-b border-[#252a33] mb-1">
        <p class="font-mono text-[11px] font-semibold text-white truncate">{contextTask.filename}</p>
        <p class="text-[10px] text-slate-400 font-mono">
          {store.t('table.colStatus')}: {contextTask.status === 'downloading' ? store.t('sidebar.statusDownloading') : contextTask.status === 'completed' ? store.t('sidebar.statusCompleted') : contextTask.status === 'paused' ? store.t('sidebar.statusPaused') : store.t('common.failed')}
        </p>
      </div>

      {#if contextTask.status === 'downloading'}
        <button
          onclick={() => { store.pauseTask(contextTask.id); closeContextMenu(); }}
          class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#252a33] text-amber-300 transition text-left cursor-pointer"
          role="menuitem"
        >
          <Pause class="w-3.5 h-3.5" />
          <span>{store.t('menu.pause')}</span>
        </button>
      {:else if contextTask.status !== 'completed'}
        <button
          onclick={() => { store.resumeTask(contextTask.id); closeContextMenu(); }}
          class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#252a33] text-emerald-400 transition text-left cursor-pointer"
          role="menuitem"
        >
          <Play class="w-3.5 h-3.5" />
          <span>{store.t('menu.resume')}</span>
        </button>
        <button
          onclick={() => { store.startRefreshLink(contextTask.id); closeContextMenu(); }}
          class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#00e5ff]/15 text-[#00e5ff] font-semibold transition text-left cursor-pointer"
          role="menuitem"
        >
          <RotateCw class="w-3.5 h-3.5 text-[#00e5ff]" />
          <span>{store.t('menu.refreshLink')}</span>
        </button>
      {/if}

      {#if contextTask.status === 'downloading' || contextTask.status === 'paused'}
        <button
          onclick={() => { store.openTransferWindow(contextTask.id); closeContextMenu(); }}
          class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#00e5ff]/15 text-[#4cd7f6] transition text-left cursor-pointer"
          role="menuitem"
        >
          <Zap class="w-3.5 h-3.5 text-[#00e5ff]" />
          <span>{store.t('transfer.windowTitle')}</span>
        </button>
      {/if}

      {#if contextTask.status === 'completed'}
        <button
          onclick={() => { store.openFile(contextTask.file_path); closeContextMenu(); }}
          class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#252a33] text-white transition text-left cursor-pointer"
          role="menuitem"
        >
          <Play class="w-3.5 h-3.5 text-[#00e5ff]" />
          <span>{store.t('menu.openFile')}</span>
        </button>
        <button
          onclick={() => { store.openFolder(contextTask.file_path); closeContextMenu(); }}
          class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#252a33] text-slate-200 transition text-left cursor-pointer"
          role="menuitem"
        >
          <FolderOpen class="w-3.5 h-3.5 text-blue-400" />
          <span>{store.t('menu.openFolder')}</span>
        </button>
      {/if}

      <button
        onclick={copyContextUrl}
        class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#252a33] text-slate-300 transition text-left cursor-pointer"
        role="menuitem"
      >
        {#if copiedContextUrl}
          <Check class="w-3.5 h-3.5 text-emerald-400" />
          <span class="text-emerald-400">{store.t('common.copied')}</span>
        {:else}
          <Copy class="w-3.5 h-3.5 text-slate-400" />
          <span>{store.t('menu.copyUrl')}</span>
        {/if}
      </button>

      <div class="h-px bg-[#252a33] my-1"></div>

      <button
        onclick={() => { store.openPropertiesModal(contextTask.id); closeContextMenu(); }}
        class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#00e5ff]/15 text-[#4cd7f6] transition text-left cursor-pointer"
        role="menuitem"
      >
        <Gauge class="w-3.5 h-3.5 text-[#00e5ff]" />
        <span>{store.t('toolbar.speedLimiter')}...</span>
      </button>

      <button
        onclick={() => { store.openPropertiesModal(contextTask.id); closeContextMenu(); }}
        class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-[#252a33] text-slate-300 transition text-left cursor-pointer"
        role="menuitem"
      >
        <Info class="w-3.5 h-3.5 text-cyan-400" />
        <span>{store.t('menu.properties')}</span>
      </button>

      <button
        onclick={() => { store.cancelTask(contextTask.id, false); closeContextMenu(); }}
        class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-red-500/20 text-red-400 transition text-left cursor-pointer"
        role="menuitem"
      >
        <Trash2 class="w-3.5 h-3.5 text-red-400" />
        <span>{store.t('menu.delete')}</span>
      </button>
    </div>
  {/if}
</div>
