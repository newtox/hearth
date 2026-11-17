<script lang="ts">
  import { LANGS, lang, setLang, t } from '../lib/i18n.svelte';
  import { api, base, isApp, setServer } from '../lib/api';
  import { login, resetPassword } from '../lib/store.svelte';
  import { THEMES, applyTheme, theme, w } from '../lib/theme.svelte';

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

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    busy = true;
    if (isApp) setServer(server);
    try {
      if (mode === 'login') await login('login', { login: username, password });
      else if (mode === 'register') await login('register', { username, email, password });
      else if (mode === 'forgot') {
        await api('POST', '/auth/forgot', { login: username, lang });
        sent = true;
      } else if (password !== repeat) error = t('Die neuen Passwörter stimmen nicht überein');
      else {
        await resetPassword(resetToken, password);
        history.replaceState(null, '', location.pathname);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : t('Fehler');
    }
    busy = false;
  }
</script>

<svelte:window onhashchange={hashchange} />

<main>
  <form class="panel stack" onsubmit={submit}>
    <div>
      <h1>Hearth</h1>
      <p class="muted">{theme.id === 'pergament' ? t('Tritt ans Feuer, Reisender.') : t('Dein Platz am Feuer.')}</p>
    </div>
    {#if mode === 'forgot' || mode === 'reset'}
      <h2>{mode === 'forgot' ? t('Passwort vergessen') : t('Neues Passwort festlegen')}</h2>
    {/if}
    {#if isApp && mode !== 'reset'}
      <label class="field">
        <span>{t('Server-Adresse')}</span>
        <input class="input" bind:value={server} placeholder="https://chat.example.de" inputmode="url" autocapitalize="off" required />
      </label>
    {/if}
    {#if mode === 'forgot' && sent}
      <p role="status">{t('Wenn es dieses Konto gibt, ist jetzt eine E-Mail mit einem Link unterwegs. Er gilt eine Stunde.')}</p>
    {:else if mode !== 'reset'}
    <label class="field">
      <span>{mode === 'register' ? t('Benutzername') : t('Benutzername oder E-Mail')}</span>
      <input class="input" bind:value={username} autocomplete="username" required />
    </label>
    {/if}
    {#if mode === 'register'}
      <label class="field">
        <span>{t('E-Mail')}</span>
        <input class="input" type="email" bind:value={email} autocomplete="email" required />
      </label>
    {/if}
    {#if mode !== 'forgot'}
      <label class="field">
        <span>{mode === 'reset' ? t('Neues Passwort') : t('Passwort')}</span>
        <input class="input" type="password" bind:value={password} autocomplete={mode === 'login' ? 'current-password' : 'new-password'} minlength="8" required />
      </label>
    {/if}
    {#if mode === 'reset'}
      <label class="field">
        <span>{t('Neues Passwort wiederholen')}</span>
        <input class="input" type="password" bind:value={repeat} autocomplete="new-password" minlength="8" required />
      </label>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if !(mode === 'forgot' && sent)}
      <button class="btn primary" disabled={busy}>{mode === 'login' ? w('login') : mode === 'register' ? w('register') : mode === 'forgot' ? t('Link senden') : t('Passwort speichern')}</button>
    {/if}
    {#if mode === 'login'}
      <button type="button" class="switch" onclick={() => ((mode = 'forgot'), (error = ''), (sent = false))}>{t('Passwort vergessen?')}</button>
    {/if}
    <button type="button" class="switch" onclick={() => ((mode = mode === 'login' ? 'register' : 'login'), (error = ''))}>
      {mode === 'login' ? t('Noch kein Konto? Konto erstellen') : t('Schon ein Konto? Anmelden')}
    </button>
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
