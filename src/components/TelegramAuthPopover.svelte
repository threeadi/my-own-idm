<script lang="ts">
  import { store } from '$lib/idmStore.svelte';
  import { Send, Shield, Key, Phone, MessageSquare, Bot, X, Check, Loader2, Plus, Sparkles, Trash2, UserCheck, ShieldCheck } from '@lucide/svelte';

  let channelInput = $state('');
  let authTab = $state<'accounts' | 'channel' | 'credentials' | 'otp' | 'bot'>('accounts');

  let apiId = $state('');
  let apiHash = $state('');
  let phoneNumber = $state('+62');
  let otpCode = $state('');
  let phoneCodeHash = $state('');
  let botToken = $state('');

  let isLoading = $state(false);
  let errorMsg = $state<string | null>(null);

  function handleAddChannel() {
    if (!channelInput.trim()) return;
    const channelName = channelInput.trim().replace(/^@/, '');
    store.selectTelegramChannel(`custom_${channelName}`, channelName);
    store.isTelegramPopoverOpen = false;
    channelInput = '';
  }

  async function handleSaveCredentials() {
    if (!apiId.trim() || !apiHash.trim()) {
      errorMsg = 'Isi API ID & API Hash (my.telegram.org)';
      return;
    }
    errorMsg = null;
    isLoading = true;
    try {
      await store.saveTelegramCredentials(apiId, apiHash);
      authTab = 'otp';
    } catch (e: any) {
      errorMsg = String(e);
    } finally {
      isLoading = false;
    }
  }

  async function handleRequestOtp() {
    if (!phoneNumber.trim() || phoneNumber.length < 5) {
      errorMsg = 'Nomor telepon tidak valid';
      return;
    }
    errorMsg = null;
    isLoading = true;
    try {
      phoneCodeHash = await store.requestTelegramOtp(phoneNumber);
      authTab = 'otp';
    } catch (e: any) {
      errorMsg = String(e);
    } finally {
      isLoading = false;
    }
  }

  async function handleVerifyOtp() {
    if (!otpCode.trim()) {
      errorMsg = 'Masukkan kode OTP';
      return;
    }
    errorMsg = null;
    isLoading = true;
    try {
      await store.verifyTelegramOtp(phoneNumber, otpCode, phoneCodeHash);
      authTab = 'accounts';
      store.isTelegramPopoverOpen = false;
    } catch (e: any) {
      errorMsg = String(e);
    } finally {
      isLoading = false;
    }
  }

  async function handleBotLogin() {
    if (!botToken.trim()) return;
    errorMsg = null;
    isLoading = true;
    try {
      await store.loginTelegramBot(botToken);
      authTab = 'accounts';
      store.isTelegramPopoverOpen = false;
    } catch (e: any) {
      errorMsg = String(e);
    } finally {
      isLoading = false;
    }
  }
</script>

{#if store.isTelegramPopoverOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-[3px] animate-in fade-in duration-150"
    onclick={(e) => { if (e.target === e.currentTarget) store.isTelegramPopoverOpen = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') store.isTelegramPopoverOpen = false; }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="w-full max-w-md bg-[#1b2028]/95 backdrop-blur-2xl rounded-2xl shadow-[0_16px_48px_rgba(0,0,0,0.8)] border border-[#30353e] p-5 text-[#dee2ee] animate-in fade-in zoom-in-95 duration-150">
    <div class="flex items-center justify-between border-b border-[#30353e]/80 pb-2.5 mb-3">
      <div class="flex items-center gap-2">
        <UserCheck class="w-4 h-4 text-[#4cd7f6]" />
        <span class="font-sans text-xs font-bold text-[#dee2ee]">{store.t('telegram.authModalTitle')}</span>
      </div>
      <button
        onclick={() => store.toggleTelegramPopover()}
        class="w-6 h-6 rounded flex items-center justify-center text-[#8c909f] hover:bg-[#30353e] hover:text-[#dee2ee] transition-colors cursor-pointer"
        type="button"
      >
        <X class="w-3.5 h-3.5" />
      </button>
    </div>

    <!-- Mode Selector Tabs (Strictly Account Management) -->
    <div class="grid grid-cols-3 gap-1 p-1 bg-[#090e16] rounded-xl border border-[#30353e] text-[11px] font-semibold mb-3">
      <button
        onclick={() => { authTab = 'accounts'; errorMsg = null; }}
        class={`py-1.5 px-2 rounded-lg text-center transition-all cursor-pointer ${authTab === 'accounts' ? 'bg-[#4cd7f6]/20 text-[#4cd7f6]' : 'text-[#8c909f] hover:text-[#dee2ee]'}`}
        type="button"
      >
        {store.t('telegram.tabAccounts', { count: store.telegramAccounts.length })}
      </button>
      <button
        onclick={() => { authTab = 'credentials'; errorMsg = null; }}
        class={`py-1.5 px-2 rounded-lg text-center transition-all cursor-pointer ${authTab === 'credentials' || authTab === 'otp' ? 'bg-[#4d8eff]/20 text-[#4d8eff]' : 'text-[#8c909f] hover:text-[#dee2ee]'}`}
        type="button"
      >
        {store.t('telegram.tabPhoneOtp')}
      </button>
      <button
        onclick={() => { authTab = 'bot'; errorMsg = null; }}
        class={`py-1.5 px-2 rounded-lg text-center transition-all cursor-pointer ${authTab === 'bot' ? 'bg-[#4edea3]/20 text-[#4edea3]' : 'text-[#8c909f] hover:text-[#dee2ee]'}`}
        type="button"
      >
        {store.t('telegram.tabBotToken')}
      </button>
    </div>

    {#if errorMsg}
      <div class="p-2 mb-3 rounded-lg bg-[#ff5252]/10 border border-[#ff5252]/30 text-[11px] text-[#ffb4ab]">
        {errorMsg}
      </div>
    {/if}

    {#if authTab === 'accounts'}
      <div class="space-y-2">
        <div class="flex items-center justify-between text-[11px] text-[#8c909f]">
          <span>{store.t('telegram.tabAccounts', { count: store.telegramAccounts.length })}</span>
          <button
            onclick={() => { authTab = 'credentials'; }}
            class="text-[#4cd7f6] hover:underline font-semibold cursor-pointer"
            type="button"
          >
            {store.t('telegram.loginAccount')}
          </button>
        </div>

        <div class="space-y-1.5 max-h-48 overflow-y-auto pr-1">
          {#each store.telegramAccounts as acc (acc.account_id)}
            {@const isActive = acc.is_active || store.activeAccountId === acc.account_id}
            <div class={`p-2.5 rounded-xl border flex items-center justify-between gap-2 transition-all ${isActive ? 'bg-[#4cd7f6]/10 border-[#4cd7f6]/40' : 'bg-[#090e16] border-[#30353e]'}`}>
              <div class="flex items-center gap-2 min-w-0">
                <div class={`w-7 h-7 rounded-lg flex items-center justify-center shrink-0 ${acc.account_type === 'bot' ? 'bg-[#4edea3]/20 text-[#4edea3]' : 'bg-[#4cd7f6]/20 text-[#4cd7f6]'}`}>
                  {#if acc.account_type === 'bot'}
                    <Bot class="w-4 h-4" />
                  {:else}
                    <UserCheck class="w-4 h-4" />
                  {/if}
                </div>
                <div class="flex flex-col min-w-0">
                  <span class="font-sans text-xs font-semibold text-[#dee2ee] truncate">
                    {acc.phone_number || acc.username || acc.account_id}
                  </span>
                  <span class="font-mono text-[10px] text-[#8c909f] truncate">
                    {acc.account_type === 'bot' ? 'Telegram Bot' : 'User MTProto Session'}
                  </span>
                </div>
              </div>

              <div class="flex items-center gap-1 shrink-0">
                {#if isActive}
                  <span class="px-2 py-0.5 rounded bg-[#4cd7f6]/20 text-[#4cd7f6] font-mono text-[10px] font-bold">
                    {store.t('telegram.activeBadge')}
                  </span>
                {:else}
                  <button
                    onclick={() => store.switchTelegramAccount(acc.account_id)}
                    class="px-2 py-0.5 rounded bg-[#252a33] text-[#8c909f] hover:text-[#4cd7f6] hover:bg-[#30353e] font-mono text-[10px] font-medium transition-colors cursor-pointer"
                    type="button"
                  >
                    {store.t('telegram.switchAccount')}
                  </button>
                {/if}
                <button
                  onclick={() => store.removeTelegramAccount(acc.account_id)}
                  class="p-1 rounded text-[#8c909f] hover:text-[#ff5252] hover:bg-[#ff5252]/10 transition-colors cursor-pointer"
                  title="Hapus Akun Ini"
                  type="button"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          {/each}

          {#if store.telegramAccounts.length === 0}
            <div class="p-4 text-center rounded-xl bg-[#090e16] border border-[#30353e] text-xs text-[#8c909f]">
              {store.t('telegram.noAccount')}
            </div>
          {/if}
        </div>
      </div>
    {:else if authTab === 'credentials'}
      <div class="space-y-2.5">
        <input
          type="text"
          bind:value={apiId}
          placeholder={store.t('telegram.apiIdLabel')}
          class="w-full h-8 px-3 bg-[#090e16] border border-[#30353e] rounded-xl text-xs font-mono text-[#dee2ee]"
        />
        <input
          type="password"
          bind:value={apiHash}
          placeholder={store.t('telegram.apiHashLabel')}
          class="w-full h-8 px-3 bg-[#090e16] border border-[#30353e] rounded-xl text-xs font-mono text-[#dee2ee]"
        />
        <input
          type="text"
          bind:value={phoneNumber}
          placeholder={store.t('telegram.phoneLabel')}
          class="w-full h-8 px-3 bg-[#090e16] border border-[#30353e] rounded-xl text-xs font-mono text-[#dee2ee]"
        />
        <button
          onclick={handleRequestOtp}
          disabled={isLoading}
          class="w-full h-9 rounded-xl bg-[#4d8eff] text-white font-bold text-xs flex items-center justify-center gap-1.5 hover:bg-[#3b82f6] transition-all cursor-pointer disabled:opacity-50"
          type="button"
        >
          {#if isLoading}
            <Loader2 class="w-3.5 h-3.5 animate-spin" />
          {:else}
            <span>{store.t('telegram.sendOtpBtn')}</span>
          {/if}
        </button>
      </div>
    {:else if authTab === 'otp'}
      <div class="space-y-2.5">
        <p class="text-[11px] text-[#4edea3]">{store.t('telegram.otpSentDesc')}</p>
        <input
          type="text"
          bind:value={otpCode}
          placeholder="12345"
          maxlength="6"
          class="w-full h-9 px-3 bg-[#090e16] border border-[#4edea3]/40 rounded-xl text-center font-mono text-base tracking-[0.4em] text-[#4edea3]"
        />
        <button
          onclick={handleVerifyOtp}
          disabled={isLoading}
          class="w-full h-9 rounded-xl bg-[#10b981] text-white font-bold text-xs flex items-center justify-center gap-1.5 hover:bg-[#059669] transition-all cursor-pointer disabled:opacity-50"
          type="button"
        >
          {#if isLoading}
            <Loader2 class="w-3.5 h-3.5 animate-spin" />
          {:else}
            <Check class="w-3.5 h-3.5" />
            <span>{store.t('telegram.verifyLoginBtn')}</span>
          {/if}
        </button>
      </div>
    {:else}
      <div class="space-y-2.5">
        <input
          type="password"
          bind:value={botToken}
          placeholder={store.t('telegram.botTokenLabel')}
          class="w-full h-9 px-3 bg-[#090e16] border border-[#30353e] rounded-xl text-xs font-mono text-[#dee2ee]"
        />
        <button
          onclick={handleBotLogin}
          disabled={isLoading}
          class="w-full h-9 rounded-xl bg-[#10b981] text-white font-bold text-xs flex items-center justify-center gap-1.5 hover:bg-[#059669] transition-all cursor-pointer disabled:opacity-50"
          type="button"
        >
          {#if isLoading}
            <Loader2 class="w-3.5 h-3.5 animate-spin" />
          {:else}
            <Check class="w-3.5 h-3.5" />
            <span>{store.t('telegram.botLoginBtn')}</span>
          {/if}
        </button>
      </div>
    {/if}
  </div>
</div>
{/if}

