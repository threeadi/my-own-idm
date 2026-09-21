<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from '$lib/idmStore.svelte';
  import Toolbar from '../components/Toolbar.svelte';
  import Sidebar from '../components/Sidebar.svelte';
  import TelemetryBento from '../components/TelemetryBento.svelte';
  import DownloadTable from '../components/DownloadTable.svelte';
  import AddDownloadModal from '../components/AddDownloadModal.svelte';
  import DownloadProgressModal from '../components/DownloadProgressModal.svelte';
  import DownloadOutcomeModal from '../components/DownloadOutcomeModal.svelte';
  import FilePropertiesModal from '../components/FilePropertiesModal.svelte';
  import SettingsModal from '../components/SettingsModal.svelte';
  import RefreshLinkModal from '../components/RefreshLinkModal.svelte';
  import DuplicateConfirmModal from '../components/DuplicateConfirmModal.svelte';

  onMount(() => {
    store.init();
    if (typeof window !== 'undefined') {
      (window as any).__store = store;
      const urlParams = new URLSearchParams(window.location.search);
      if (urlParams.get('test_outcome') === 'completed') {
        store.openOutcomeModal('mock-1', 'completed');
      } else if (urlParams.get('test_outcome') === 'failed') {
        store.openOutcomeModal(
          'mock-3',
          'failed',
          'HTTP 504 Gateway Timeout / Connection Refused by Host Server'
        );
      } else if (urlParams.get('test_refresh') === 'listening') {
        store.startRefreshLink('mock-4');
      } else if (urlParams.get('test_refresh') === 'detected') {
        store.startRefreshLink('mock-4');
        store.refreshDetectedUrl =
          'https://instagram.fsrg2-1.fna.fbcdn.net/v/t50.2886-16/fresh_token_video_1080p.mp4';
      }
    }
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
    </main>
  </div>

  <!-- Interactive Dialog Modals -->
  <AddDownloadModal />
  <DownloadProgressModal />
  <DownloadOutcomeModal />
  <FilePropertiesModal />
  <RefreshLinkModal />
  <DuplicateConfirmModal />
  <SettingsModal />
</div>

