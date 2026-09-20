<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes, formatEta, formatSpeed, type DownloadTask } from '$lib/types';
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
    LayoutGrid,
    List,
    AlertCircle,
    CheckCircle2
  } from '@lucide/svelte';

  const tasks = $derived(store.filteredTasks);
  const viewMode = $derived(store.viewMode);

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
      case 'video': return 'Video';
      case 'audio': return 'Musik';
      case 'compressed': return 'Arsip ZIP';
      case 'programs': return 'Aplikasi';
      case 'documents': return 'Dokumen';
      default: return 'Berkas';
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
    store.openProgressModal(task.id);
  }

  function cycleSort() {
    if (store.sortBy === 'date') store.sortBy = 'size';
    else if (store.sortBy === 'size') store.sortBy = 'name';
    else store.sortBy = 'date';
  }
</script>

<div class="flex-1 flex flex-col min-w-0 overflow-y-auto select-none bg-[#0f141c] pr-1">
  <!-- Control & Filter Header Bar -->
  <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2 mb-3 shrink-0">
    <div class="flex items-center gap-2">
      <h3 class="font-sans font-bold text-sm text-[#dee2ee]">
        Daftar Antrean Berkas
      </h3>
      <span class="px-2 py-0.5 rounded-full bg-[#252a33] text-[#8c909f] font-mono text-[11px]">
        {tasks.length} Berkas
      </span>
    </div>

    <div class="flex items-center gap-2 w-full sm:w-auto justify-end flex-wrap">
      <!-- Quick Status Filters -->
      <div class="flex items-center bg-[#171c24] p-0.5 rounded-lg border border-[#252a33]">
        <button
          onclick={() => (store.activeCategory = 'all')}
          class="px-2.5 py-1 rounded text-xs transition-colors cursor-pointer {store.activeCategory === 'all' ? 'bg-[#252a33] text-[#dee2ee] font-semibold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
        >
          Semua
        </button>
        <button
          onclick={() => (store.activeCategory = 'downloading')}
          class="px-2.5 py-1 rounded text-xs transition-colors cursor-pointer {store.activeCategory === 'downloading' ? 'bg-[#252a33] text-[#4cd7f6] font-semibold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
        >
          Aktif
        </button>
        <button
          onclick={() => (store.activeCategory = 'completed')}
          class="px-2.5 py-1 rounded text-xs transition-colors cursor-pointer {store.activeCategory === 'completed' ? 'bg-[#252a33] text-[#4edea3] font-semibold' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
        >
          Selesai
        </button>
      </div>

      <!-- Sorting Button -->
      <button
        onclick={cycleSort}
        class="h-7 px-2.5 rounded-lg bg-[#171c24] text-[#dee2ee] hover:bg-[#252a33] text-xs font-sans flex items-center gap-1.5 transition-colors cursor-pointer border border-[#252a33]"
      >
        <ArrowUpDown class="w-3 h-3 text-[#8c909f]" />
        <span>Urutkan: {store.sortBy === 'date' ? 'Tanggal' : store.sortBy === 'size' ? 'Ukuran' : 'Nama'}</span>
      </button>

      <!-- View Switcher (Cards vs Table) -->
      <div class="flex items-center bg-[#171c24] rounded-lg p-0.5 border border-[#252a33]">
        <button
          onclick={() => (store.viewMode = 'cards')}
          class="w-6 h-6 flex items-center justify-center rounded transition-colors cursor-pointer {viewMode === 'cards' ? 'bg-[#252a33] text-[#4cd7f6]' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
          title="Tampilan Kartu Telemetri"
        >
          <LayoutGrid class="w-3.5 h-3.5" />
        </button>
        <button
          onclick={() => (store.viewMode = 'table')}
          class="w-6 h-6 flex items-center justify-center rounded transition-colors cursor-pointer {viewMode === 'table' ? 'bg-[#252a33] text-[#4cd7f6]' : 'text-[#8c909f] hover:text-[#dee2ee]'}"
          title="Tampilan Tabel Rinci"
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
        <p class="text-xs font-semibold text-[#dee2ee]">Tidak ada unduhan dalam kategori ini</p>
        <p class="text-[11px] text-[#8c909f] mt-0.5">Klik "+ Tambah URL" untuk mulai mengunduh file dengan akselerasi multi-thread</p>
      </div>
      <button
        onclick={() => store.openAddModal()}
        class="mt-1 px-3 py-1.5 rounded-lg bg-[#4d8eff] hover:bg-[#3b82f6] text-white font-semibold text-xs transition-all shadow-[0_0_12px_rgba(77,142,255,0.3)] cursor-pointer"
      >
        + Tambah Unduhan Baru
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
                      Mengunduh
                    </span>
                  {:else if isCompleted}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#10b981]/15 text-[#4edea3] font-sans text-[10px] font-semibold border border-[#10b981]/30">
                      Selesai
                    </span>
                  {:else if isPaused}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#f59e0b]/15 text-[#f59e0b] font-sans text-[10px] font-semibold border border-[#f59e0b]/30">
                      Dijeda
                    </span>
                  {:else}
                    <span class="shrink-0 px-2 py-0.2 rounded-full bg-[#93000a]/20 text-[#ffb4ab] font-sans text-[10px] font-semibold border border-[#ffb4ab]/30">
                      Gagal
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
                  <span class="text-[#4cd7f6] font-bold">{pct.toFixed(1)}%</span>
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
                  title="Jeda Unduhan"
                >
                  <Pause class="w-3.5 h-3.5 fill-[#dee2ee]" />
                </button>
              {:else if !isCompleted}
                <button
                  onclick={(e) => { e.stopPropagation(); store.resumeTask(task.id); }}
                  class="w-7 h-7 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4edea3] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                  title="Lanjutkan Unduhan"
                >
                  <Play class="w-3.5 h-3.5 fill-[#dee2ee]" />
                </button>
              {/if}

              <button
                onclick={(e) => { e.stopPropagation(); store.openProgressModal(task.id); }}
                class="w-7 h-7 rounded-lg bg-[#252a33] text-[#4cd7f6] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                title="Buka Dialog Rincian Transfer"
              >
                <Zap class="w-3.5 h-3.5" />
              </button>

              <button
                onclick={(e) => { e.stopPropagation(); store.openFolder(task.file_path); }}
                class="w-7 h-7 rounded-lg bg-[#252a33] text-[#dee2ee] hover:text-[#4cd7f6] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                title="Buka Folder Penyimpanan"
              >
                <FolderOpen class="w-3.5 h-3.5" />
              </button>

              <button
                onclick={(e) => { e.stopPropagation(); store.cancelTask(task.id, true); }}
                class="w-7 h-7 rounded-lg bg-[#252a33] text-[#8c909f] hover:text-[#ffb4ab] hover:bg-[#343942] flex items-center justify-center transition-colors cursor-pointer"
                title="Hapus Unduhan"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <!-- Segmented Multi-Thread Parallel Buffer Bar -->
          <div class="flex flex-col gap-1 pt-1">
            <div class="h-2 w-full bg-[#090e16] rounded-full overflow-hidden flex gap-[2px] p-[1px] border border-[#252a33]">
              {#if task.segments && task.segments.length > 0}
                {#each task.segments as seg}
                  {@const segTotal = seg.end_byte - seg.start_byte + 1}
                  {@const segDone = segTotal > 0 ? (seg.downloaded_bytes / segTotal) : 0}
                  <div
                    class="h-full rounded-sm transition-all {seg.is_finished ? 'bg-[#10b981]' : isDownloading && segDone > 0 ? 'bg-[#00e5ff] shadow-[0_0_8px_rgba(0,229,255,0.4)]' : 'bg-[#252a33]'}"
                    style="flex: 1;"
                    title="Bagian #{seg.index + 1}: {(segDone * 100).toFixed(0)}%"
                  ></div>
                {/each}
              {:else}
                <div
                  class="h-full rounded-full transition-all duration-300 {isCompleted ? 'bg-[#10b981]' : isDownloading ? 'bg-[#00e5ff] shadow-glow-cyan' : 'bg-[#252a33]'}"
                  style="width: {pct}%"
                ></div>
              {/if}
            </div>

            <div class="flex items-center justify-between text-[10px] text-[#8c909f] px-0.5">
              <span class="flex items-center gap-1.5 font-sans">
                <span class="w-1.5 h-1.5 rounded-full {isDownloading ? 'bg-[#4cd7f6] animate-pulse' : isCompleted ? 'bg-[#4edea3]' : 'bg-[#8c909f]'}"></span>
                {task.connections || 1} Jalur Koneksi Aktif
              </span>
              <span class="font-mono text-[9px] text-[#8c909f]">
                {task.is_hls ? 'HLS Stream Remux' : 'Multi-Part Parallel'}
              </span>
            </div>
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <!-- TABLE VIEW -->
    <div class="border border-[#252a33] rounded-xl overflow-hidden bg-[#171c24] mb-6">
      <table class="w-full text-left border-collapse text-xs">
        <thead class="bg-[#1b2028] border-b border-[#252a33] text-[#8c909f] font-semibold uppercase tracking-wider text-[10px]">
          <tr>
            <th class="py-2.5 px-3 w-10 text-center">#</th>
            <th class="py-2.5 px-3">Nama Berkas</th>
            <th class="py-2.5 px-3 w-28">Ukuran</th>
            <th class="py-2.5 px-3 w-44">Progres</th>
            <th class="py-2.5 px-3 w-24">Kecepatan</th>
            <th class="py-2.5 px-3 w-20">Sisa Waktu</th>
            <th class="py-2.5 px-3 w-24">Status</th>
            <th class="py-2.5 px-3 w-24 text-right">Aksi</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[#252a33]/60 font-sans">
          {#each tasks as task, index (task.id)}
            {@const isSelected = store.selectedTaskId === task.id}
            {@const Icon = getFileIcon(task.category)}
            {@const pct = getPercent(task)}
            <tr
              onclick={() => handleTaskClick(task)}
              ondblclick={() => handleTaskDoubleClick(task)}
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
                      class="h-full rounded-full transition-all {task.status === 'completed' ? 'bg-[#10b981]' : task.status === 'downloading' ? 'bg-[#00e5ff]' : 'bg-[#30353e]'}"
                      style="width: {pct}%"
                    ></div>
                  </div>
                  <span class="text-[10px] font-mono text-[#8c909f]">{pct.toFixed(1)}%</span>
                </div>
              </td>
              <td class="py-2 px-3 font-mono text-[11px] text-[#4edea3]">
                {task.status === 'downloading' ? formatSpeed(task.speed_bps) : '--'}
              </td>
              <td class="py-2 px-3 font-mono text-[11px] text-[#8c909f]">
                {task.status === 'downloading' ? formatEta(task.eta_seconds) : '--'}
              </td>
              <td class="py-2 px-3">
                <span class="text-[10px] px-1.5 py-0.5 rounded font-medium {task.status === 'completed' ? 'bg-[#10b981]/20 text-[#4edea3]' : task.status === 'downloading' ? 'bg-[#03b5d3]/20 text-[#4cd7f6]' : 'bg-[#252a33] text-[#8c909f]'}">
                  {task.status === 'downloading' ? 'Mengunduh' : task.status === 'completed' ? 'Selesai' : 'Dijeda'}
                </span>
              </td>
              <td class="py-2 px-3 text-right">
                <button
                  onclick={(e) => { e.stopPropagation(); store.openProgressModal(task.id); }}
                  class="px-2 py-0.5 rounded bg-[#252a33] text-[#4cd7f6] hover:bg-[#343942] text-[10px] font-semibold cursor-pointer"
                >
                  Detail
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
