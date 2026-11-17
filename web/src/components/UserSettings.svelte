<script lang="ts">
  import { ask } from '../lib/dialog.svelte';
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
  import SettingsPage, { type Section } from './SettingsPage.svelte';
  import Modal from './Modal.svelte';
  import { notifications } from '../lib/notify';
  import { setStatus } from '../lib/store.svelte';

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
    const res = await attempt(api<{ url: string }>('GET', '/spotify'));
    if (res) location.href = res.url;
  }

  async function disconnect(provider: string) {
    if (await attempt(api('DELETE', '/spotify'))) loadConnections();
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
    if (await ask({ title: t('Abzeichen löschen'), text: t('„{name}“ wird gelöscht und allen entzogen.', { name: b.name }), confirm: t('Löschen'), danger: true })) {
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

  let section = $state('account');
  const sections = $derived<Section[]>([
    { id: 'account', name: t('Mein Konto'), group: t('Benutzereinstellungen') },
    { id: 'profile', name: t('Profil'), group: t('Benutzereinstellungen') },
    { id: 'connections', name: t('Verbindungen'), group: t('Benutzereinstellungen') },
    { id: 'appearance', name: t('Darstellung'), group: t('App-Einstellungen') },
    { id: 'language', name: t('Sprache'), group: t('App-Einstellungen') },
    { id: 'notifications', name: t('Benachrichtigungen'), group: t('App-Einstellungen') },
    { id: 'activity', name: t('Aktivität'), group: t('App-Einstellungen') },
    ...(app.me!.is_admin ? [{ id: 'badges', name: t('Globale Abzeichen'), group: t('Instanz') }] : []),
    { id: 'help', name: t('Hilfe') },
    { id: 'logout', name: t('Abmelden'), danger: true, action: async () => (await ask({ title: t('Abmelden'), text: t('Willst du dich wirklich abmelden?'), confirm: t('Abmelden'), danger: true })) && logout() },
  ]);

  let showEmail = $state(false);
  let editing = $state<null | 'username' | 'email' | 'password'>(null);
  let form = $state({ value: '', password: '', next: '', repeat: '' });

  function edit(what: 'username' | 'email' | 'password') {
    form = { value: what === 'username' ? me.username : what === 'email' ? (me.email ?? '') : '', password: '', next: '', repeat: '' };
    editing = what;
  }

  async function saveAccount(e: SubmitEvent) {
    e.preventDefault();
    const body: Record<string, string> = { password: form.password };
    if (editing === 'password') {
      if (form.next !== form.repeat) return toast(t('Die neuen Passwörter stimmen nicht überein'));
      body.new_password = form.next;
    } else if (editing) body[editing] = form.value;
    const u = await attempt(api<User>('PATCH', '/users/@me/account', body));
    if (u) {
      app.me = { ...app.me!, ...u };
      editing = null;
      toast(t('Konto gespeichert'));
    }
  }

  function masked(email: string) {
    const [name, domain] = email.split('@');
    return `${'*'.repeat(Math.max(3, name.length))}@${domain ?? ''}`;
  }

  let notifyOn = $state(notifications.enabled());

  async function toggleNotify() {
    if (notifyOn) {
      notifications.disable();
      notifyOn = false;
    } else {
      notifyOn = await notifications.enable();
      if (!notifyOn) toast(t('Der Browser hat Benachrichtigungen blockiert. Erlaube sie in den Seiteneinstellungen.'));
    }
  }
</script>

<SettingsPage title={t('Einstellungen')} {sections} bind:current={section} onclose={() => (app.modal = null)}>
  {#if section === 'account'}
    <div class="card account">
      <div class="banner" style:background-color={me.accent_color ?? 'var(--panel-2)'} style:background-image={me.banner ? `url(${asset(me.banner)})` : null}></div>
      <div class="row who-row">
        <Avatar id={me.id} name={me.display_name} image={me.avatar} size={72} status={me.status ?? 'online'} />
        <div class="grow">
          <strong class="big">{me.display_name}</strong>
          <p class="muted">@{me.username}</p>
        </div>
        <button class="btn small" onclick={() => (section = 'profile')}>{t('Profil bearbeiten')}</button>
      </div>
      <dl class="facts">
        <div class="fact">
          <div class="grow"><dt>{t('Benutzername')}</dt><dd>{me.username}</dd></div>
          <button class="btn small" onclick={() => edit('username')}>{t('Bearbeiten')}</button>
        </div>
        <div class="fact">
          <div class="grow">
            <dt>{t('E-Mail')}</dt>
            <dd>{showEmail ? me.email : masked(me.email ?? '')} <button class="link" onclick={() => (showEmail = !showEmail)}>{showEmail ? t('Verbergen') : t('Anzeigen')}</button></dd>
          </div>
          <button class="btn small" onclick={() => edit('email')}>{t('Bearbeiten')}</button>
        </div>
        <div><dt>{t('Status')}</dt><dd>
          <select class="input" value={me.status ?? 'online'} onchange={(e) => setStatus(e.currentTarget.value as any)}>
            <option value="online">{t('Online')}</option>
            <option value="idle">{t('Abwesend')}</option>
            <option value="dnd">{t('Bitte nicht stören')}</option>
            <option value="invisible">{t('Unsichtbar')}</option>
          </select>
        </dd></div>
      </dl>
    </div>

    <h3>{t('Passwort')}</h3>
    <div><button class="btn primary" onclick={() => edit('password')}>{t('Passwort ändern')}</button></div>
    {#if editing}
      <Modal title={editing === 'username' ? t('Benutzername ändern') : editing === 'email' ? t('E-Mail ändern') : t('Passwort ändern')} onclose={() => (editing = null)}>
        <form class="stack" onsubmit={saveAccount}>
          {#if editing === 'username'}
            <label class="field"><span>{t('Benutzername')}</span><input class="input" bind:value={form.value} maxlength="32" autocomplete="username" required /></label>
          {:else if editing === 'email'}
            <label class="field"><span>{t('E-Mail')}</span><input class="input" type="email" bind:value={form.value} maxlength="254" autocomplete="email" required /></label>
          {:else}
            <label class="field"><span>{t('Neues Passwort')}</span><input class="input" type="password" bind:value={form.next} minlength="8" maxlength="128" autocomplete="new-password" required /></label>
            <label class="field"><span>{t('Neues Passwort wiederholen')}</span><input class="input" type="password" bind:value={form.repeat} maxlength="128" autocomplete="new-password" required /></label>
            <p class="muted small">{t('Nach einem neuen Passwort werden alle anderen Geräte abgemeldet.')}</p>
          {/if}
          <label class="field"><span>{t('Aktuelles Passwort')}</span><input class="input" type="password" bind:value={form.password} autocomplete="current-password" required /></label>
          <div class="row end">
            <button type="button" class="btn" onclick={() => (editing = null)}>{t('Abbrechen')}</button>
            <button class="btn primary">{t('Speichern')}</button>
          </div>
        </form>
      </Modal>
    {/if}
  {:else if section === 'profile'}
    <div class="grid">
      <form class="stack" onsubmit={save}>
        <label class="field">
          <span>{t('Anzeigename')}</span>
          <input class="input" bind:value={displayName} maxlength="32" required />
        </label>
        <label class="field">
          <span>{t('Über mich')} ({bio.length}/500)</span>
          <textarea class="input" rows="5" maxlength="500" bind:value={bio}></textarea>
        </label>
        <label class="field">
          <span>{t('Akzentfarbe (Bannerfarbe ohne Bild)')}</span>
          <input type="color" bind:value={accent} />
        </label>
        <div class="row wrap">
          <label class="btn">{t('Avatar ändern')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={(e) => pick(e, 'avatar')} hidden /></label>
          <label class="btn">{t('Banner ändern')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={(e) => pick(e, 'banner')} hidden /></label>
        </div>
        <div><button class="btn primary">{t('Profil speichern')}</button></div>
      </form>
      <div class="stack">
        <span class="muted small">{t('Vorschau')}</span>
        <div class="preview">
          <div class="banner" style:background-color={accent} style:background-image={me.banner ? `url(${asset(me.banner)})` : null}></div>
          <div class="who">
            <Avatar id={me.id} name={displayName || me.username} image={me.avatar} size={64} />
            <strong>{displayName}</strong>
            <span class="muted">@{me.username}</span>
            {#if bio}<p class="bio">{bio}</p>{/if}
          </div>
        </div>
      </div>
    </div>
  {:else if section === 'connections'}
    <p class="muted">{t('Mit Spotify erscheint der laufende Titel als deine Aktivität und das Konto in deinem Profil.')}</p>
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
    {:else}
      <p class="muted">{t('Spotify ist auf diesem Server nicht eingerichtet.')}</p>
    {/each}
  {:else if section === 'appearance'}
    <div class="swatches" role="radiogroup" aria-label={t('Darstellung')}>
      {#each THEMES as th (th.id)}
        <button class="swatch" class:on={theme.id === th.id} role="radio" aria-checked={theme.id === th.id} onclick={() => applyTheme(th.id)}>
          <span class="chip" style:background={th.swatch}></span>
          <span class="label">{t(th.name)}</span>
        </button>
      {/each}
    </div>
  {:else if section === 'language'}
    <div class="stack langs" role="radiogroup" aria-label={t('Sprache')}>
      {#each LANGS as l (l.id)}
        <label class="option"><input type="radio" name="lang" checked={lang === l.id} onchange={() => setLang(l.id)} /> {l.name}</label>
      {/each}
    </div>
  {:else if section === 'notifications'}
    <h3>{t('Desktop-Benachrichtigungen')}</h3>
    <p class="muted">{t('Für Direktnachrichten, Gruppen, Erwähnungen und Antworten auf dich, solange das Fenster nicht im Vordergrund ist. Bei „Bitte nicht stören“ bleiben sie aus.')}</p>
    {#if notifications.supported}
      <label class="row"><input type="checkbox" checked={notifyOn} onchange={toggleNotify} /> {t('Desktop-Benachrichtigungen anzeigen')}</label>
    {:else}
      <p class="muted">{t('Dieser Browser unterstützt keine Benachrichtigungen.')}</p>
    {/if}
    <h3>{t('Töne')}</h3>
    <label class="row"><input type="checkbox" bind:checked={soundOn} onchange={() => (sounds.set(soundOn), play('message'))} /> {t('Töne für Nachrichten, Anrufe und Sprachkanäle')}</label>
  {:else if section === 'activity'}
    <h3>{t('Aktivität von Hand setzen')}</h3>
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
    <h3>{t('Spiele automatisch erkennen')}</h3>
    {#if detection.available}
      <label class="row"><input type="checkbox" bind:checked={detect} /> {t('Laufende Spiele automatisch erkennen')}</label>
      <label class="field">
        <span>{t('Eigene Spiele, eine Zeile pro Spiel: prozessname=Spielname')}</span>
        <textarea class="input" rows="4" bind:value={games} placeholder="factorio=Factorio"></textarea>
      </label>
      <div><button class="btn small" onclick={saveDetection}>{t('Erkennung speichern')}</button></div>
    {:else}
      <p class="muted">{t('Nur in der Desktop-App für Windows und Linux. Sie schaut alle 20 Sekunden nach laufenden Spielen und setzt sie als Aktivität.')}</p>
      <a class="btn small" href={releases} target="_blank" rel="noopener noreferrer">{t('Apps herunterladen')}</a>
    {/if}
  {:else if section === 'badges'}
    <p class="muted">{t('Nur für Instanz-Admins sichtbar. Vergeben werden Abzeichen im Profil eines Nutzers unter „Abzeichen vergeben“.')}</p>
    <div class="row wrap">
      {#each badges as b (b.id)}
        <span class="badge" style:--c={b.color} title={fromEnglish(b.description)}><BadgeIcon badge={b} size={16} /> {fromEnglish(b.name)}<button onclick={() => removeBadge(b)} aria-label={t('{name} löschen', { name: b.name })}>✕</button></span>
      {/each}
    </div>
    <h3>{t('Abzeichen anlegen')}</h3>
    <form class="stack" onsubmit={addBadge}>
      <div class="icons" role="radiogroup" aria-label={t('Symbol')}>
        {#each Object.keys(BADGE_ICONS) as name}
          <button type="button" class="pick" class:on={badge.icon === 'svg:' + name} role="radio" aria-checked={badge.icon === 'svg:' + name} aria-label={name} onclick={() => (badge.icon = 'svg:' + name)}>
            <BadgeIcon badge={{ icon: 'svg:' + name, color: badge.color }} size={22} />
          </button>
        {/each}
      </div>
      <div class="row wrap">
        <input class="input name" bind:value={badge.name} maxlength="32" placeholder={t('Name')} required />
        <input class="input grow" bind:value={badge.description} maxlength="120" placeholder={t('Beschreibung (optional)')} />
        <input type="color" bind:value={badge.color} aria-label={t('Farbe')} />
      </div>
      <div><button class="btn">{t('Abzeichen anlegen')}</button></div>
    </form>
  {:else if section === 'help'}
    <div class="stack">
      <a class="btn" href={docs('guide')} target="_blank" rel="noopener noreferrer">{t('Anleitung')}</a>
      <a class="btn" href={releases} target="_blank" rel="noopener noreferrer">{t('Apps herunterladen')}</a>
      {#if me.is_admin}<a class="btn" href={docs('installation')} target="_blank" rel="noopener noreferrer">{t('Server einrichten')}</a>{/if}
    </div>
  {/if}
</SettingsPage>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
    align-items: start;
  }
  h3 {
    margin: 24px 0 10px;
  }
  .small {
    font-size: 0.86em;
    margin: 0;
  }
  .card {
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--panel-2);
  }
  .card .banner {
    height: 96px;
  }
  .who-row {
    flex-wrap: wrap;
    gap: 14px;
    padding: 0 18px;
    margin-top: -26px;
    align-items: flex-end;
  }
  .who-row p {
    margin: 0;
  }
  .big {
    font-size: 1.2em;
  }
  .facts {
    margin: 16px;
    padding: 6px 16px;
    border-radius: var(--radius-s);
    background: var(--panel);
  }
  .facts > div {
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
  }
  .facts > div:last-child {
    border-bottom: 0;
  }
  .fact {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .end {
    justify-content: flex-end;
  }
  dt {
    font-size: 0.78em;
    font-weight: 700;
    color: var(--muted);
  }
  dd {
    margin: 2px 0 0;
  }
  .link {
    color: var(--accent);
    font-size: 0.9em;
    margin-left: 6px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    max-width: 360px;
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
