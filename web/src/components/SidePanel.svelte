<script lang="ts">
  import { api } from '../lib/api';
  import { locale, t } from '../lib/i18n.svelte';
  import { has } from '../lib/perms';
  import { app, attempt, channel, guild, jumpTo, memberName, runSearch, P, type Dm, type Message } from '../lib/store.svelte';
  import { groupMay } from '../lib/groups';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import SearchBox from './SearchBox.svelte';
  import Markdown from './Markdown.svelte';

  const g = $derived(guild());
  const ch = $derived(channel());
  const direct = $derived(ch && (ch.kind === 'dm' || ch.kind === 'group') ? (ch as Dm) : null);
  const canPin = $derived(!!ch && (g ? has((ch as any).permissions, P.PIN_MESSAGES) : direct?.kind === 'group' ? groupMay(direct, 'pin') : !!direct));

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
    {#if m.attachments.length}<p class="muted small"><Icon name="clip" size={13} /> {m.attachments.length === 1 ? m.attachments[0].filename : t('side_panel_n_files', { n: m.attachments.length })}</p>{/if}
    <div class="row actions">
      <button class="btn small" onclick={() => open(m)}>{t('side_panel_jump')}</button>
      {#if !showChannel && canPin}<button class="btn small" onclick={() => unpin(m)}>{t('common_unpin')}</button>{/if}
    </div>
  </article>
{/snippet}

<aside class="panel pane-members side">
  <header class="row">
    <h3 class="grow">{app.side === 'pins' ? t('common_pinned') : t('side_panel_search_results')}</h3>
    <button class="icon" onclick={() => ((app.side = null), (app.pane = 'chat'))} aria-label={t('common_close')} title={t('common_close')}><Icon name="close" size={16} /></button>
  </header>
  <div class="list scroll">
    {#if app.side === 'pins'}
      {#if pins === null}
        <p class="muted pad">{t('side_panel_loading')}</p>
      {:else}
        {#each pins as m (m.id)}{@render item(m, false)}{:else}<p class="muted pad">{t('side_panel_nothing_pinned_here_yet_pin')}</p>{/each}
      {/if}
    {:else}
      <SearchBox wide />
      {#if !app.search.query}
        <p class="muted pad">{t('search_tip')}</p>
      {:else if app.search.loading && !app.search.results.length}
        <p class="muted pad">{t('side_panel_searching')}</p>
      {:else}
        <p class="muted pad">{app.search.total === 1 ? t('side_panel_1_result') : t('side_panel_n_results', { n: app.search.total })}</p>
        {#each app.search.results as m (m.id)}{@render item(m, !!g)}{:else}<p class="muted pad">{t('search_nothing_found')}</p>{/each}
        {#if app.search.total > app.search.results.length}
          <button class="btn small more" disabled={app.search.loading} onclick={() => runSearch(app.search.query, true)}>{app.search.loading ? t('side_panel_searching') : t('search_load_more')}</button>
        {/if}
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
  .more {
    align-self: center;
    margin: 4px 0 8px;
  }
</style>
