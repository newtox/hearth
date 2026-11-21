<script lang="ts">
  import { LANGS, lang, setLang, t } from '../lib/i18n.svelte';
  import { api, base, isApp, setServer } from '../lib/api';
  import { login, loginSecondFactor, resetPassword } from '../lib/store.svelte';
  import { THEMES, applyTheme, theme, w } from '../lib/theme.svelte';

  const TAGLINES: Record<string, string> = {
    pergament: 'auth_step_up_fire_traveller',
    nordland: 'auth_tagline_nordland',
    neon: 'auth_tagline_neon',
    aurum: 'auth_tagline_aurum',
    hain: 'auth_tagline_grove',
  };

  const tokenFromHash = () => location.hash.match(/^#\/reset\/([0-9a-f]{64})/)?.[1] ?? '';
  let resetToken = $state(tokenFromHash());
  let mode = $state<'login' | 'register' | 'forgot' | 'reset'>(tokenFromHash() ? 'reset' : 'login');

  function hashchange() {
    const token = tokenFromHash();
    if (!token) return;
    resetToken = token;
    mode = 'reset';
    error = '';
  }
  let repeat = $state('');
  let sent = $state(false);
  let username = $state('');
  let email = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);
  let server = $state(base());
  let ticket = $state('');
  let code = $state('');

  function back() {
    ticket = '';
    code = '';
    error = '';
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    busy = true;
    if (isApp) setServer(server);
    try {
      if (ticket) await loginSecondFactor(ticket, code);
      else if (mode === 'login') ticket = (await login('login', { login: username, password })) ?? '';
      else if (mode === 'register') await login('register', { username, email, password, lang });
      else if (mode === 'forgot') {
        await api('POST', '/auth/forgot', { login: username, lang });
        sent = true;
      } else if (password !== repeat) error = t('common_new_passwords_do_not_match');
      else {
        ticket = (await resetPassword(resetToken, password)) ?? '';
        history.replaceState(null, '', location.pathname);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : t('common_error');
    }
    busy = false;
  }
</script>

<svelte:window onhashchange={hashchange} />

<main>
  <form class="panel stack" onsubmit={submit}>
    <div>
      <h1>Hearth</h1>
      <p class="muted">{t(TAGLINES[theme.id] ?? 'auth_place_fire')}</p>
    </div>
    {#if ticket}
      <h2>{t('auth_two_factor_title')}</h2>
      <p class="muted">{t('auth_two_factor_text')}</p>
      <label class="field">
        <span>{t('auth_two_factor_code')}</span>
        <input class="input code" bind:value={code} autocomplete="one-time-code" autocapitalize="off" spellcheck="false" maxlength="11" required />
      </label>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <button class="btn primary" disabled={busy}>{t('auth_two_factor_confirm')}</button>
      <button type="button" class="switch" onclick={back}>{t('auth_two_factor_back')}</button>
    {:else}
    {#if mode === 'forgot' || mode === 'reset'}
      <h2>{mode === 'forgot' ? t('auth_forgot_password') : t('auth_choose_new_password')}</h2>
    {/if}
    {#if isApp && mode !== 'reset'}
      <label class="field">
        <span>{t('auth_server_address')}</span>
        <input class="input" bind:value={server} placeholder="https://chat.example.de" inputmode="url" autocapitalize="off" required />
      </label>
    {/if}
    {#if mode === 'forgot' && sent}
      <p role="status">{t('auth_if_account_exists_email_link')}</p>
    {:else if mode !== 'reset'}
    <label class="field">
      <span>{mode === 'register' ? t('common_username') : t('auth_username_email')}</span>
      <input class="input" bind:value={username} autocomplete="username" required />
    </label>
    {/if}
    {#if mode === 'register'}
      <label class="field">
        <span>{t('common_email')}</span>
        <input class="input" type="email" bind:value={email} autocomplete="email" required />
      </label>
    {/if}
    {#if mode !== 'forgot'}
      <label class="field">
        <span>{mode === 'reset' ? t('common_new_password') : t('common_password')}</span>
        <input class="input" type="password" bind:value={password} autocomplete={mode === 'login' ? 'current-password' : 'new-password'} minlength="8" required />
      </label>
    {/if}
    {#if mode === 'reset'}
      <label class="field">
        <span>{t('common_repeat_new_password')}</span>
        <input class="input" type="password" bind:value={repeat} autocomplete="new-password" minlength="8" required />
      </label>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if !(mode === 'forgot' && sent)}
      <button class="btn primary" disabled={busy}>{mode === 'login' ? w('login') : mode === 'register' ? w('register') : mode === 'forgot' ? t('auth_send_link') : t('auth_save_password')}</button>
    {/if}
    {#if mode === 'login'}
      <button type="button" class="switch" onclick={() => ((mode = 'forgot'), (error = ''), (sent = false))}>{t('auth_forgot_password_question')}</button>
    {/if}
    <button type="button" class="switch" onclick={() => ((mode = mode === 'login' ? 'register' : 'login'), (error = ''))}>
      {mode === 'login' ? t('auth_no_account_yet_create_one') : t('auth_already_have_account_sign')}
    </button>
    {/if}
    <div class="row themes">
      {#each LANGS as l}
        <button type="button" class="btn small" class:primary={lang === l.id} onclick={() => setLang(l.id)}>{l.name}</button>
      {/each}
    </div>
    <div class="row themes second">
      {#each THEMES as th}
        <button type="button" class="dot" class:on={theme.id === th.id} style:background={th.swatch} onclick={() => applyTheme(th.id)} title={t(th.name)} aria-label={t(th.name)} aria-pressed={theme.id === th.id}></button>
      {/each}
    </div>
  </form>
</main>

<style>
  main {
    height: 100%;
    display: grid;
    place-items: center;
    padding: 16px;
    overflow: auto;
  }
  form {
    width: min(400px, 100%);
    padding: 28px;
  }
  h1 {
    font-size: 2.6rem;
  }
  .error {
    color: var(--danger);
  }
  .input.code {
    font-family: ui-monospace, 'Cascadia Code', 'Fira Code', monospace;
    letter-spacing: 0.15em;
  }
  .switch {
    text-decoration: underline;
    text-underline-offset: 3px;
    color: var(--muted);
  }
  .dot {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    border: 1px solid var(--line);
    outline: 2px solid transparent;
    outline-offset: 2px;
  }
  .dot.on {
    outline-color: var(--accent);
  }
  .themes.second {
    border-top: 0;
    padding-top: 0;
  }
  .themes {
    justify-content: center;
    padding-top: 6px;
    border-top: 1px solid var(--line);
  }
</style>
