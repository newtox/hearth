<script lang="ts">
  import { LANGS, lang, setLang, t } from '../lib/i18n.svelte';
  import { base, isApp, setServer } from '../lib/api';
  import { login } from '../lib/store.svelte';
  import { THEMES, applyTheme, theme, w } from '../lib/theme.svelte';

  let mode = $state<'login' | 'register'>('login');
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
      else await login('register', { username, email, password });
    } catch (err) {
      error = err instanceof Error ? err.message : t('Fehler');
    }
    busy = false;
  }
</script>

<main>
  <form class="panel stack" onsubmit={submit}>
    <div>
      <h1>Hearth</h1>
      <p class="muted">{theme.id === 'pergament' ? t('Tritt ans Feuer, Reisender.') : t('Dein Platz am Feuer.')}</p>
    </div>
    {#if isApp}
      <label class="field">
        <span>{t('Server-Adresse')}</span>
        <input class="input" bind:value={server} placeholder="https://chat.example.de" inputmode="url" autocapitalize="off" required />
      </label>
    {/if}
    <label class="field">
      <span>{mode === 'login' ? t('Benutzername oder E-Mail') : t('Benutzername')}</span>
      <input class="input" bind:value={username} autocomplete="username" required />
    </label>
    {#if mode === 'register'}
      <label class="field">
        <span>{t('E-Mail')}</span>
        <input class="input" type="email" bind:value={email} autocomplete="email" required />
      </label>
    {/if}
    <label class="field">
      <span>{t('Passwort')}</span>
      <input class="input" type="password" bind:value={password} autocomplete={mode === 'login' ? 'current-password' : 'new-password'} minlength="8" required />
    </label>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <button class="btn primary" disabled={busy}>{mode === 'login' ? w('login') : w('register')}</button>
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
