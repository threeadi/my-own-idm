<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from '$lib/idmStore.svelte';
  import Toolbar from '../components/Toolbar.svelte';
  import Sidebar from '../components/Sidebar.svelte';
  import TelemetryBento from '../components/TelemetryBento.svelte';
  import DownloadTable from '../components/DownloadTable.svelte';
  import SegmentVisualizer from '../components/SegmentVisualizer.svelte';
  import AddDownloadModal from '../components/AddDownloadModal.svelte';
  import DownloadProgressModal from '../components/DownloadProgressModal.svelte';
  import DownloadOutcomeModal from '../components/DownloadOutcomeModal.svelte';
  import SettingsModal from '../components/SettingsModal.svelte';

  onMount(() => {
    store.init();
  });
</script>

<div class="h-screen w-screen flex flex-col bg-[#0f141c] text-[#dee2ee] overflow-hidden font-sans select-none">
  <!-- Top IDM Turbo Application Toolbar -->
  <Toolbar />

  <!-- Main Workspace -->
  <div class="flex-1 flex overflow-hidden">
    <!-- Left Categories & Status Navigation -->
    <Sidebar />

    <!-- Right Workspace Area -->
    <main class="flex-1 flex flex-col min-w-0 overflow-y-auto bg-[#0f141c] p-3 sm:p-4">
      <!-- Bento Telemetry Dashboard -->
      <TelemetryBento />

      <!-- Active Downloads Queue / Cards -->
      <DownloadTable />

      <!-- IDM Realtime Segment Monitor for Selected Task -->
      <SegmentVisualizer />
    </main>
  </div>

  <!-- Interactive Dialog Modals -->
  <AddDownloadModal />
  <DownloadProgressModal />
  <DownloadOutcomeModal />
  <SettingsModal />
</div>

