<script lang="ts">
  import { LANGS, fromEnglish, lang, setLang, t } from '../lib/i18n.svelte';
  import { api, asset, upload } from '../lib/api';
  import { customGames, detection } from '../lib/activity';
  import { play, sounds } from '../lib/sounds';
  import { BADGE_ICONS } from '../lib/badges';
  import BadgeIcon from './BadgeIcon.svelte';
  import { app, attempt, logout, toast, type Badge, type Connection, type User } from '../lib/store.svelte';
  import { THEMES, applyTheme, theme } from '../lib/theme.svelte';
  import { docs, releases } from '../lib/docs';
  import Avatar from './Avatar.svelte';
  import Modal from './Modal.svelte';

  const me = $derived(app.me!);
  let displayName = $state(app.me!.display_name);
  let bio = $state(app.me!.bio ?? '');
  let accent = $state(app.me!.accent_color ?? '#6b5bd6');

  async function save(e: SubmitEvent) {
    e.preventDefault();
    const u = await attempt(api<User>('PATCH', '/users/@me', { display_name: displayName, bio, accent_color: accent }));
    if (u) {
      app.me = u;
      toast(t('Profil gespeichert'));
    }
  }

  let badges = $state<Badge[]>([]);
  let badge = $state({ name: '', icon: 'svg:star', description: '', color: '#8b6cff' });
  let available = $state<{ id: string; name: string }[]>([]);
  let linked = $state<Connection[]>([]);

  async function loadConnections() {
    const res = await attempt(api<{ available: { id: string; name: string }[]; linked: Connection[] }>('GET', '/connections'));
    if (res) ({ available, linked } = res);
  }

  async function connectAccount(provider: string) {
    const res = await attempt(api<{ url: string }>('GET', provider === 'spotify' ? '/spotify' : `/connections/${provider}`));
    if (res) location.href = res.url;
  }

  async function disconnect(provider: string) {
    if (await attempt(api('DELETE', provider === 'spotify' ? '/spotify' : `/connections/${provider}`))) loadConnections();
  }

  loadConnections();

  async function loadBadges() {
    badges = (await attempt(api<Badge[]>('GET', '/badges'))) ?? [];
  }

  let kind = $state(app.me!.activity?.kind ?? 'playing');
  let activityName = $state(app.me!.activity?.name ?? '');
  let detect = $state(detection.enabled());
  let soundOn = $state(sounds.enabled());
  let games = $state(localStorage.getItem('hearth_games') ?? '');

  async function setActivity(e: SubmitEvent) {
    e.preventDefault();
    if (await attempt(api('POST', '/users/@me/activity', { kind, name: activityName }))) toast(t('Aktivität gesetzt'));
  }

  async function clearActivity() {
    await attempt(api('DELETE', '/users/@me/activity'));
    activityName = '';
  }

  function saveDetection() {
    detection.set(detect);
    localStorage.setItem('hearth_games', games);
    toast(t('{n} eigene Spiele gespeichert', { n: customGames().length }));
  }

  async function addBadge(e: SubmitEvent) {
    e.preventDefault();
    const created = await attempt(api<Badge>('POST', '/badges', badge));
    if (created) {
      badge = { name: '', icon: 'svg:star', description: '', color: '#8b6cff' };
      loadBadges();
    }
  }

  async function removeBadge(b: Badge) {
    if (confirm(t('Abzeichen „{name}“ löschen? Es wird allen entzogen.', { name: b.name }))) {
      await attempt(api('DELETE', `/badges/${b.id}`));
      loadBadges();
    }
  }

  if (app.me!.is_admin) loadBadges();

  async function pick(e: Event, kind: 'avatar' | 'banner') {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const u = await attempt(upload<User>(`/users/@me/${kind}`, file));
    if (u) app.me = u;
  }
</script>

<Modal title={t('Einstellungen')} onclose={() => (app.modal = null)} wide>
  <div class="grid">
    <form class="stack" onsubmit={save}>
      <h3>{t('Profil')}</h3>
      <label class="field">
        <span>{t('Anzeigename')}</span>
        <input class="input" bind:value={displayName} maxlength="32" required />
      </label>
      <label class="field">
        <span>{t('Über mich')} ({bio.length}/500)</span>
        <textarea class="input" rows="4" maxlength="500" bind:value={bio}></textarea>
      </label>
      <label class="field">
        <span>{t('Akzentfarbe (Bannerfarbe ohne Bild)')}</span>
        <input type="color" bind:value={accent} />
      </label>
      <div class="row wrap">
        <label class="btn">{t('Avatar ändern')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={(e) => pick(e, 'avatar')} hidden /></label>
        <label class="btn">{t('Banner ändern')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={(e) => pick(e, 'banner')} hidden /></label>
      </div>
      <button class="btn primary">{t('Profil speichern')}</button>
    </form>

    <div class="stack side">
      <div class="preview">
        <div class="banner" style:background-color={accent} style:background-image={me.banner ? `url(${asset(me.banner)})` : null}></div>
        <div class="who">
          <Avatar id={me.id} name={displayName || me.username} image={me.avatar} size={64} />
          <strong>{displayName}</strong>
          <span class="muted">@{me.username}</span>
          {#if bio}<p class="bio">{bio}</p>{/if}
        </div>
      </div>

      <h3>{t('Darstellung')}</h3>
      <div class="swatches" role="radiogroup" aria-label={t('Darstellung')}>
        {#each THEMES as th (th.id)}
          <button class="swatch" class:on={theme.id === th.id} role="radio" aria-checked={theme.id === th.id} onclick={() => applyTheme(th.id)}>
            <span class="chip" style:background={th.swatch}></span>
            <span class="label">{t(th.name)}</span>
          </button>
        {/each}
      </div>

      <h3>{t('Aktivität')}</h3>
      <form class="row wrap" onsubmit={setActivity}>
        <select class="input kind" bind:value={kind} aria-label={t('Art')}>
          <option value="playing">{t('Spielt')}</option>
          <option value="listening">{t('Hört')}</option>
          <option value="watching">{t('Schaut')}</option>
          <option value="custom">{t('Eigener Text')}</option>
        </select>
        <input class="input grow" bind:value={activityName} maxlength="64" placeholder={t('z. B. Tetris')} required />
        <button class="btn small">{t('Setzen')}</button>
        <button type="button" class="btn small" onclick={clearActivity}>{t('Entfernen')}</button>
      </form>
      {#if detection.available}
        <label class="row"><input type="checkbox" bind:checked={detect} /> {t('Laufende Spiele automatisch erkennen')}</label>
        <label class="field">
          <span>{t('Eigene Spiele, eine Zeile pro Spiel: prozessname=Spielname')}</span>
          <textarea class="input" rows="3" bind:value={games} placeholder="factorio=Factorio"></textarea>
        </label>
        <button class="btn small" onclick={saveDetection}>{t('Erkennung speichern')}</button>
      {/if}

      {#if available.length}
        <h3>{t('Verbindungen')}</h3>
        {#each available as p (p.id)}
          {@const link = linked.find((x) => x.provider === p.id)}
          <div class="row conn">
            <strong>{p.name}</strong>
            <span class="grow muted ellipsis">{link ? link.name : t('Nicht verbunden')}</span>
            {#if link}
              <button class="btn small" onclick={() => disconnect(p.id)}>{t('Trennen')}</button>
            {:else}
              <button class="btn small" onclick={() => connectAccount(p.id)}>{t('Verbinden')}</button>
            {/if}
          </div>
        {/each}
      {/if}

      <h3>{t('Töne')}</h3>
      <label class="row"><input type="checkbox" bind:checked={soundOn} onchange={() => (sounds.set(soundOn), play('message'))} /> {t('Töne für Nachrichten, Anrufe und Sprachkanäle')}</label>

      <h3>{t('Sprache')}</h3>
      <div class="row">
        {#each LANGS as l (l.id)}
          <button class="btn small" class:primary={lang === l.id} onclick={() => setLang(l.id)}>{l.name}</button>
        {/each}
      </div>

      <h3>{t('Hilfe')}</h3>
      <div class="row wrap help">
        <a class="btn small" href={docs('guide')} target="_blank" rel="noopener noreferrer">{t('Anleitung')}</a>
        <a class="btn small" href={releases} target="_blank" rel="noopener noreferrer">{t('Apps herunterladen')}</a>
        {#if me.is_admin}<a class="btn small" href={docs('installation')} target="_blank" rel="noopener noreferrer">{t('Server einrichten')}</a>{/if}
      </div>

      <h3>{t('Konto')}</h3>
      <p class="muted">{me.email}</p>
      <button class="btn danger" onclick={logout}>{t('Abmelden')}</button>
    </div>
  </div>
  {#if me.is_admin}
    <div class="stack admin">
      <h3>{t('Globale Abzeichen')}</h3>
      <p class="muted">{t('Nur für Instanz-Admins sichtbar. Vergeben werden Abzeichen im Profil eines Nutzers unter „Abzeichen vergeben“.')}</p>
      <div class="row wrap">
        {#each badges as b (b.id)}
          <span class="badge" style:--c={b.color} title={fromEnglish(b.description)}><BadgeIcon badge={b} size={16} /> {fromEnglish(b.name)}<button onclick={() => removeBadge(b)} aria-label={t('{name} löschen', { name: b.name })}>✕</button></span>
        {/each}
      </div>
      <form class="row wrap" onsubmit={addBadge}>
        <div class="icons" role="radiogroup" aria-label={t('Symbol')}>
          {#each Object.keys(BADGE_ICONS) as name}
            <button type="button" class="pick" class:on={badge.icon === 'svg:' + name} role="radio" aria-checked={badge.icon === 'svg:' + name} aria-label={name} onclick={() => (badge.icon = 'svg:' + name)}>
              <BadgeIcon badge={{ icon: 'svg:' + name, color: badge.color }} size={22} />
            </button>
          {/each}
        </div>
        <input class="input name" bind:value={badge.name} maxlength="32" placeholder={t('Name')} required />
        <input class="input grow" bind:value={badge.description} maxlength="120" placeholder={t('Beschreibung (optional)')} />
        <input type="color" bind:value={badge.color} aria-label={t('Farbe')} />
        <button class="btn">{t('Abzeichen anlegen')}</button>
      </form>
    </div>
  {/if}
</Modal>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
    align-items: start;
  }
  .wrap {
    flex-wrap: wrap;
  }
  input[type='color'] {
    width: 64px;
    height: 36px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    background: none;
  }
  .conn {
    padding: 4px 0;
  }
  .conn strong {
    min-width: 84px;
  }
  .kind {
    width: 130px;
  }
  .row .grow {
    min-width: 120px;
  }
  .admin {
    margin-top: 24px;
    padding-top: 20px;
    border-top: 1px solid var(--line);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px 2px 10px;
    border-radius: 99px;
    border: 1px solid var(--c);
    background: color-mix(in srgb, var(--c) 22%, transparent);
    font-size: 0.88em;
    font-weight: 600;
  }
  .badge button {
    padding: 0 4px;
    opacity: 0.7;
  }
  .icons {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    width: 100%;
  }
  .pick {
    padding: 6px;
    border-radius: var(--radius-s);
    border: 1px solid transparent;
  }
  .pick.on {
    border-color: var(--accent);
    background: var(--panel-2);
  }
  .input.name {
    width: 170px;
  }
  .preview {
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    overflow: hidden;
  }
  .banner {
    height: 84px;
    background-size: cover;
    background-position: center;
  }
  .who {
    display: grid;
    justify-items: start;
    gap: 2px;
    padding: 0 14px 14px;
    margin-top: -30px;
  }
  .bio {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    margin-top: 6px;
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .swatch {
    display: grid;
    justify-items: center;
    gap: 5px;
    font-size: 0.82em;
    color: var(--muted);
  }
  .chip {
    width: 52px;
    height: 62px;
    border-radius: 12px;
    border: 1px solid var(--line);
    outline: 3px solid transparent;
    outline-offset: 3px;
  }
  .swatch.on {
    color: var(--ink);
    font-weight: 600;
  }
  .swatch.on .chip {
    outline-color: var(--accent);
  }
  @media (max-width: 700px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
