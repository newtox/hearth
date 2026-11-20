<script lang="ts">
  import { renderSVG } from 'uqr';
  import { t } from '../lib/i18n.svelte';
  import { api } from '../lib/api';
  import { app, attempt, toast } from '../lib/store.svelte';
  import Modal from './Modal.svelte';
  import Icon from './Icon.svelte';

  type Step = null | 'password' | 'scan' | 'codes' | 'disable' | 'renew';

  const on = $derived(!!app.me?.two_factor);
  let step = $state<Step>(null);
  let password = $state('');
  let code = $state('');
  let setup = $state<{ secret: string; uri: string } | null>(null);
  let codes = $state<string[]>([]);
  let remaining = $state<number | null>(null);
  let busy = $state(false);

  const qr = $derived(setup ? renderSVG(setup.uri, { border: 2 }) : '');
  const grouped = $derived(setup?.secret.match(/.{1,4}/g)?.join(' ') ?? '');

  $effect(() => {
    if (on) loadRemaining();
  });

  async function loadRemaining() {
    const r = await attempt(api<{ remaining: number }>('GET', '/users/@me/2fa'));
    remaining = r?.remaining ?? null;
  }

  function open(next: Step) {
    password = '';
    code = '';
    setup = null;
    codes = [];
    step = next;
  }

  function close() {
    step = null;
    setup = null;
    password = '';
    code = '';
  }

  async function run<T>(p: Promise<T>) {
    busy = true;
    const r = await attempt(p);
    busy = false;
    return r;
  }

  async function start(e: SubmitEvent) {
    e.preventDefault();
    const r = await run(api<{ secret: string; uri: string }>('POST', '/users/@me/2fa/setup', { password }));
    if (!r) return;
    setup = r;
    password = '';
    step = 'scan';
  }

  async function confirm(e: SubmitEvent) {
    e.preventDefault();
    const r = await run(api<{ codes: string[] }>('POST', '/users/@me/2fa/enable', { code }));
    if (!r) return;
    codes = r.codes;
    app.me = { ...app.me!, two_factor: true };
    step = 'codes';
  }

  async function disable(e: SubmitEvent) {
    e.preventDefault();
    const r = await run(api('POST', '/users/@me/2fa/disable', { password, code }));
    if (r === undefined) return;
    app.me = { ...app.me!, two_factor: false };
    remaining = null;
    close();
    toast(t('two_factor_disabled'));
  }

  async function renew(e: SubmitEvent) {
    e.preventDefault();
    const r = await run(api<{ codes: string[] }>('POST', '/users/@me/2fa/codes', { password, code }));
    if (!r) return;
    codes = r.codes;
    step = 'codes';
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast(t('two_factor_copied'));
    } catch {
      toast(t('two_factor_copy_failed'));
    }
  }

  function finish() {
    close();
    loadRemaining();
  }
</script>

<h3>{t('two_factor_title')}</h3>
<div class="box">
  <span class="state" class:on><Icon name={on ? 'shield' : 'key'} size={20} /></span>
  <div class="grow">
    <strong>{on ? t('two_factor_on') : t('two_factor_off')}</strong>
    <p class="muted">{on ? t('two_factor_on_text') : t('two_factor_off_text')}</p>
    {#if on && remaining !== null}<p class="muted small">{t('two_factor_codes_left', { n: remaining })}</p>{/if}
  </div>
</div>
<div class="row wrap actions">
  {#if on}
    <button class="btn" onclick={() => open('renew')}>{t('two_factor_new_codes')}</button>
    <button class="btn danger" onclick={() => open('disable')}>{t('two_factor_disable')}</button>
  {:else}
    <button class="btn primary" onclick={() => open('password')}>{t('two_factor_enable')}</button>
  {/if}
</div>

{#if step}
  <Modal title={step === 'disable' ? t('two_factor_disable') : step === 'renew' ? t('two_factor_new_codes') : t('two_factor_enable')} onclose={step === 'codes' ? finish : close}>
    {#if step === 'password'}
      <form class="stack" onsubmit={start}>
        <p class="muted">{t('two_factor_step_password')}</p>
        <label class="field"><span>{t('user_settings_current_password')}</span><input class="input" type="password" bind:value={password} autocomplete="current-password" required /></label>
        <div class="row end">
          <button type="button" class="btn" onclick={close}>{t('common_cancel')}</button>
          <button class="btn primary" disabled={busy}>{t('two_factor_continue')}</button>
        </div>
      </form>
    {:else if step === 'scan' && setup}
      <form class="stack" onsubmit={confirm}>
        <p class="muted">{t('two_factor_step_scan')}</p>
        <div class="qr" aria-label={t('two_factor_qr')} role="img">{@html qr}</div>
        <div class="secret">
          <span class="muted small">{t('two_factor_manual')}</span>
          <code>{grouped}</code>
          <button type="button" class="btn small" onclick={() => copy(setup!.secret)}>{t('two_factor_copy')}</button>
        </div>
        <label class="field"><span>{t('two_factor_code_from_app')}</span><input class="input code" bind:value={code} inputmode="numeric" autocomplete="one-time-code" minlength="6" maxlength="7" required /></label>
        <div class="row end">
          <button type="button" class="btn" onclick={close}>{t('common_cancel')}</button>
          <button class="btn primary" disabled={busy}>{t('two_factor_activate')}</button>
        </div>
      </form>
    {:else if step === 'codes'}
      <div class="stack">
        <p>{t('two_factor_codes_text')}</p>
        <ol class="codes">
          {#each codes as c (c)}<li><code>{c}</code></li>{/each}
        </ol>
        <div class="row end">
          <button type="button" class="btn" onclick={() => copy(codes.join('\n'))}>{t('two_factor_copy_all')}</button>
          <button type="button" class="btn primary" onclick={finish}>{t('two_factor_saved_them')}</button>
        </div>
      </div>
    {:else}
      <form class="stack" onsubmit={step === 'disable' ? disable : renew}>
        <p class="muted">{step === 'disable' ? t('two_factor_disable_text') : t('two_factor_renew_text')}</p>
        <label class="field"><span>{t('user_settings_current_password')}</span><input class="input" type="password" bind:value={password} autocomplete="current-password" required /></label>
        <label class="field"><span>{t('auth_two_factor_code')}</span><input class="input code" bind:value={code} autocomplete="one-time-code" maxlength="11" required /></label>
        <div class="row end">
          <button type="button" class="btn" onclick={close}>{t('common_cancel')}</button>
          <button class="btn" class:danger={step === 'disable'} class:primary={step === 'renew'} disabled={busy}>{step === 'disable' ? t('two_factor_disable') : t('two_factor_new_codes')}</button>
        </div>
      </form>
    {/if}
  </Modal>
{/if}

<style>
  h3 {
    margin: 24px 0 10px;
  }
  .box {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    max-width: 560px;
    padding: 12px 14px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
  .box p {
    margin: 2px 0 0;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .state {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    flex: none;
    border-radius: 50%;
    background: var(--panel);
    color: var(--muted);
  }
  .state.on {
    color: var(--ok);
  }
  .actions {
    gap: 8px;
    margin-top: 10px;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .end {
    justify-content: flex-end;
  }
  .small {
    font-size: 0.85em;
  }
  .qr {
    width: 200px;
    height: 200px;
    margin: 0 auto;
    padding: 6px;
    border-radius: var(--radius-s);
    background: #fff;
  }
  .qr :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .secret {
    display: grid;
    gap: 6px;
    justify-items: start;
  }
  code {
    font-family: ui-monospace, 'Cascadia Code', 'Fira Code', monospace;
    font-size: 0.95em;
    letter-spacing: 0.04em;
    overflow-wrap: anywhere;
  }
  .input.code {
    font-family: ui-monospace, 'Cascadia Code', 'Fira Code', monospace;
    letter-spacing: 0.15em;
  }
  .codes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 18px;
    margin: 0;
    padding: 12px 14px 12px 34px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
</style>
