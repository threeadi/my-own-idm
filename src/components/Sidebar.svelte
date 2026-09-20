<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import {
    ListFilter,
    ArrowDownCircle,
    CheckCircle2,
    PauseCircle,
    Trash2,
    Video,
    Music,
    Archive,
    Cpu,
    FileText,
    CalendarClock,
    Activity
  } from '@lucide/svelte';

  const counts = $derived(store.categoryCounts);

  const statusFilters = [
    { id: 'all', label: 'Semua Unduhan', icon: ListFilter, countKey: 'all', color: 'text-[#adc6ff]' },
    { id: 'downloading', label: 'Sedang Mengunduh', icon: ArrowDownCircle, countKey: 'downloading', color: 'text-[#4cd7f6]', pulse: true },
    { id: 'paused', label: 'Dijeda', icon: PauseCircle, countKey: 'paused', color: 'text-[#8c909f]' },
    { id: 'completed', label: 'Selesai', icon: CheckCircle2, countKey: 'completed', color: 'text-[#4edea3]' },
  ];

  const categoryFilters = [
    { id: 'programs', label: 'Aplikasi', icon: Cpu, ext: '.exe / .dmg', color: 'text-[#4cd7f6]' },
    { id: 'documents', label: 'Dokumen', icon: FileText, ext: '.pdf / .docx', color: 'text-[#adc6ff]' },
    { id: 'video', label: 'Video', icon: Video, ext: '.mp4 / .mkv', color: 'text-[#4edea3]' },
    { id: 'audio', label: 'Musik', icon: Music, ext: '.mp3 / .flac', color: 'text-[#4cd7f6]' },
    { id: 'compressed', label: 'Arsip ZIP', icon: Archive, ext: '.zip / .rar', color: 'text-[#adc6ff]' },
  ];
</script>

<aside class="w-60 shrink-0 select-none bg-[#171c24]/90 backdrop-blur-xl border-r border-[#30353e]/70 p-3 flex flex-col justify-between overflow-y-auto">
  <div class="space-y-4">
    <!-- Status Unduhan -->
    <div>
      <div class="px-2 py-1 mb-1">
        <span class="font-sans text-[10px] font-bold uppercase tracking-wider text-[#8c909f]">
          Status Unduhan
        </span>
      </div>
      <nav class="flex flex-col gap-1">
        {#each statusFilters as item}
          {@const isActive = store.activeCategory === item.id}
          <button
            onclick={() => (store.activeCategory = item.id)}
            class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs transition-all cursor-pointer {isActive ? 'bg-[#4d8eff] text-white font-semibold shadow-[0_0_12px_rgba(77,142,255,0.3)]' : 'text-[#c2c6d6] hover:bg-[#252a33] hover:text-[#dee2ee]'}"
          >
            <span class="flex items-center gap-2">
              <item.icon class="w-3.5 h-3.5 {isActive ? 'text-white' : item.color} {item.pulse && counts[item.countKey] > 0 ? 'animate-pulse' : ''}" />
              <span>{item.label}</span>
            </span>
            <span class="font-mono text-[11px] px-1.5 py-0.2 rounded {isActive ? 'bg-black/20 text-white font-bold' : 'bg-[#252a33] text-[#8c909f]'}">
              {counts[item.countKey] || 0}
            </span>
          </button>
        {/each}
      </nav>
    </div>

    <!-- Kategori Berkas -->
    <div>
      <div class="px-2 py-1 mb-1">
        <span class="font-sans text-[10px] font-bold uppercase tracking-wider text-[#8c909f]">
          Kategori Berkas
        </span>
      </div>
      <nav class="flex flex-col gap-1">
        {#each categoryFilters as item}
          {@const isActive = store.activeCategory === item.id}
          <button
            onclick={() => (store.activeCategory = item.id)}
            class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs transition-all cursor-pointer {isActive ? 'bg-[#4d8eff] text-white font-semibold shadow-[0_0_12px_rgba(77,142,255,0.3)]' : 'text-[#c2c6d6] hover:bg-[#252a33] hover:text-[#dee2ee]'}"
          >
            <span class="flex items-center gap-2">
              <item.icon class="w-3.5 h-3.5 {isActive ? 'text-white' : item.color}" />
              <span>{item.label}</span>
            </span>
            <span class="font-mono text-[10px] {isActive ? 'text-white/80' : 'text-[#8c909f]'}">
              {item.ext}
            </span>
          </button>
        {/each}
      </nav>
    </div>

    <!-- Otomasi -->
    <div>
      <div class="px-2 py-1 mb-1">
        <span class="font-sans text-[10px] font-bold uppercase tracking-wider text-[#8c909f]">
          Otomasi
        </span>
      </div>
      <nav class="flex flex-col gap-1">
        <button
          onclick={() => (store.activeCategory = 'all')}
          class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs text-[#c2c6d6] hover:bg-[#252a33] hover:text-[#dee2ee] transition-all cursor-pointer"
        >
          <span class="flex items-center gap-2">
            <CalendarClock class="w-3.5 h-3.5 text-[#4cd7f6]" />
            <span>Antrean & Jadwal</span>
          </span>
          <span class="w-2 h-2 rounded-full bg-[#4edea3] shadow-[0_0_6px_rgba(78,222,163,0.5)]"></span>
        </button>
      </nav>
    </div>
  </div>

  <!-- Bottom Engine Status Panel -->
  <div class="p-2.5 rounded-xl bg-[#252a33]/60 border border-[#30353e]/80 flex items-center justify-between mt-3">
    <div class="flex flex-col">
      <span class="font-sans text-[9px] text-[#8c909f] uppercase tracking-wider font-semibold">
        Status Mesin
      </span>
      <span class="font-sans text-[11px] font-semibold text-[#4edea3] flex items-center gap-1.5 mt-0.5">
        <span class="w-1.5 h-1.5 rounded-full bg-[#4edea3] animate-ping"></span>
        Multi-Thread Aktif
      </span>
    </div>
    <span class="font-mono text-xs text-[#4cd7f6] bg-[#090e16] px-2 py-0.5 rounded border border-[#30353e]">
      32 Bagian
    </span>
  </div>
</aside>
