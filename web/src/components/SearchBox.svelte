<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { editing, HAS_KINDS, hasWord, keyword, yesWord, type FilterKey, type HasKind } from '../lib/search';
  import { app, guild, runSearch, searchPeople } from '../lib/store.svelte';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  let { wide = false }: { wide?: boolean } = $props();

  let query = $state(app.search.query);
  let open = $state(false);
  let more = $state(false);
  let active = $state(-1);
  let box = $state<HTMLDivElement>();
  let input = $state<HTMLInputElement>();

  $effect(() => {
    query = app.search.query;
  });

  const g = $derived(guild());
  const current = $derived(editing(query));

  const HAS_LABEL: Record<HasKind, string> = {
    link: 'search_has_link',
    file: 'search_has_file',
    image: 'search_has_image',
    video: 'search_has_video',
    sound: 'search_has_sound',
    sticker: 'search_has_sticker',
    poll: 'search_has_poll',
  };

  type Option = { id: string; icon?: string; title: string; hint?: string; user?: { id: string; name: string; avatar: string | null }; apply: () => void };

  function start(key: FilterKey) {
    const base = query.trimEnd();
    query = `${base}${base ? ' ' : ''}${keyword(key)}:`;
    active = 0;
    input?.focus();
  }

  function fill(value: string) {
    if (!current) return;
    query = `${query.slice(0, current.start)}${keyword(current.key)}:${value} `;
    active = -1;
    input?.focus();
  }

  const filters = $derived.by(() => {
    const list: { key: FilterKey; icon: string; title: string; hint: string }[] = [
      { key: 'from', icon: 'user', title: t('search_from_user'), hint: `${keyword('from')}: ${t('search_user')}` },
      ...(g ? [{ key: 'in' as FilterKey, icon: 'hash', title: t('search_in_channel'), hint: `${keyword('in')}: ${t('search_channel')}` }] : []),
      { key: 'has', icon: 'clip', title: t('search_includes_type'), hint: `${keyword('has')}: ${t('search_has_hint')}` },
      { key: 'mentions', icon: 'at', title: t('search_mentions_user'), hint: `${keyword('mentions')}: ${t('search_user')}` },
    ];
    if (more)
      list.push(
        { key: 'before', icon: 'clock', title: t('search_before_date'), hint: `${keyword('before')}: ${t('search_date')}` },
        { key: 'during', icon: 'clock', title: t('search_on_date'), hint: `${keyword('during')}: ${t('search_date')}` },
        { key: 'after', icon: 'clock', title: t('search_after_date'), hint: `${keyword('after')}: ${t('search_date')}` },
        { key: 'pinned', icon: 'pin', title: t('search_pinned_only'), hint: `${keyword('pinned')}: ${yesWord()}` },
      );
    return list;
  });

  const options = $derived.by((): Option[] => {
    if (!current) {
      const list: Option[] = filters.map((f) => ({ id: f.key, icon: f.icon, title: f.title, hint: f.hint, apply: () => start(f.key) }));
      if (!more) list.push({ id: 'more', icon: 'sliders', title: t('search_more_filters'), hint: t('search_more_hint'), apply: () => ((more = true), input?.focus()) });
      return list;
    }
    const v = current.value.replace(/^[@#]/, '').toLowerCase();
    if (current.key === 'from' || current.key === 'mentions')
      return searchPeople()
        .filter((u) => !v || u.username.toLowerCase().includes(v) || u.display_name.toLowerCase().includes(v))
        .slice(0, 8)
        .map((u) => ({ id: u.id, title: u.display_name, hint: `@${u.username}`, user: { id: u.id, name: u.display_name, avatar: u.avatar }, apply: () => fill(u.username) }));
    if (current.key === 'in')
      return (g?.channels ?? [])
        .filter((c) => c.kind === 'text' && (!v || c.name.toLowerCase().includes(v)))
        .slice(0, 8)
        .map((c) => ({ id: c.id, icon: 'hash', title: c.name, apply: () => fill(c.name) }));
    if (current.key === 'has')
      return HAS_KINDS.filter((k) => !v || hasWord(k).includes(v)).map((k) => ({ id: k, icon: 'clip', title: hasWord(k), hint: t(HAS_LABEL[k]), apply: () => fill(hasWord(k)) }));
    if (current.key === 'pinned') return [{ id: 'yes', icon: 'pin', title: yesWord(), hint: t('search_pinned_only'), apply: () => fill(yesWord()) }];
    return [];
  });

  const dateKey = $derived(current && (current.key === 'before' || current.key === 'after' || current.key === 'during'));

  function run() {
    if (!query.trim()) return;
    open = false;
    input?.blur();
    runSearch(query.trim());
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (!options.length) return;
      e.preventDefault();
      open = true;
      active = (active + (e.key === 'ArrowDown' ? 1 : -1) + options.length) % options.length;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (open && active >= 0 && options[active]) options[active].apply();
      else run();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (open) open = false;
      else {
        query = '';
        if (app.side === 'search') app.side = null;
      }
    }
  }

  $effect(() => {
    void current?.key;
    active = current ? 0 : -1;
  });
</script>

<div class="wrap" class:wide bind:this={box} onfocusin={() => (open = true)} onfocusout={(e) => !box?.contains(e.relatedTarget as Node) && (open = false)}>
  <label class="searchbox">
    <Icon name="search" size={15} />
    <input type="search" role="combobox" bind:this={input} bind:value={query} onkeydown={key} oninput={() => (open = true)} placeholder={g ? t('common_search_server', { server: w('server') }) : t('common_search')} aria-label={t('common_search_messages')} aria-expanded={open} aria-controls="search-popup" autocomplete="off" />
  </label>
  {#if open}
    <div class="popup panel" id="search-popup" role="listbox" tabindex="-1" onmousedown={(e) => e.target instanceof HTMLInputElement || e.preventDefault()}>
      <p class="head">{current ? t('search_suggestions') : t('search_filters')}</p>
      {#each options as o, i (o.id)}
        <button class="option" class:active={i === active} role="option" aria-selected={i === active} onclick={o.apply} onmousemove={() => (active = i)}>
          {#if o.user}<Avatar id={o.user.id} name={o.user.name} image={o.user.avatar} size={22} />{:else if o.icon}<span class="ico"><Icon name={o.icon} size={18} /></span>{/if}
          <span class="text">
            <strong class="ellipsis">{o.title}</strong>
            {#if o.hint}<small class="ellipsis">{o.hint}</small>{/if}
          </span>
        </button>
      {/each}
      {#if dateKey}
        <label class="date">
          <span>{t('search_pick_date')}</span>
          <input class="input" type="date" onchange={(e) => e.currentTarget.value && fill(e.currentTarget.value)} />
        </label>
      {:else if current && !options.length}
        <p class="empty muted">{t('search_no_matches')}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    flex: 0 1 220px;
    min-width: 110px;
  }
  .wrap.wide {
    flex: none;
    width: 100%;
  }
  .searchbox {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    height: 30px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    color: var(--muted);
  }
  .wide .searchbox {
    height: 36px;
    padding: 0 10px;
  }
  .searchbox input {
    min-width: 0;
    width: 100%;
    border: 0;
    background: none;
    color: var(--ink);
    font: inherit;
    font-size: 0.9em;
    outline: none;
  }
  .searchbox:focus-within {
    outline: 2px solid var(--accent);
  }
  .popup {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 45;
    width: min(320px, 90vw);
    padding: 8px;
    display: grid;
    gap: 2px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }
  .wide .popup {
    left: 0;
    width: auto;
  }
  .head {
    margin: 2px 8px 6px;
    font-size: 0.78em;
    font-weight: 700;
    color: var(--muted);
  }
  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 8px;
    border-radius: var(--radius-s);
    text-align: left;
    min-width: 0;
  }
  .option.active {
    background: var(--panel-2);
  }
  .ico {
    display: inline-flex;
    color: var(--muted);
    width: 22px;
    justify-content: center;
  }
  .text {
    display: grid;
    min-width: 0;
  }
  .text strong {
    font-weight: 600;
    font-size: 0.92em;
  }
  .text small {
    color: var(--muted);
    font-size: 0.8em;
  }
  .date {
    display: grid;
    gap: 6px;
    padding: 6px 8px;
    font-size: 0.85em;
    color: var(--muted);
  }
  .empty {
    padding: 6px 8px;
    font-size: 0.88em;
  }
</style>
