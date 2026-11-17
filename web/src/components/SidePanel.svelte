<script lang="ts">
  import { api } from '../lib/api';
  import { locale, t } from '../lib/i18n.svelte';
  import { has } from '../lib/perms';
  import { app, attempt, channel, guild, jumpTo, memberName, P, type Dm, type Message } from '../lib/store.svelte';
  import { groupMay } from '../lib/groups';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import Markdown from './Markdown.svelte';

  const g = $derived(guild());
  const ch = $derived(channel());
  const direct = $derived(ch && (ch.kind === 'dm' || ch.kind === 'group') ? (ch as Dm) : null);
  const canPin = $derived(!!ch && (g ? has((ch as any).permissions, P.MANAGE_MESSAGES) : direct?.kind === 'group' ? groupMay(direct, 'delete') : !!direct));

  let pins = $state<Message[] | null>(null);

  $effect(() => {
    if (app.side !== 'pins' || !ch) return;
    const id = ch.id;
    void app.pinTick;
    attempt(api<Message[]>('GET', `/channels/${id}/pins`)).then((list) => {
      if (channel()?.id === id) pins = list ?? [];
    });
  });

  const when = new Intl.DateTimeFormat(locale(), { dateStyle: 'medium', timeStyle: 'short' });

  function open(m: Message) {
    jumpTo(g?.id ?? null, m.channel_id, m.id);
  }

  function unpin(m: Message) {
    attempt(api('DELETE', `/channels/${m.channel_id}/pins/${m.id}`));
  }
</script>

{#snippet item(m: Message, showChannel: boolean)}
  <article class="hit">
    <div class="row meta">
      <Avatar id={m.author.id} name={m.author.display_name} image={m.author.avatar} size={22} />
      <strong class="ellipsis">{memberName(g, m.author)}</strong>
      {#if showChannel && m.channel_name}<span class="muted ellipsis">#{m.channel_name}</span>{/if}
      <time class="muted" datetime={m.created_at}>{when.format(new Date(m.created_at))}</time>
    </div>
    {#if m.content && !m.content.startsWith('<sticker:')}<div class="text"><Markdown text={m.content} /></div>{/if}
    {#if m.attachments.length}<p class="muted small"><Icon name="clip" size={13} /> {m.attachments.length === 1 ? m.attachments[0].filename : t('{n} Dateien', { n: m.attachments.length })}</p>{/if}
    <div class="row actions">
      <button class="btn small" onclick={() => open(m)}>{t('Springen')}</button>
      {#if !showChannel && canPin}<button class="btn small" onclick={() => unpin(m)}>{t('Nicht mehr anheften')}</button>{/if}
    </div>
  </article>
{/snippet}

<aside class="panel pane-members side">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'chat')} aria-label={t('Zurück zum Chat')}><Icon name="close" /></button>
    <h3 class="grow">{app.side === 'pins' ? t('Angeheftet') : t('Suchergebnisse')}</h3>
    <button class="icon" onclick={() => (app.side = null)} aria-label={t('Schließen')} title={t('Schließen')}><Icon name="close" size={16} /></button>
  </header>
  <div class="list scroll">
    {#if app.side === 'pins'}
      {#if pins === null}
        <p class="muted pad">{t('Lädt …')}</p>
      {:else}
        {#each pins as m (m.id)}{@render item(m, false)}{:else}<p class="muted pad">{t('Hier ist noch nichts angeheftet. Über das Menü einer Nachricht lässt sie sich anheften.')}</p>{/each}
      {/if}
    {:else}
      {#if app.search.loading}
        <p class="muted pad">{t('Sucht …')}</p>
      {:else}
        <p class="muted pad">{app.search.total === 1 ? t('1 Treffer') : t('{n} Treffer', { n: app.search.total })}{#if app.search.total > app.search.results.length}, {t('die neuesten {n} werden gezeigt', { n: app.search.results.length })}{/if}</p>
        {#each app.search.results as m (m.id)}{@render item(m, !!g)}{:else}<p class="muted pad">{t('Nichts gefunden. Tipp: „von:name“ sucht nach Verfasser.')}</p>{/each}
      {/if}
    {/if}
  </div>
</aside>

<style>
  aside {
    width: 320px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    padding: 12px 10px 10px 14px;
    border-bottom: 1px solid var(--line);
  }
  header h3 {
    margin: 0;
  }
  .mobile {
    display: none;
  }
  .list {
    flex: 1;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .pad {
    padding: 4px 6px;
    margin: 0;
  }
  .hit {
    padding: 10px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    display: grid;
    gap: 6px;
  }
  .meta {
    gap: 8px;
    font-size: 0.9em;
  }
  .meta time {
    margin-left: auto;
    flex: none;
    font-size: 0.85em;
  }
  .text {
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    max-height: 9em;
    overflow: hidden;
  }
  .small {
    margin: 0;
    font-size: 0.85em;
  }
  .actions {
    gap: 6px;
  }
  @media (max-width: 1100px) {
    aside {
      width: 280px;
    }
  }
  @media (max-width: 860px) {
    .mobile {
      display: inline-flex;
    }
  }
</style>
