<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { formatBytes, formatSpeed } from '$lib/types';
  import { Activity, Check, Zap } from '@lucide/svelte';

  const task = $derived(store.selectedTask);
</script>

{#if task}
  <div class="rounded-xl border border-[#252a33] bg-[#171c24]/90 backdrop-blur-md p-3.5 select-none shrink-0 mt-2 shadow-lg">
    <!-- Header of Visualizer -->
    <div class="flex items-center justify-between mb-2.5 flex-wrap gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <Activity class="w-4 h-4 text-[#4cd7f6] shrink-0" />
        <span class="text-xs font-semibold text-[#dee2ee] truncate">
          {store.t('segment.monitorTitle')} <span class="text-[#4cd7f6] font-mono">{task.filename}</span>
        </span>
        <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-[#090e16] text-[#4cd7f6] border border-[#30353e]">
          {task.connections || 1} {task.connections > 1 ? store.t('segment.parallelTracks') : store.t('segment.singleStream')}
        </span>
      </div>

      <div class="flex items-center gap-3 text-xs font-mono">
        {#if task.status === 'downloading'}
          <span class="text-emerald-400 font-semibold flex items-center gap-1">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping"></span>
            {formatSpeed(task.speed_bps)}
          </span>
        {/if}
        <span class="text-[#8c909f]">
          {formatBytes(task.downloaded_bytes)} / {formatBytes(task.total_bytes)}
        </span>
      </div>
    </div>

    <!-- Segments Grid -->
    {#if task.segments && task.segments.length > 0}
      <div class="grid grid-cols-2 sm:grid-cols-4 md:grid-cols-8 gap-2 max-h-36 overflow-y-auto pr-1">
        {#each task.segments as seg}
          {@const segTotal = seg.end_byte - seg.start_byte + 1}
          {@const segPct = segTotal > 0 ? Math.min(100, Math.max(0, (seg.downloaded_bytes / segTotal) * 100)) : 0}
          <div class="bg-[#090e16] border {seg.is_finished ? 'border-[#10b981]/40' : task.status === 'downloading' ? 'border-[#00e5ff]/40 shadow-sm shadow-[rgba(0,229,255,0.1)]' : 'border-[#252a33]'} rounded-lg p-2 flex flex-col justify-between gap-1 transition-all">
            <!-- Part Header -->
            <div class="flex items-center justify-between text-[10px]">
              <span class="font-mono font-semibold {seg.is_finished ? 'text-[#4edea3]' : 'text-[#8c909f]'}">
                {store.t('segment.partNumber', { num: seg.index + 1 })}
              </span>
              {#if seg.is_finished}
                <span class="flex items-center gap-0.5 text-[#4edea3] text-[10px] font-sans">
                  <Check class="w-3 h-3 stroke-[3]" /> {store.t('common.completed')}
                </span>
              {:else}
                <span class="font-mono text-slate-300 text-[10px]">
                  {segPct.toFixed(0)}%
                </span>
              {/if}
            </div>

            <!-- Segment Progress Bar -->
            <div class="w-full bg-[#1b2028] h-1.5 rounded-full overflow-hidden relative border border-[#252a33]">
              <div
                class="h-full rounded-full transition-all duration-150 {seg.is_finished ? 'bg-[#10b981]' : task.status === 'downloading' ? 'bg-[#00e5ff] animate-pulse' : 'bg-[#30353e]'}"
                style="width: {segPct}%"
              ></div>
            </div>

            <!-- Byte Range readout -->
            <div class="text-[9px] font-mono text-[#8c909f] truncate">
              {formatBytes(seg.downloaded_bytes)}
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="py-2.5 text-center text-xs text-[#8c909f] font-sans">
        Single connection streaming mode ({task.is_hls ? 'HLS Remuxing' : 'Direct'})
      </div>
    {/if}
  </div>
{/if}
