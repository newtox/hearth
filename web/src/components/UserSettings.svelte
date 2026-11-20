<script lang="ts">
  import { ask } from '../lib/dialog.svelte';
  import { LANGS, fromEnglish, lang, setLang, t } from '../lib/i18n.svelte';
  import { api, upload } from '../lib/api';
  import { play, sounds } from '../lib/sounds';
  import { BADGE_ICONS } from '../lib/badges';
  import BadgeIcon from './BadgeIcon.svelte';
  import { badgeLabel } from '../lib/activity';
  import { app, attempt, logout, toast, type Badge, type User } from '../lib/store.svelte';
  import { THEMES, applyTheme, theme } from '../lib/theme.svelte';
  import { docs, releases } from '../lib/docs';
  import Avatar from './Avatar.svelte';
  import { backdrop, hoverHost } from '../lib/still.svelte';
  import { resendVerification } from '../lib/verify';
  import SettingsPage, { type Section } from './SettingsPage.svelte';
  import Modal from './Modal.svelte';
  import { notifications } from '../lib/notify';
  import { allowDevices, canPickOutput, listDevices, setInputDevice, setInputVolume, setOutputDevice, setOutputVolume, voice, type AudioDevice } from '../lib/voice.svelte';

  const me = $derived(app.me!);
  let displayName = $state(app.me!.display_name);
  let bio = $state(app.me!.bio ?? '');
  let accent = $state(app.me!.accent_color ?? '#6b5bd6');

  async function save(e: SubmitEvent) {
    e.preventDefault();
    const u = await attempt(api<User>('PATCH', '/users/@me', { display_name: displayName, bio, accent_color: accent }));
    if (u) {
      app.me = u;
      toast(t('user_settings_profile_saved'));
    }
  }

  let badges = $state<Badge[]>([]);
  let badge = $state({ name: '', icon: 'svg:star', description: '', color: '#8b6cff', translations: { de: { name: '', description: '' }, da: { name: '', description: '' } } });

  async function loadBadges() {
    badges = (await attempt(api<Badge[]>('GET', '/badges'))) ?? [];
  }

  let kind = $state(app.me!.activity?.kind ?? 'playing');
  let activityName = $state(app.me!.activity?.name ?? '');
  let soundOn = $state(sounds.enabled());

  async function setActivity(e: SubmitEvent) {
    e.preventDefault();
    if (await attempt(api('POST', '/users/@me/activity', { kind, name: activityName }))) toast(t('user_settings_activity_set'));
  }

  async function clearActivity() {
    await attempt(api('DELETE', '/users/@me/activity'));
    activityName = '';
  }


  const emptyBadge = () => ({ name: '', icon: 'svg:star', description: '', color: '#8b6cff', translations: { de: { name: '', description: '' }, da: { name: '', description: '' } } });
  let badgeEditing = $state<string | null>(null);
  let canTranslate = $state(false);
  api<{ translate?: boolean }>('GET', '/config').then((c) => (canTranslate = !!c.translate)).catch(() => {});

  function editBadge(b: Badge) {
    badgeEditing = b.id;
    badge = {
      name: b.name,
      icon: b.icon,
      description: b.description,
      color: b.color,
      translations: {
        de: { name: b.translations?.de?.name ?? '', description: b.translations?.de?.description ?? '' },
        da: { name: b.translations?.da?.name ?? '', description: b.translations?.da?.description ?? '' },
      },
    };
  }

  function cancelBadge() {
    badgeEditing = null;
    badge = emptyBadge();
  }

  async function addBadge(e: SubmitEvent) {
    e.preventDefault();
    const saved = await attempt(api<Badge>(badgeEditing ? 'PATCH' : 'POST', badgeEditing ? `/badges/${badgeEditing}` : '/badges', badge));
    if (saved) {
      toast(badgeEditing ? t('user_settings_badge_saved') : t('user_settings_badge_created'));
      cancelBadge();
      loadBadges();
    }
  }

  async function removeBadge(b: Badge) {
    if (await ask({ title: t('user_settings_delete_badge'), text: t('user_settings_name_deleted_removed_from_everyone', { name: b.name }), confirm: t('common_delete'), danger: true })) {
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
  let devices = $state<{ inputs: AudioDevice[]; outputs: AudioDevice[]; labeled: boolean } | null>(null);

  async function loadDevices() {
    devices = await listDevices();
  }

  $effect(() => {
    if (section !== 'voice' || !navigator.mediaDevices) return;
    loadDevices();
    navigator.mediaDevices.addEventListener('devicechange', loadDevices);
    return () => navigator.mediaDevices.removeEventListener('devicechange', loadDevices);
  });

  async function revealDevices() {
    if (await allowDevices()) await loadDevices();
    else toast(t('voice_no_access_microphone'));
  }
  const sections = $derived<Section[]>([
    { id: 'account', name: t('user_settings_my_account'), group: t('user_settings_user_settings') },
    { id: 'profile', name: t('common_profile'), group: t('user_settings_user_settings') },
    { id: 'appearance', name: t('user_settings_appearance'), group: t('user_settings_app_settings') },
    { id: 'language', name: t('user_settings_language'), group: t('user_settings_app_settings') },
    { id: 'voice', name: t('user_settings_voice'), group: t('user_settings_app_settings') },
    { id: 'notifications', name: t('user_settings_notifications'), group: t('user_settings_app_settings') },
    { id: 'activity', name: t('user_settings_activity'), group: t('user_settings_app_settings') },
    ...(app.me!.is_admin ? [{ id: 'badges', name: t('user_settings_global_badges'), group: t('user_settings_instance') }] : []),
    { id: 'help', name: t('user_settings_help') },
    { id: 'logout', name: t('user_settings_sign_out'), danger: true, action: async () => (await ask({ title: t('user_settings_sign_out'), text: t('user_settings_do_really_want_log_out'), confirm: t('user_settings_sign_out'), danger: true })) && logout() },
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
    const body: Record<string, string> = { password: form.password, lang };
    if (editing === 'password') {
      if (form.next !== form.repeat) return toast(t('common_new_passwords_do_not_match'));
      body.new_password = form.next;
    } else if (editing) body[editing] = form.value;
    const u = await attempt(api<User>('PATCH', '/users/@me/account', body));
    if (u) {
      app.me = { ...app.me!, ...u };
      badgeEditing = null;
      toast(t('user_settings_account_saved'));
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
      if (!notifyOn) toast(t('user_settings_browser_blocked_notifications_allow_them'));
    }
  }
  let bannerMoving = $state(false);
  let previewMoving = $state(false);
</script>

<SettingsPage title={t('common_settings')} {sections} bind:current={section} onclose={() => (app.modal = null)}>
  {#if section === 'account'}
    <div class="card account" data-hover>
      <div class="banner" use:hoverHost={(on) => (bannerMoving = on)} style:background-color={me.accent_color ?? 'var(--panel-2)'} style:background-image={backdrop(me.banner, bannerMoving)}></div>
      <div class="row who-row">
        <Avatar id={me.id} name={me.display_name} image={me.avatar} size={72} status={me.status ?? 'online'} />
        <div class="grow">
          <strong class="big">{me.display_name}</strong>
          <p class="muted">@{me.username}</p>
        </div>
        <button class="btn small" onclick={() => (section = 'profile')}>{t('common_edit_profile')}</button>
      </div>
      <dl class="facts">
        <div class="fact">
          <div class="grow"><dt>{t('common_username')}</dt><dd>{me.username}</dd></div>
          <button class="btn small" onclick={() => edit('username')}>{t('common_edit')}</button>
        </div>
        <div class="fact">
          <div class="grow">
            <dt>{t('common_email')}</dt>
            <dd>{showEmail ? me.email : masked(me.email ?? '')} <button class="link" onclick={() => (showEmail = !showEmail)}>{showEmail ? t('user_settings_hide') : t('user_settings_show')}</button>{#if me.email_verified === false}<span class="unverified">{t('verify_not_confirmed')}</span> <button class="link" onclick={() => resendVerification(lang)}>{t('verify_resend')}</button>{/if}</dd>
          </div>
          <button class="btn small" onclick={() => edit('email')}>{t('common_edit')}</button>
        </div>
      </dl>
    </div>

    <h3>{t('common_password')}</h3>
    <div><button class="btn primary" onclick={() => edit('password')}>{t('user_settings_change_password')}</button></div>
    {#if editing}
      <Modal title={editing === 'username' ? t('user_settings_change_username') : editing === 'email' ? t('user_settings_change_email') : t('user_settings_change_password')} onclose={() => (badgeEditing = null)}>
        <form class="stack" onsubmit={saveAccount}>
          {#if editing === 'username'}
            <label class="field"><span>{t('common_username')}</span><input class="input" bind:value={form.value} maxlength="32" autocomplete="username" required /></label>
          {:else if editing === 'email'}
            <label class="field"><span>{t('common_email')}</span><input class="input" type="email" bind:value={form.value} maxlength="254" autocomplete="email" required /></label>
          {:else}
            <label class="field"><span>{t('common_new_password')}</span><input class="input" type="password" bind:value={form.next} minlength="8" maxlength="128" autocomplete="new-password" required /></label>
            <label class="field"><span>{t('common_repeat_new_password')}</span><input class="input" type="password" bind:value={form.repeat} maxlength="128" autocomplete="new-password" required /></label>
            <p class="muted small">{t('user_settings_new_password_signs_out_all')}</p>
          {/if}
          <label class="field"><span>{t('user_settings_current_password')}</span><input class="input" type="password" bind:value={form.password} autocomplete="current-password" required /></label>
          <div class="row end">
            <button type="button" class="btn" onclick={() => (badgeEditing = null)}>{t('common_cancel')}</button>
            <button class="btn primary">{t('common_save')}</button>
          </div>
        </form>
      </Modal>
    {/if}
  {:else if section === 'profile'}
    <div class="grid">
      <form class="stack" onsubmit={save}>
        <label class="field">
          <span>{t('user_settings_display_name')}</span>
          <input class="input" bind:value={displayName} maxlength="32" required />
        </label>
        <label class="field">
          <span>{t('user_settings_about_me')} ({bio.length}/500)</span>
          <textarea class="input" rows="5" maxlength="500" bind:value={bio}></textarea>
        </label>
        <label class="field">
          <span>{t('user_settings_accent_colour_banner_colour_without')}</span>
          <input type="color" bind:value={accent} />
        </label>
        <div class="row wrap">
          <label class="btn">{t('user_settings_change_avatar')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={(e) => pick(e, 'avatar')} hidden /></label>
          <label class="btn">{t('user_settings_change_banner')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={(e) => pick(e, 'banner')} hidden /></label>
        </div>
        <div><button class="btn primary">{t('user_settings_save_profile')}</button></div>
      </form>
      <div class="stack">
        <span class="muted small">{t('user_settings_preview')}</span>
        <div class="preview" data-hover>
          <div class="banner" use:hoverHost={(on) => (previewMoving = on)} style:background-color={accent} style:background-image={backdrop(me.banner, previewMoving)}></div>
          <div class="who">
            <Avatar id={me.id} name={displayName || me.username} image={me.avatar} size={64} />
            <strong>{displayName}</strong>
            <span class="muted">@{me.username}</span>
            {#if bio}<p class="bio">{bio}</p>{/if}
          </div>
        </div>
      </div>
    </div>
  {:else if section === 'appearance'}
    <div class="swatches" role="radiogroup" aria-label={t('user_settings_appearance')}>
      {#each THEMES as th (th.id)}
        <button class="swatch" class:on={theme.id === th.id} role="radio" aria-checked={theme.id === th.id} onclick={() => applyTheme(th.id)}>
          <span class="chip" style:background={th.swatch}></span>
          <span class="label">{t(th.name)}</span>
        </button>
      {/each}
    </div>
  {:else if section === 'language'}
    <div class="stack langs" role="radiogroup" aria-label={t('user_settings_language')}>
      {#each LANGS as l (l.id)}
        <label class="option"><input type="radio" name="lang" checked={lang === l.id} onchange={() => setLang(l.id)} /> {l.name}</label>
      {/each}
    </div>
  {:else if section === 'voice'}
    <h3>{t('voice_devices')}</h3>
    {#if devices}
      <label class="field device">
        <span>{t('voice_input_device')}</span>
        <select class="input" value={devices.inputs.some((d) => d.id === voice.inputDevice) ? voice.inputDevice : ''} onchange={(e) => setInputDevice(e.currentTarget.value)}>
          <option value="">{t('voice_device_default')}</option>
          {#each devices.inputs as d (d.id)}<option value={d.id}>{d.name}</option>{/each}
        </select>
      </label>
      {#if canPickOutput}
        <label class="field device">
          <span>{t('voice_output_device')}</span>
          <select class="input" value={devices.outputs.some((d) => d.id === voice.outputDevice) ? voice.outputDevice : ''} onchange={(e) => setOutputDevice(e.currentTarget.value)}>
            <option value="">{t('voice_device_default')}</option>
            {#each devices.outputs as d (d.id)}<option value={d.id}>{d.name}</option>{/each}
          </select>
        </label>
      {:else}
        <p class="muted">{t('voice_output_device_unsupported')}</p>
      {/if}
      {#if !devices.labeled}
        <p class="muted row hint">{t('voice_devices_need_access')} <button class="btn small" onclick={revealDevices}>{t('voice_devices_allow')}</button></p>
      {/if}
    {/if}
    <h3>{t('voice_volume')}</h3>
    <label class="slider">
      <span class="row"><span class="grow">{t('voice_input_volume')}</span><span class="muted">{Math.round(voice.inputVolume * 100)} %</span></span>
      <input type="range" min="0" max="2" step="0.01" value={voice.inputVolume} oninput={(e) => setInputVolume(Number(e.currentTarget.value))} ondblclick={() => setInputVolume(1)} />
    </label>
    <label class="slider">
      <span class="row"><span class="grow">{t('voice_output_volume')}</span><span class="muted">{Math.round(voice.outputVolume * 100)} %</span></span>
      <input type="range" min="0" max="2" step="0.01" value={voice.outputVolume} oninput={(e) => setOutputVolume(Number(e.currentTarget.value))} ondblclick={() => setOutputVolume(1)} />
    </label>
    <p class="muted">{t('user_settings_voice_user_hint')}</p>
  {:else if section === 'notifications'}
    <h3>{t('user_settings_desktop_notifications')}</h3>
    <p class="muted">{t('user_settings_direct_messages_groups_mentions_replies')}</p>
    {#if notifications.supported}
      <label class="row"><input type="checkbox" checked={notifyOn} onchange={toggleNotify} /> {t('user_settings_show_desktop_notifications')}</label>
    {:else}
      <p class="muted">{t('user_settings_browser_does_not_support_notifications')}</p>
    {/if}
    <h3>{t('user_settings_sounds')}</h3>
    <label class="row"><input type="checkbox" bind:checked={soundOn} onchange={() => (sounds.set(soundOn), play('message'))} /> {t('user_settings_sounds_messages_calls_voice_channels')}</label>
  {:else if section === 'activity'}
    <h3>{t('user_settings_set_activity_hand')}</h3>
    <form class="row wrap" onsubmit={setActivity}>
      <select class="input kind" bind:value={kind} aria-label={t('common_type')}>
        <option value="playing">{t('user_settings_playing')}</option>
        <option value="listening">{t('user_settings_listening')}</option>
        <option value="watching">{t('user_settings_watching')}</option>
        <option value="custom">{t('user_settings_custom_text')}</option>
      </select>
      <input class="input grow" bind:value={activityName} maxlength="64" placeholder={t('user_settings_e_g_tetris')} required />
      <button class="btn small">{t('user_settings_set')}</button>
      <button type="button" class="btn small" onclick={clearActivity}>{t('common_remove')}</button>
    </form>
  {:else if section === 'badges'}
    <p class="muted">{t('user_settings_only_visible_instance_admins_badges')}</p>
    <div class="row wrap">
      {#each badges as b (b.id)}
        {@const label = badgeLabel(b)}
        <span class="badge" class:on={badgeEditing === b.id} style:--c={b.color} title={label.description}><BadgeIcon badge={b} size={16} /> {label.name}<button onclick={() => editBadge(b)} aria-label={t('common_edit')}>✎</button><button onclick={() => removeBadge(b)} aria-label={t('common_delete_name', { name: label.name })}>✕</button></span>
      {/each}
    </div>
    <h3>{badgeEditing ? t('user_settings_edit_badge') : t('user_settings_create_badge')}</h3>
    <form class="stack" onsubmit={addBadge}>
      <div class="icons" role="radiogroup" aria-label={t('user_settings_symbol')}>
        {#each Object.keys(BADGE_ICONS) as name}
          <button type="button" class="pick" class:on={badge.icon === 'svg:' + name} role="radio" aria-checked={badge.icon === 'svg:' + name} aria-label={name} onclick={() => (badge.icon = 'svg:' + name)}>
            <BadgeIcon badge={{ icon: 'svg:' + name, color: badge.color }} size={22} />
          </button>
        {/each}
      </div>
      <div class="row wrap">
        <input class="input name" bind:value={badge.name} maxlength="32" placeholder={t('user_settings_name_english')} required />
        <input class="input grow" bind:value={badge.description} maxlength="120" placeholder={t('user_settings_description_english')} />
        <input type="color" bind:value={badge.color} aria-label={t('user_settings_colour')} />
      </div>
      <details class="langs-edit">
        <summary>{t('user_settings_translations')}</summary>
        <p class="muted small">{canTranslate ? t('user_settings_translations_auto') : t('user_settings_translations_manual')}</p>
        {#each [['de', 'Deutsch'], ['da', 'Dansk']] as [code, label] (code)}
          <div class="row wrap">
            <span class="langname">{label}</span>
            <input class="input name" bind:value={badge.translations[code as 'de' | 'da'].name} maxlength="32" placeholder={t('common_name')} />
            <input class="input grow" bind:value={badge.translations[code as 'de' | 'da'].description} maxlength="120" placeholder={t('user_settings_description_optional')} />
          </div>
        {/each}
      </details>
      <div class="row">
        <button class="btn">{badgeEditing ? t('common_save') : t('user_settings_create_badge')}</button>
        {#if badgeEditing}<button type="button" class="btn" onclick={cancelBadge}>{t('common_cancel')}</button>{/if}
      </div>
    </form>
  {:else if section === 'help'}
    <div class="stack">
      <a class="btn" href={docs('guide')} target="_blank" rel="noopener noreferrer">{t('user_settings_guide')}</a>
      <a class="btn" href={releases} target="_blank" rel="noopener noreferrer">{t('user_settings_download_apps')}</a>
      {#if me.is_admin}<a class="btn" href={docs('installation')} target="_blank" rel="noopener noreferrer">{t('user_settings_set_up_server')}</a>{/if}
    </div>
  {/if}
</SettingsPage>

<style>
  .device {
    max-width: 420px;
    margin-bottom: 14px;
  }
  .hint {
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
    margin-bottom: 18px;
  }
  .slider {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 420px;
    margin-bottom: 16px;
  }
  .slider .grow {
    flex: 1;
  }
  .slider input {
    width: 100%;
    accent-color: var(--accent);
  }
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
  .unverified {
    margin: 0 6px;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.8em;
    background: color-mix(in srgb, var(--danger) 18%, transparent);
    color: var(--danger);
  }
  .langs-edit {
    display: grid;
    gap: 8px;
  }
  .langs-edit summary {
    cursor: pointer;
    font-weight: 600;
    font-size: 0.9em;
  }
  .langname {
    width: 64px;
    font-size: 0.85em;
    color: var(--muted);
  }
  .small {
    font-size: 0.85em;
  }
  .badge.on {
    outline: 2px solid var(--accent);
  }
</style>
