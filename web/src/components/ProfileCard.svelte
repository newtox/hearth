<script lang="ts">
  import { fromEnglish, locale, t } from '../lib/i18n.svelte';
  import { api, asset } from '../lib/api';
  import { app, attempt, canModerate, guild, isBlocked, isFriend, openDm, P, type Badge, type User, presenceOf } from '../lib/store.svelte';
  import { activityText, badgeLabel, elapsed } from '../lib/activity';
  import Avatar from './Avatar.svelte';
  import BadgeIcon from './BadgeIcon.svelte';
  import ProgressModal from './ProgressModal.svelte';
  import Icon from './Icon.svelte';

  let { userId }: { userId: string } = $props();

  const g = $derived(guild());
  const member = $derived(g?.members.find((m) => m.id === userId) ?? null);
  const roles = $derived(g && member ? g.roles.filter((r) => member.roles.includes(r.id)) : []);
  let user = $state<User | null>(null);
  const PROVIDERS: Record<string, string> = { twitch: 'Twitch', youtube: 'YouTube', steam: 'Steam', spotify: 'Spotify', mock: 'Mock' };
  let allBadges = $state<Badge[]>([]);
  let progressTrack = $state<string | null>(null);
  const live = $derived(app.guilds.flatMap((x) => x.members).find((m) => m.id === userId)?.badges);

  async function toggleBadge(b: Badge) {
    const has = user?.badges?.some((x) => x.id === b.id);
    await attempt(api(has ? 'DELETE' : 'PUT', `/users/${userId}/badges/${b.id}`));
    user = (await attempt(api<User>('GET', `/users/${userId}`))) ?? user;
  }

  $effect(() => {
    user = null;
    attempt(api<User>('GET', `/users/${userId}`)).then((u) => (user = u ?? null));
    if (app.me?.is_admin) attempt(api<Badge[]>('GET', '/badges')).then((b) => (allBadges = b ?? []));
  });

  const shown = $derived(member ?? app.dms.flatMap((d) => d.recipients).find((r) => r.id === userId) ?? user ?? { activity: null, streaming: false });
  const since = new Intl.DateTimeFormat(locale(), { day: 'numeric', month: 'long', year: 'numeric' });

  function close() {
    app.profile = null;
  }

  async function kick() {
    if (g && confirm(t('{name} wirklich entfernen?', { name: user?.display_name ?? '' }))) {
      await attempt(api('DELETE', `/guilds/${g.id}/members/${userId}`));
      close();
    }
  }

  async function ban() {
    if (!g) return;
    const reason = prompt(t('Grund für den Bann von {name}:', { name: user?.display_name ?? '' }), '');
    if (reason === null) return;
    await attempt(api('PUT', `/guilds/${g.id}/bans/${userId}`, { reason }));
    close();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && !progressTrack && close()} />

<div class="backdrop" onclick={(e) => e.target === e.currentTarget && close()} role="presentation">
  <div class="panel card" role="dialog" aria-modal="true" aria-label={t('Profil')}>
    <div class="banner" style:background-color={user?.accent_color ?? 'var(--panel-2)'} style:background-image={user?.banner ? `url(${asset(user.banner)})` : null}>
      <button class="x" onclick={close} aria-label={t('Schließen')}><Icon name="close" /></button>
    </div>
    {#if user}
      <div class="body">
        <div class="face"><Avatar id={user.id} name={user.display_name} image={user.avatar} size={84} status={presenceOf(user)} /></div>
        <h2>{member?.nickname || user.display_name}</h2>
        <p class="muted">@{user.username}{member?.nickname ? t(', auch bekannt als {name}', { name: user.display_name }) : ''}</p>
        {#if (live ?? user.badges)?.length}
          <div class="badges">
            {#each live ?? user.badges ?? [] as b (b.id)}
              {@const label = badgeLabel(b)}
              {#if b.track}
                <button class="mark" title="{label.name}: {label.description}" aria-label="{label.name}: {label.description}" onclick={() => (progressTrack = b.track!)}><BadgeIcon badge={b} size={26} /></button>
              {:else}
                <span class="mark" role="img" title="{label.name}{label.description ? ': ' + label.description : ''}" aria-label={label.name}><BadgeIcon badge={b} size={26} /></span>
              {/if}
            {/each}
          </div>
        {/if}
        {#if shown.activity || shown.streaming}
          <div class="activity">
            {#if shown.activity}
              <Icon name={shown.activity.kind === 'playing' ? 'gamepad' : shown.activity.kind === 'listening' ? 'volume' : 'smile'} size={20} />
              <div class="grow">
                <strong class="ellipsis">{activityText(shown.activity)}</strong>
                <small class="muted">{shown.activity.details ? shown.activity.details + ', ' : ''}{elapsed(shown.activity.since)}</small>
              </div>
            {/if}
            {#if shown.streaming}<span class="live"><Icon name="broadcast" size={16} /> {t('Teilt den Bildschirm')}</span>{/if}
          </div>
        {/if}
        {#if user.bio}<p class="bio">{user.bio}</p>{/if}
        {#if roles.length}
          <div class="roles">
            {#each roles as r (r.id)}<span class="role" style:--c={r.color ?? 'var(--muted)'}>{r.name}</span>{/each}
          </div>
        {/if}
        {#if user.connections?.length}
          <div class="links">
            {#each user.connections as c (c.provider)}
              {#if c.url}
                <a class="link" href={c.url} target="_blank" rel="noopener noreferrer nofollow"><strong>{PROVIDERS[c.provider] ?? c.provider}</strong><span class="ellipsis">{c.name}</span></a>
              {:else}
                <span class="link"><strong>{PROVIDERS[c.provider] ?? c.provider}</strong><span class="ellipsis">{c.name}</span></span>
              {/if}
            {/each}
          </div>
        {/if}
        <p class="muted small">{t('Dabei seit {date}', { date: since.format(new Date(user.created_at!)) })}{member ? t(', hier seit {date}', { date: since.format(new Date(member.joined_at)) }) : ''}</p>
        <div class="row actions">
          {#if user.id === app.me?.id}
            <button class="btn" onclick={() => ((app.profile = null), (app.modal = { kind: 'user' }))}><Icon name="edit" size={16} /> {t('Profil bearbeiten')}</button>
            {#if user.progress}<button class="btn" onclick={() => (progressTrack = 'play')}>{t('Fortschritt')}</button>{/if}
          {:else}
            <button class="btn primary" onclick={() => openDm(user!.id)}><Icon name="mail" size={16} /> {t('Nachricht schreiben')}</button>
            {#if isBlocked(user.id)}
              <button class="btn" onclick={() => attempt(api('DELETE', `/blocks/${user!.id}`))}>{t('Blockierung aufheben')}</button>
            {:else if isFriend(user.id)}
              <button class="btn" onclick={() => attempt(api('DELETE', `/friends/${user!.id}`))}>{t('Freund entfernen')}</button>
            {:else if app.friends.incoming.some((f) => f.id === user!.id)}
              <button class="btn" onclick={() => attempt(api('PUT', `/friends/${user!.id}`))}>{t('Anfrage annehmen')}</button>
            {:else if app.friends.outgoing.some((f) => f.id === user!.id)}
              <button class="btn" disabled>{t('Anfrage gesendet')}</button>
            {:else}
              <button class="btn" onclick={() => attempt(api('POST', '/friends', { user_id: user!.id }))}><Icon name="userPlus" size={16} /> {t('Freund hinzufügen')}</button>
            {/if}
            {#if !isBlocked(user.id)}<button class="btn danger" onclick={() => confirm(t('{name} blockieren? Ihr könnt euch dann nicht mehr schreiben, und eine Freundschaft endet.', { name: user!.display_name })) && attempt(api('PUT', `/blocks/${user!.id}`))}>{t('Blockieren')}</button>{/if}
            {#if g && canModerate(g, user.id, P.KICK_MEMBERS)}<button class="btn danger" onclick={kick}>{t('Entfernen')}</button>{/if}
            {#if g && canModerate(g, user.id, P.BAN_MEMBERS)}<button class="btn danger" onclick={ban}>{t('Bannen')}</button>{/if}
          {/if}
        </div>
        {#if app.me?.is_admin && allBadges.length}
          <details>
            <summary class="muted">{t('Abzeichen vergeben')}</summary>
            <div class="badges grant">
              {#each allBadges as b (b.id)}
                <button class="badge" class:dim={!user.badges?.some((x) => x.id === b.id)} style:--c={b.color} onclick={() => toggleBadge(b)} aria-pressed={user.badges?.some((x) => x.id === b.id)}><BadgeIcon badge={b} size={16} />{fromEnglish(b.name)}</button>
              {/each}
            </div>
          </details>
        {/if}
      </div>
    {:else}
      <div class="body"><p class="muted">{t('Lade Profil …')}</p></div>
    {/if}
  </div>
</div>

{#if progressTrack && user?.progress}
  <ProgressModal progress={user.progress} track={progressTrack} own={user.id === app.me?.id} name={user.display_name} onclose={() => (progressTrack = null)} />
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(8, 5, 20, 0.6);
  }
  .card {
    width: min(400px, 100%);
    overflow: hidden;
  }
  .banner {
    height: 120px;
    background-size: cover;
    background-position: center;
    display: flex;
    justify-content: flex-end;
    align-items: flex-start;
    padding: 8px;
  }
  .x {
    padding: 6px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.45);
    color: #fff;
  }
  .body {
    padding: 0 20px 20px;
    display: grid;
    gap: 8px;
  }
  .face {
    margin-top: -42px;
    width: fit-content;
    border-radius: 50%;
    border: 5px solid var(--panel);
    background: var(--panel);
    line-height: 0;
  }
  :global([data-theme='pergament']) .face {
    border-radius: 4px;
  }
  .bio {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
  .roles {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .role {
    padding: 1px 10px;
    border-radius: 99px;
    border: 1px solid var(--c);
    font-size: 0.85em;
    font-weight: 600;
  }
  .role::before {
    content: '';
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--c);
    margin-right: 6px;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .mark {
    display: inline-flex;
    cursor: default;
  }
  button.mark {
    cursor: pointer;
    border-radius: 6px;
  }
  .activity {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
  }
  .activity .grow {
    display: grid;
    line-height: 1.3;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--danger);
    font-weight: 600;
    font-size: 0.88em;
    white-space: nowrap;
  }
  .badges {
    align-items: center;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 10px 2px 7px;
    border-radius: 99px;
    font-size: 0.85em;
    font-weight: 600;
    border: 1px solid var(--c);
    background: color-mix(in srgb, var(--c) 22%, transparent);
  }
  .badge.dim {
    opacity: 0.45;
    background: none;
    border-style: dashed;
  }
  .grant {
    margin-top: 8px;
  }
  summary {
    cursor: pointer;
    font-size: 0.88em;
  }
  .links {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 6px;
  }
  .link {
    display: grid;
    padding: 6px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
    text-decoration: none;
    font-size: 0.88em;
    line-height: 1.3;
    min-width: 0;
  }
  a.link:hover {
    background: var(--panel-2);
  }
  .link strong {
    font-size: 0.85em;
    color: var(--muted);
  }
  .small {
    font-size: 0.85em;
  }
  .actions {
    flex-wrap: wrap;
    margin-top: 6px;
  }
</style>
