<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { X, Settings, FolderOpen, Save, FileText } from '@lucide/svelte';

  let defaultDir = $state('');
  let defaultConnections = $state(8);

  $effect(() => {
    if (store.isSettingsModalOpen) {
      invoke<string>('get_default_download_dir').then((dir) => {
        defaultDir = dir;
      });
    }
  });

  async function browseDefaultFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: defaultDir || undefined,
        title: 'Select Default Downloads Folder',
      });
      if (selected && typeof selected === 'string') {
        defaultDir = selected;
      }
    } catch (e) {
      console.error('Folder selection error:', e);
    }
  }

  function saveSettings() {
    store.isSettingsModalOpen = false;
  }
</script>

{#if store.isSettingsModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4 select-none animate-in fade-in duration-150">
    <div class="w-full max-w-md bg-slate-900 border border-slate-700/80 rounded-2xl shadow-2xl shadow-cyan-500/10 overflow-hidden flex flex-col">
      <!-- Modal Header -->
      <div class="px-5 py-4 border-b border-slate-800 flex items-center justify-between bg-slate-900/50">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-lg bg-cyan-500/20 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
            <Settings class="w-4 h-4" />
          </div>
          <div>
            <h3 class="font-bold text-sm text-slate-100">Preferences</h3>
            <p class="text-[11px] text-slate-400">Configure default download behavior</p>
          </div>
        </div>
        <button
          onclick={() => (store.isSettingsModalOpen = false)}
          class="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-5 space-y-4 text-xs">
        <div>
          <label for="default-dir-input" class="block text-slate-300 font-medium mb-1">Default Downloads Directory</label>
          <div class="flex gap-2">
            <input
              id="default-dir-input"
              type="text"
              bind:value={defaultDir}
              class="flex-1 bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-slate-200 text-xs focus:outline-none focus:border-cyan-500"
            />
            <button
              type="button"
              onclick={browseDefaultFolder}
              class="px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 font-medium text-xs flex items-center gap-1.5 transition-colors cursor-pointer shrink-0"
            >
              <FolderOpen class="w-3.5 h-3.5" />
              <span>Browse...</span>
            </button>
          </div>
        </div>

        <div>
          <label for="default-connections-select" class="block text-slate-300 font-medium mb-1">Default Connection Threads</label>
          <select
            id="default-connections-select"
            bind:value={defaultConnections}
            class="w-full bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-2 text-slate-200 text-xs focus:outline-none focus:border-cyan-500"
          >
            <option value={4}>4 connections</option>
            <option value={8}>8 connections (Recommended)</option>
            <option value={16}>16 connections (High Speed)</option>
            <option value={32}>32 connections (Max Throughput)</option>
          </select>
        </div>

        <div class="p-3 bg-slate-950/60 rounded-xl border border-slate-800 space-y-1.5">
          <span class="font-semibold text-slate-300 text-xs block">System Tray Behavior</span>
          <p class="text-[11px] text-slate-400">
            Closing the window automatically minimizes to the Windows System Tray so active downloads continue without interruption.
          </p>
        </div>

        <div class="p-3 bg-slate-950/60 rounded-xl border border-slate-800 flex items-center justify-between">
          <div>
            <span class="font-semibold text-slate-300 text-xs block">Diagnostics & Application Logs</span>
            <p class="text-[11px] text-slate-400">Inspect download probe, network headers, and error logs</p>
          </div>
          <button
            type="button"
            onclick={() => invoke('open_log_folder')}
            class="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-cyan-400 font-medium text-xs flex items-center gap-1.5 transition-colors cursor-pointer shrink-0"
          >
            <FileText class="w-3.5 h-3.5" />
            <span>Open Logs</span>
          </button>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="px-5 py-4 border-t border-slate-800 bg-slate-900/50 flex items-center justify-end gap-2">
        <button
          onclick={() => (store.isSettingsModalOpen = false)}
          class="px-3.5 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 font-medium text-xs transition-colors cursor-pointer"
        >
          Cancel
        </button>
        <button
          onclick={saveSettings}
          class="px-4 py-2 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs flex items-center gap-1.5 transition-all shadow-md shadow-cyan-500/20 active:scale-95 cursor-pointer"
        >
          <Save class="w-3.5 h-3.5" />
          <span>Save Changes</span>
        </button>
      </div>
    </div>
  </div>
{/if}
