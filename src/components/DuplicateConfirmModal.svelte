<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { X, Check } from '@lucide/svelte';

  const data = $derived(store.duplicateModalData);
  const currentUrl = $derived(store.duplicateModalUrl);

  let selectedOption = $state<'numbered' | 'overwrite' | 'resume'>('resume');
  let rememberChoice = $state<boolean>(false);

  // Reset state whenever modal is opened
  $effect(() => {
    if (store.isDuplicateModalOpen) {
      selectedOption = (store.settings.duplicateAction && store.settings.duplicateAction !== 'ask')
        ? store.settings.duplicateAction
        : 'resume';
      rememberChoice = store.settings.duplicateActionRemember || false;
    }
  });

  function handleClose() {
    store.closeDuplicateModal();
  }

  async function handleConfirm() {
    await store.proceedWithDuplicateAction(selectedOption, rememberChoice);
  }
</script>

{#if store.isDuplicateModalOpen && data}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/60 backdrop-blur-[3px] select-none animate-in fade-in duration-150">
    <!-- Obsidian Compact Dialog Container (Matching IDM screenshot layout) -->
    <div
      id="duplicate-confirm-modal"
      class="relative w-full max-w-lg bg-[#171c24]/95 backdrop-blur-2xl rounded-xl shadow-2xl border border-[#30353e] overflow-hidden flex flex-col transition-all duration-200 animate-in zoom-in-95"
    >
      <!-- Top Cyan Glow Accent Line -->
      <div class="absolute inset-x-0 top-0 h-[2px] bg-gradient-to-r from-transparent via-[#00e5ff]/80 to-transparent"></div>

      <!-- Window Title Bar -->
      <div data-tauri-drag-region class="px-4 py-2 bg-[#252a33]/90 flex items-center justify-between border-b border-[#30353e]/80 cursor-move">
        <span class="font-sans text-xs font-semibold text-[#dee2ee] tracking-wide">
          {store.t('dup.modalTitle')}
        </span>

        <button
          onclick={handleClose}
          class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:text-[#dee2ee] hover:bg-[#30353e] transition-colors cursor-pointer"
          aria-label={store.t('common.close')}
          type="button"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-4 sm:p-5 space-y-4">
        <!-- Top URL Box (Read-only horizontally scrollable box as in IDM screenshot) -->
        <div class="w-full px-3 py-2 rounded-lg bg-[#090e16]/90 border border-[#30353e] shadow-inner text-xs font-mono text-[#4cd7f6] overflow-x-auto whitespace-nowrap scrollbar-thin select-all">
          {currentUrl || data.filename || ''}
        </div>

        <!-- Subtitle Prompt Text -->
        <p class="font-sans text-xs text-[#c2c6d6] leading-relaxed">
          {store.t('dup.prompt')}
        </p>

        <!-- 3 Radio Options -->
        <div class="space-y-3 pt-1">
          <!-- Option 1: Numbered Filename -->
          <label class="flex items-start gap-3 cursor-pointer group">
            <div class="pt-0.5">
              <input
                type="radio"
                name="duplicate-choice"
                value="numbered"
                checked={selectedOption === 'numbered'}
                onchange={() => selectedOption = 'numbered'}
                class="sr-only"
              />
              <div class="w-4 h-4 rounded-full border transition-all flex items-center justify-center {selectedOption === 'numbered' ? 'border-[#00e5ff] bg-[#00e5ff]/15' : 'border-[#4b5260] bg-[#090e16] group-hover:border-[#8c909f]'}">
                {#if selectedOption === 'numbered'}
                  <div class="w-2 h-2 rounded-full bg-[#00e5ff] shadow-[0_0_8px_#00e5ff]"></div>
                {/if}
              </div>
            </div>
            <span class="font-sans text-xs text-[#dee2ee] leading-tight select-none group-hover:text-white transition-colors">
              {store.t('dup.optNumbered')}
            </span>
          </label>

          <!-- Option 2: Overwrite Existing File -->
          <label class="flex items-start gap-3 cursor-pointer group">
            <div class="pt-0.5">
              <input
                type="radio"
                name="duplicate-choice"
                value="overwrite"
                checked={selectedOption === 'overwrite'}
                onchange={() => selectedOption = 'overwrite'}
                class="sr-only"
              />
              <div class="w-4 h-4 rounded-full border transition-all flex items-center justify-center {selectedOption === 'overwrite' ? 'border-[#00e5ff] bg-[#00e5ff]/15' : 'border-[#4b5260] bg-[#090e16] group-hover:border-[#8c909f]'}">
                {#if selectedOption === 'overwrite'}
                  <div class="w-2 h-2 rounded-full bg-[#00e5ff] shadow-[0_0_8px_#00e5ff]"></div>
                {/if}
              </div>
            </div>
            <span class="font-sans text-xs text-[#dee2ee] leading-tight select-none group-hover:text-white transition-colors">
              {store.t('dup.optOverwrite')}
            </span>
          </label>

          <!-- Option 3: Complete / Resume (Default in IDM screenshot) -->
          <label class="flex items-start gap-3 cursor-pointer group">
            <div class="pt-0.5">
              <input
                type="radio"
                name="duplicate-choice"
                value="resume"
                checked={selectedOption === 'resume'}
                onchange={() => selectedOption = 'resume'}
                class="sr-only"
              />
              <div class="w-4 h-4 rounded-full border transition-all flex items-center justify-center {selectedOption === 'resume' ? 'border-[#00e5ff] bg-[#00e5ff]/15' : 'border-[#4b5260] bg-[#090e16] group-hover:border-[#8c909f]'}">
                {#if selectedOption === 'resume'}
                  <div class="w-2 h-2 rounded-full bg-[#00e5ff] shadow-[0_0_8px_#00e5ff]"></div>
                {/if}
              </div>
            </div>
            <span class="font-sans text-xs text-[#dee2ee] leading-tight select-none group-hover:text-white transition-colors">
              {store.t('dup.optCompleteOrResume')}
            </span>
          </label>
        </div>

        <!-- Buttons Row (OK and Batal centered/standard layout) -->
        <div class="pt-3 flex items-center justify-center gap-4">
          <button
            onclick={handleConfirm}
            class="min-w-[96px] h-8 px-5 rounded-lg bg-[#252a33] hover:bg-[#30353e] hover:border-[#00e5ff]/60 border border-[#30353e] text-[#dee2ee] hover:text-white text-xs font-semibold shadow-md active:scale-95 transition-all cursor-pointer"
            type="button"
          >
            {store.t('dup.btnOk')}
          </button>

          <button
            onclick={handleClose}
            class="min-w-[96px] h-8 px-5 rounded-lg bg-[#252a33] hover:bg-[#30353e] border border-[#30353e] text-[#dee2ee] hover:text-white text-xs font-medium active:scale-95 transition-all cursor-pointer"
            type="button"
          >
            {store.t('dup.btnCancel')}
          </button>
        </div>

        <!-- Remember Choice Checkbox (Bottom section) -->
        <div class="pt-2 border-t border-[#30353e]/60">
          <label class="flex items-start gap-2.5 cursor-pointer group">
            <div class="pt-0.5">
              <input
                type="checkbox"
                bind:checked={rememberChoice}
                class="sr-only"
              />
              <div class="w-4 h-4 rounded border transition-all flex items-center justify-center {rememberChoice ? 'bg-[#00e5ff] border-[#00e5ff] text-[#00363d]' : 'bg-[#090e16] border-[#4b5260] group-hover:border-[#8c909f]'}">
                {#if rememberChoice}
                  <Check class="w-3 h-3 stroke-[3]" />
                {/if}
              </div>
            </div>
            <div class="flex-1 min-w-0 space-y-0.5">
              <span class="font-sans text-xs text-[#dee2ee] leading-tight block group-hover:text-white transition-colors select-none">
                {store.t('dup.remember')}
              </span>
              <span class="font-sans text-[10px] text-[#8c909f] leading-tight block select-none">
                {store.t('dup.rememberSub')}
              </span>
            </div>
          </label>
        </div>
      </div>
    </div>
  </div>
{/if}
