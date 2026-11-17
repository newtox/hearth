<script lang="ts">
  import { memberMenu, openMenu } from '../lib/menus';
  import { copy } from '../lib/moderation';
  import { locale, t } from '../lib/i18n.svelte';
  import { tick } from 'svelte';
  import { api, asset, upload } from '../lib/api';
  import { CUSTOM, STICKER } from '../lib/markdown';
  import { has } from '../lib/perms';
  import { app, attempt, channel, dmTitle, select, findEmoji, guild, loadMessages, memberColor, memberName, partner, toast, P, type Attachment, type Dm, type Emoji, type Message, presenceOf, jumpTo, mentionsMe, runSearch } from '../lib/store.svelte';
  import { GROUP_ROLES, groupMay, groupRole } from '../lib/groups';
  import { isBlocked } from '../lib/store.svelte';
  import CallBar from './CallBar.svelte';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import EmojiPicker from './EmojiPicker.svelte';
  import Icon from './Icon.svelte';
  import Markdown from './Markdown.svelte';
  import PollCard from './PollCard.svelte';
  import PollModal from './PollModal.svelte';

  const g = $derived(guild());
  const ch = $derived(channel());
  const direct = $derived(ch && (ch.kind === 'dm' || ch.kind === 'group') ? (ch as Dm) : null);
  const dmUser = $derived(direct?.kind === 'dm' ? partner(direct) : null);
  const title = $derived(!ch ? '' : direct ? dmTitle(direct) : ch.name);
  const perms = $derived(!ch ? 0 : direct ? P.VIEW_CHANNEL | P.SEND_MESSAGES | P.ATTACH_FILES | P.ADD_REACTIONS : (ch as any).permissions);
  const messages = $derived(ch ? (app.messages[ch.id] ?? []) : []);
  const typers = $derived.by(() => {
    if (!ch) return [];
    return Object.keys(app.typing[ch.id] ?? {}).map((id) => {
      const u = g?.members.find((m) => m.id === id) ?? direct?.recipients.find((r) => r.id === id) ?? null;
      return u ? memberName(g, u) : t('Jemand');
    });
  });

  const external = $derived(!!direct || has(perms, P.USE_EXTERNAL_EMOJIS));
  const groupDelete = $derived(direct?.kind === 'group' && groupMay(direct, 'delete'));
  let revealed = $state<string[]>([]);
  let pollOpen = $state(false);

  function nameColor(userId: string) {
    if (direct?.kind === 'group') return GROUP_ROLES[groupRole(direct, userId)].color;
    return memberColor(g, userId);
  }

  const parent = $derived(ch && 'parent_id' in ch && ch.kind === 'thread' ? (g?.channels.find((c) => c.id === ch.parent_id) ?? null) : null);
  const canThread = $derived(!!g && !!ch && ch.kind === 'text' && has(perms, P.SEND_MESSAGES));

  function threadOf(messageId: string) {
    return g?.channels.find((c) => c.kind === 'thread' && c.starter_id === messageId) ?? null;
  }

  async function startThread(m: Message) {
    const name = prompt(t('Name des Threads:'), m.content.slice(0, 40));
    if (!name || !ch) return;
    const made = await attempt(api<{ id: string }>('POST', `/channels/${ch.id}/threads`, { name, message_id: m.id }));
    if (made) setTimeout(() => select(g!.id, made.id), 300);
  }

  let draft = $state('');
  let pending = $state<Attachment[]>([]);
  let uploading = $state(0);
  let picker = $state<null | { message: Message | null }>(null);
  let editing = $state<string | null>(null);
  let editText = $state('');
  let list = $state<HTMLDivElement>();
  let input = $state<HTMLTextAreaElement>();

  let replying = $state<Message | null>(null);
  let query = $state('');
  const canPin = $derived(!!ch && (g ? has(perms, P.PIN_MESSAGES) : direct?.kind === 'group' ? groupMay(direct, 'pin') : !!direct));

  function startReply(m: Message) {
    replying = m;
    input?.focus();
  }

  function togglePin(m: Message) {
    attempt(api(m.pinned ? 'DELETE' : 'PUT', `/channels/${m.channel_id}/pins/${m.id}`));
  }

  function searchKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && query.trim()) runSearch(query);
    if (e.key === 'Escape') {
      query = '';
      if (app.side === 'search') app.side = null;
    }
  }

  $effect(() => {
    const id = app.highlight;
    if (!id || !list) return;
    stick = false;
    tick().then(() => {
      list?.querySelector(`[data-id="${id}"]`)?.scrollIntoView({ block: 'center' });
      setTimeout(() => {
        if (app.highlight === id) app.highlight = null;
      }, 2200);
    });
  });

  $effect(() => {
    const text = app.insert;
    if (!text) return;
    app.insert = '';
    draft = draft && !/\s$/.test(draft) ? `${draft} ${text}` : draft + text;
    input?.focus();
  });
  let lastTyping = 0;
  let stick = true;
  let shown = '';

  const time = new Intl.DateTimeFormat(locale(), { hour: '2-digit', minute: '2-digit' });
  const day = new Intl.DateTimeFormat(locale(), { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });

  function fileSize(n: number) {
    return n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(0)} KB` : `${(n / 1048576).toFixed(1)} MB`;
  }

  function encode(text: string) {
    return text.replace(/(^|[^<\w]):([a-z0-9_]{2,32}):/gi, (all, pre, name) => {
      const e = findEmoji(name, 'emoji', external);
      return e ? `${pre}<:${e.name}:${e.file}>` : all;
    });
  }

  async function attach(files: FileList | File[] | null | undefined) {
    if (!ch || !files) return;
    for (const f of [...files].slice(0, 10 - pending.length)) {
      if (f.size > 25 * 1024 * 1024) {
        toast(t('{name} ist größer als 25 MB', { name: f.name }));
        continue;
      }
      uploading++;
      const a = await attempt(upload<Attachment>(`/channels/${ch.id}/attachments`, f));
      uploading--;
      if (a) pending.push(a);
    }
  }

  function paste(e: ClipboardEvent) {
    const files = [...(e.clipboardData?.files ?? [])];
    if (files.length && has(perms, P.ATTACH_FILES)) {
      e.preventDefault();
      attach(files);
    }
  }

  function react(m: Message, emoji: string) {
    const mine = m.reactions.find((r) => r.emoji === emoji)?.users.includes(app.me!.id) ?? false;
    attempt(api('POST', `/channels/${m.channel_id}/messages/${m.id}/reactions`, { emoji, on: !mine }));
  }

  function pick(value: string, kind: 'unicode' | 'custom' | 'sticker', emoji?: Emoji) {
    const target = picker?.message;
    picker = null;
    if (target) react(target, value);
    else if (kind === 'sticker' && ch) attempt(api('POST', `/channels/${ch.id}/messages`, { content: value }));
    else {
      draft += kind === 'custom' ? `:${emoji!.name}: ` : value;
      input?.focus();
    }
  }

  function grouped(m: Message, prev: Message | undefined) {
    return !!prev && !m.reply_to && prev.author.id === m.author.id && Date.parse(m.created_at) - Date.parse(prev.created_at) < 5 * 60000 && !newDay(m, prev);
  }

  function newDay(m: Message, prev: Message | undefined) {
    return !prev || new Date(m.created_at).toDateString() !== new Date(prev.created_at).toDateString();
  }

  $effect(() => {
    const id = ch?.id ?? '';
    const count = messages.length;
    if (!list) return;
    if (id !== shown) {
      shown = id;
      stick = true;
      editing = null;
      pending = [];
      picker = null;
      replying = null;
    }
    if (stick && count >= 0) tick().then(() => list && (list.scrollTop = list.scrollHeight));
  });

  function onscroll() {
    if (list) stick = list.scrollHeight - list.scrollTop - list.clientHeight < 80;
  }

  async function older() {
    if (!ch || !list) return;
    const before = list.scrollHeight;
    stick = false;
    await loadMessages(ch.id, true);
    await tick();
    list.scrollTop += list.scrollHeight - before;
  }

  async function send() {
    const raw = draft.trim();
    if ((!raw && !pending.length) || !ch || uploading) return;
    const files = pending;
    const reply = replying;
    draft = '';
    pending = [];
    replying = null;
    stick = true;
    const ok = await attempt(api('POST', `/channels/${ch.id}/messages`, { content: encode(raw), attachment_ids: files.map((f) => f.id), reply_to: reply?.id ?? null }));
    if (!ok) {
      draft = raw;
      pending = files;
      replying = reply;
    }
    await tick();
    resize();
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
    }
    if (e.key === 'Escape' && replying) replying = null;
  }

  function resize() {
    if (!input) return;
    input.style.height = 'auto';
    input.style.height = Math.min(input.scrollHeight, 180) + 'px';
  }

  function oninput() {
    resize();
    if (ch && draft.trim() && Date.now() - lastTyping > 4000) {
      lastTyping = Date.now();
      api('POST', `/channels/${ch.id}/typing`).catch(() => {});
    }
  }

  function startEdit(m: Message) {
    editing = m.id;
    editText = m.content;
  }

  async function saveEdit(m: Message) {
    const content = editText.trim();
    if (content && content !== m.content) await attempt(api('PATCH', `/channels/${m.channel_id}/messages/${m.id}`, { content }));
    editing = null;
  }

  function editKey(e: KeyboardEvent, m: Message) {
    if (e.key === 'Escape') editing = null;
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      saveEdit(m);
    }
  }

  function messageMenu(e: MouseEvent, m: Message) {
    if ((e.target as HTMLElement).closest('a, input, textarea')) return;
    const own = m.author.id === app.me?.id;
    openMenu(e, [
      has(perms, P.SEND_MESSAGES) && { label: t('Antworten'), icon: 'reply', action: () => startReply(m) },
      has(perms, P.ADD_REACTIONS) && { label: t('Reagieren'), icon: 'smile', action: () => (picker = { message: m }) },
      canThread && !threadOf(m.id) && { label: t('Thread starten'), icon: 'thread', action: () => startThread(m) },
      own && !!m.content && !m.poll && !m.content.match(STICKER) && { label: t('Bearbeiten'), icon: 'edit', action: () => startEdit(m) },
      'sep',
      !!m.content && !m.content.match(STICKER) && { label: t('Text kopieren'), icon: 'copy', action: () => copy(m.content) },
      !own && { label: t('Erwähnen'), icon: 'at', action: () => (app.insert = `@${m.author.username} `) },
      canPin && { label: m.pinned ? t('Nicht mehr anheften') : t('Anheften'), icon: 'pin', action: () => togglePin(m) },
      { label: t('Nachrichten-ID kopieren'), icon: 'id', action: () => copy(m.id) },
      'sep',
      (own || (g && has(perms, P.MANAGE_MESSAGES)) || groupDelete) && { label: t('Nachricht löschen'), icon: 'trash', danger: true, action: () => remove(m) },
    ]);
  }

  function remove(m: Message) {
    if (confirm(t('Nachricht löschen?'))) attempt(api('DELETE', `/channels/${m.channel_id}/messages/${m.id}`));
  }
</script>

{#snippet headerTools()}
  <button class="icon headtool" class:on={app.side === 'pins'} onclick={() => ((app.side = app.side === 'pins' ? null : 'pins'), app.side && (app.pane = 'members'))} aria-label={t('Angeheftete Nachrichten')} title={t('Angeheftete Nachrichten')} aria-pressed={app.side === 'pins'}><Icon name="pin" /></button>
  <label class="searchbox">
    <Icon name="search" size={15} />
    <input type="search" bind:value={query} onkeydown={searchKey} placeholder={g ? t('{server} durchsuchen', { server: w('server') }) : t('Durchsuchen')} aria-label={t('Nachrichten durchsuchen')} />
  </label>
{/snippet}

<section class="panel pane-chat">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'side')} aria-label={t('Kanalliste')}><Icon name="menu" /></button>
    {#if ch}
      {#if dmUser}
        <button class="row who" onclick={() => (app.profile = dmUser.id)} oncontextmenu={(e) => openMenu(e, memberMenu(dmUser.id, null))}>
          <Avatar id={dmUser.id} name={dmUser.display_name} image={dmUser.avatar} size={28} status={presenceOf(dmUser)} />
          <h3 class="ellipsis">{title}</h3>
        </button>
        <span class="grow"></span>
        {@render headerTools()}
        <CallBar dm={direct!} compact />
      {:else if direct}
        <button class="row who" onclick={() => (app.modal = { kind: 'groupchat', dm: direct })}>
          <Icon name="users" />
          <h3 class="ellipsis">{title}</h3>
          <span class="muted">{direct.recipients.length}</span>
        </button>
        <span class="grow"></span>
        {@render headerTools()}
        <CallBar dm={direct} compact />
      {:else}
        {#if parent}
          <button class="row crumb muted" onclick={() => select(g!.id, parent.id)}><Icon name="hash" size={16} />{parent.name}</button>
          <Icon name="thread" />
        {:else}
          <Icon name="hash" />
        {/if}
        <h3 class="ellipsis">{title}</h3>
        <p class="topic muted grow ellipsis">{(ch as any).topic ?? ''}</p>
        {@render headerTools()}
        <button class="icon mobile" onclick={() => (app.pane = 'members')} aria-label={w('members')}><Icon name="users" /></button>
      {/if}
    {:else}
      <h3 class="grow">Hearth</h3>
    {/if}
  </header>

  {#if ch}
    {#if direct}<CallBar dm={direct} />{/if}
    <div class="messages scroll" bind:this={list} {onscroll}>
      {#if app.more[ch.id]}
        <button class="btn small older" onclick={older}>{t('Ältere Nachrichten laden')}</button>
      {:else if app.messages[ch.id]}
        <p class="start muted">{dmUser ? t('Hier beginnt deine Unterhaltung mit {name}.', { name: title }) : direct ? t('Hier beginnt die Gruppe {name}.', { name: title }) : t('Hier beginnt #{name}.', { name: title })}</p>
      {/if}
      {#each messages as m, i (m.id)}
        {@const own = m.author.id === app.me?.id}
        {@const joined = grouped(m, messages[i - 1])}
        {#if newDay(m, messages[i - 1])}
          <div class="day"><span>{day.format(new Date(m.created_at))}</span></div>
        {/if}
        <article class="msg" class:own class:joined class:mention={!own && mentionsMe(m, !!g)} class:flash={app.highlight === m.id} data-id={m.id} oncontextmenu={(e) => editing !== m.id && messageMenu(e, m)}>
          {#if !own}
            <button class="face" onclick={() => (app.profile = m.author.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.author.id, g))} aria-label={t('Profil von {name}', { name: m.author.display_name })} tabindex={joined ? -1 : 0}>
              {#if !joined}<Avatar id={m.author.id} name={m.author.display_name} image={m.author.avatar} size={34} />{/if}
            </button>
          {/if}
          <div class="content">
            {#if !joined}
              <div class="meta">
                {#if !own}<button class="name" style:color={nameColor(m.author.id)} onclick={() => (app.profile = m.author.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.author.id, g))}>{memberName(g, m.author)}</button>{/if}
                <time datetime={m.created_at}>{time.format(new Date(m.created_at))}</time>
                {#if m.pinned}<span class="pinmark" title={t('Angeheftet')}><Icon name="pin" size={12} /></span>{/if}
              </div>
            {/if}
            {#if m.reply}
              {#if m.reply.deleted}
                <p class="quote muted"><Icon name="reply" size={13} /><em>{t('Die ursprüngliche Nachricht wurde gelöscht')}</em></p>
              {:else}
                <button class="quote" onclick={() => jumpTo(g?.id ?? null, m.channel_id, m.reply!.id)}>
                  <Icon name="reply" size={13} />
                  <Avatar id={m.reply.author!.id} name={m.reply.author!.display_name} image={m.reply.author!.avatar} size={16} />
                  <strong style:color={nameColor(m.reply.author!.id)}>{memberName(g, m.reply.author!)}</strong>
                  <span class="ellipsis">{m.reply.content?.match(STICKER) ? t('Sticker') : m.reply.content || (m.reply.attachments ? t('Datei') : '')}</span>
                </button>
              {/if}
            {/if}
            {#if editing === m.id}
              <textarea class="input" bind:value={editText} onkeydown={(e) => editKey(e, m)} rows="2"></textarea>
              <div class="row hint">
                <button class="btn small primary" onclick={() => saveEdit(m)}>{t('Speichern')}</button>
                <button class="btn small" onclick={() => (editing = null)}>{t('Abbrechen')}</button>
              </div>
            {:else if !own && isBlocked(m.author.id) && !revealed.includes(m.id)}
              <button class="hidden muted" onclick={() => (revealed = [...revealed, m.id])}>{t('Nachricht von blockiertem Nutzer. Anzeigen')}</button>
            {:else}
              {@const sticker = m.content.match(STICKER)}
              {#if sticker}
                <img class="sticker" src={asset(sticker[2])} alt={sticker[1]} title={sticker[1]} />
              {:else if m.content}
                <div class="bubble"><Markdown text={m.content} />{#if m.edited_at}<small class="edited"> {t('(bearbeitet)')}</small>{/if}</div>
              {/if}
            {/if}
            {#if m.poll}<PollCard message={m} />{/if}
            {#if m.attachments.length}
              <div class="files">
                {#each m.attachments as a (a.id)}
                  {#if a.is_image}
                    <a href={asset(a.file)} target="_blank" rel="noopener noreferrer"><img class="shot" src={asset(a.file)} alt={a.filename} loading="lazy" /></a>
                  {:else}
                    <a class="file" href={asset(a.file)} download={a.filename}><Icon name="clip" size={16} /><span class="ellipsis">{a.filename}</span><small class="muted">{fileSize(a.size)}</small></a>
                  {/if}
                {/each}
              </div>
            {/if}
            {#if threadOf(m.id)}
              {@const th = threadOf(m.id)!}
              <button class="threadlink" onclick={() => select(g!.id, th.id)}><Icon name="thread" size={14} />{th.name}</button>
            {/if}
            {#if m.reactions.length}
              <div class="reactions">
                {#each m.reactions as r (r.emoji)}
                  {@const custom = r.emoji.match(CUSTOM)}
                  <button class="reaction" class:mine={r.users.includes(app.me!.id)} onclick={() => react(m, r.emoji)} aria-pressed={r.users.includes(app.me!.id)}>
                    {#if custom}<img src={asset(custom[2])} alt=":{custom[1]}:" />{:else}<span class="glyph">{r.emoji}</span>{/if}
                    {r.users.length}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          {#if editing !== m.id}
            <div class="tools">
              {#if has(perms, P.SEND_MESSAGES)}<button class="icon" onclick={() => startReply(m)} aria-label={t('Antworten')} title={t('Antworten')}><Icon name="reply" size={15} /></button>{/if}
              {#if has(perms, P.ADD_REACTIONS)}<button class="icon" onclick={() => (picker = { message: m })} aria-label={t('Reagieren')}><Icon name="smile" size={15} /></button>{/if}
              {#if canThread && !threadOf(m.id)}<button class="icon" onclick={() => startThread(m)} aria-label={t('Thread starten')} title={t('Thread starten')}><Icon name="thread" size={15} /></button>{/if}
              {#if own && m.content && !m.poll && !m.content.match(STICKER)}<button class="icon" onclick={() => startEdit(m)} aria-label={t('Bearbeiten')}><Icon name="edit" size={15} /></button>{/if}
              {#if own || (g && has(perms, P.MANAGE_MESSAGES)) || groupDelete}<button class="icon" onclick={() => remove(m)} aria-label={t('Löschen')}><Icon name="trash" size={15} /></button>{/if}
            </div>
          {/if}
        </article>
      {/each}
    </div>
    <p class="typing muted" aria-live="polite">
      {#if typers.length === 1}{t('{name} schreibt …', { name: typers[0] })}{:else if typers.length > 1}{t('{names} schreiben …', { names: typers.slice(0, 3).join(', ') })}{/if}
    </p>
    {#if has(perms, P.SEND_MESSAGES)}
      {#if pending.length || uploading}
        <div class="pending">
          {#each pending as a (a.id)}
            <span class="chip">
              {#if a.is_image}<img src={asset(a.file)} alt="" />{:else}<Icon name="clip" size={14} />{/if}
              <span class="ellipsis">{a.filename}</span>
              <button onclick={() => (pending = pending.filter((x) => x.id !== a.id))} aria-label={t('{name} entfernen', { name: a.filename })}><Icon name="close" size={13} /></button>
            </span>
          {/each}
          {#if uploading}<span class="muted">{t('Lade hoch …')}</span>{/if}
        </div>
      {/if}
      {#if replying}
        <div class="replying row">
          <Icon name="reply" size={14} />
          <span class="grow ellipsis">{t('Antwort an {name}', { name: memberName(g, replying.author) })}</span>
          <button class="icon" onclick={() => (replying = null)} aria-label={t('Antwort abbrechen')} title={t('Antwort abbrechen')}><Icon name="close" size={14} /></button>
        </div>
      {/if}
      <form class="composer row" onsubmit={(e) => (e.preventDefault(), send())}>
        {#if has(perms, P.ATTACH_FILES)}
          <label class="icon tool" title={t('Datei anhängen')}>
            <Icon name="clip" />
            <input type="file" multiple hidden onchange={(e) => (attach(e.currentTarget.files), (e.currentTarget.value = ''))} />
          </label>
        {/if}
        <textarea class="input" rows="1" maxlength="4000" bind:this={input} bind:value={draft} onkeydown={keydown} {oninput} onpaste={paste} placeholder={t('{message} an {target}', { message: w('message'), target: direct ? title : '#' + title })}></textarea>
        <button type="button" class="icon tool" onclick={() => (pollOpen = true)} aria-label={t('Umfrage erstellen')} title={t('Umfrage erstellen')}><Icon name="poll" /></button>
        <button type="button" class="icon tool" onclick={() => (picker = { message: null })} aria-label={t('Emojis und Sticker')}><Icon name="smile" /></button>
        <button class="btn primary" disabled={(!draft.trim() && !pending.length) || uploading > 0} aria-label={w('send')}><Icon name="send" /><span class="send-label">{w('send')}</span></button>
      </form>
    {:else}
      <p class="readonly muted">{t('Du darfst in diesem Kanal nicht schreiben.')}</p>
    {/if}
    {#if pollOpen}<PollModal channelId={ch.id} onclose={() => (pollOpen = false)} />{/if}
    {#if picker}<EmojiPicker stickers={!picker.message} {external} onpick={pick} onclose={() => (picker = null)} />{/if}
  {:else}
    <div class="blank">
      <h2>{app.guildId ? t('Kein Kanal sichtbar') : t('Willkommen am Feuer')}</h2>
      <p class="muted">
        {#if app.guildId}{t('Hier gibt es keinen Textkanal, den du sehen darfst.')}
        {:else if app.guilds.length}{t('Wähle einen {server} oder öffne eine Unterhaltung.', { server: w('server') })}
        {:else}{t('Erstelle deinen ersten {server} oder tritt einem mit einem Einladungscode bei.', { server: w('server') })}{/if}
      </p>
      {#if !app.guilds.length}<button class="btn primary" onclick={() => (app.modal = { kind: 'create' })}>{t('{server} erstellen oder beitreten', { server: w('server') })}</button>{/if}
    </div>
  {/if}
</section>

<style>
  .headtool.on {
    color: var(--accent);
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
    flex: 0 1 200px;
    min-width: 110px;
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
  .quote {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    margin: 0 0 4px;
    font-size: 0.85em;
    color: var(--muted);
    text-align: left;
  }
  .quote strong {
    flex: none;
  }
  button.quote:hover span {
    color: var(--ink);
  }
  .pinmark {
    color: var(--muted);
    display: inline-flex;
  }
  .msg.flash {
    animation: flash 2s ease-out;
  }
  @keyframes flash {
    0%,
    40% {
      background: color-mix(in srgb, var(--accent) 22%, transparent);
    }
    100% {
      background: transparent;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .msg.flash {
      animation: none;
      outline: 2px solid var(--accent);
    }
  }
  .replying {
    margin: 0 12px;
    padding: 6px 10px;
    gap: 8px;
    border-radius: var(--radius-s) var(--radius-s) 0 0;
    background: var(--panel-2);
    font-size: 0.88em;
    color: var(--muted);
  }
  @media (max-width: 700px) {
    .searchbox {
      display: none;
    }
  }
  section {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .hidden {
    font-size: 0.88em;
    font-style: italic;
    text-decoration: underline dotted;
  }
  .crumb {
    gap: 4px;
    font-weight: 600;
  }
  .crumb:hover {
    color: var(--ink);
  }
  .threadlink {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    padding: 2px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
    font-size: 0.88em;
    font-weight: 600;
    color: var(--accent);
  }
  .sticker {
    display: block;
    width: 150px;
    height: 150px;
    object-fit: contain;
  }
  .files {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 5px;
  }
  .own .files {
    align-items: flex-end;
  }
  .shot {
    display: block;
    max-width: min(360px, 100%);
    max-height: 300px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
  }
  .file {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
    background: var(--panel-2);
    text-decoration: none;
    text-align: left;
  }
  .reactions {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }
  .own .reactions {
    justify-content: flex-end;
  }
  .reaction {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 1px 8px;
    border-radius: 99px;
    border: 1px solid var(--line);
    background: var(--panel-2);
    font-size: 0.88em;
    font-weight: 600;
  }
  .reaction.mine {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .reaction img {
    width: 1.25em;
    height: 1.25em;
    object-fit: contain;
  }
  .glyph {
    font-family: system-ui, 'Noto Color Emoji', sans-serif;
  }
  .pending {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 0 14px 4px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 220px;
    padding: 3px 6px 3px 8px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    border: 1px solid var(--line);
    font-size: 0.88em;
  }
  .chip img {
    width: 22px;
    height: 22px;
    object-fit: cover;
    border-radius: 4px;
  }
  .tool {
    cursor: pointer;
    margin-bottom: 3px;
  }
  header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--line);
    min-height: 54px;
  }
  .topic {
    font-size: 0.92em;
    padding-left: 8px;
  }
  .who {
    min-width: 0;
  }
  .mobile {
    display: none;
  }
  .icon {
    padding: 6px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .icon:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .messages {
    flex: 1;
    min-height: 0;
    padding: 16px 16px 4px;
    display: flex;
    flex-direction: column;
  }
  .older {
    align-self: center;
    margin-bottom: 12px;
  }
  .start {
    text-align: center;
    padding: 12px 0 18px;
    font-family: var(--font-display);
    font-size: 1.1em;
    margin-top: auto;
  }
  .day {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 14px 0 6px;
    color: var(--muted);
    font-size: 0.85em;
  }
  .day::before,
  .day::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--line);
  }
  .msg {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-top: 12px;
    max-width: min(78%, 720px);
  }
  .msg.joined {
    margin-top: 3px;
  }
  .msg.own {
    align-self: flex-end;
    flex-direction: row-reverse;
  }
  .face {
    width: 34px;
    flex: none;
  }
  .content {
    min-width: 0;
  }
  .meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 3px;
    font-size: 0.88em;
  }
  .own .meta {
    justify-content: flex-end;
  }
  .name {
    font-weight: 600;
  }
  :global([data-style='modern']) section {
    background: var(--chat);
  }
  .name:hover {
    text-decoration: underline;
  }
  time {
    color: var(--muted);
    font-size: 0.9em;
  }
  .bubble {
    display: inline-block;
    padding: 8px 13px;
    border-radius: 4px var(--radius) var(--radius) var(--radius);
    background: var(--bubble);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .mention .bubble {
    background: color-mix(in srgb, var(--accent) 16%, var(--bubble));
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .own .bubble {
    background: var(--bubble-own);
    color: var(--bubble-own-ink);
    border-radius: var(--radius) 4px var(--radius) var(--radius);
  }
  .own .content {
    text-align: right;
  }
  .own .bubble {
    text-align: left;
  }
  :global([data-theme='pergament']) .bubble {
    border: 1px solid var(--line);
    border-left: 4px solid var(--accent);
    border-radius: 2px;
  }
  :global([data-theme='pergament']) .own .bubble {
    border-left: 1px solid var(--line);
    border-right: 4px solid #9a7636;
  }
  .edited {
    opacity: 0.65;
  }
  .tools {
    display: flex;
    opacity: 0;
    align-self: center;
  }
  .msg:hover .tools,
  .tools:focus-within {
    opacity: 1;
  }
  .hint {
    margin-top: 6px;
  }
  .typing {
    min-height: 1.6em;
    padding: 0 18px;
    font-size: 0.85em;
  }
  .composer {
    padding: 4px 12px 12px;
    align-items: flex-end;
  }
  .composer textarea {
    resize: none;
    max-height: 180px;
  }
  .readonly {
    padding: 8px 18px 16px;
  }
  .blank {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 12px;
    text-align: center;
    padding: 24px;
  }
  @media (max-width: 860px) {
    .mobile {
      display: block;
    }
    .msg {
      max-width: 92%;
    }
    .send-label {
      display: none;
    }
    .topic {
      display: none;
    }
    .topic + .mobile {
      margin-left: auto;
    }
  }
  @media (hover: none) {
    .tools {
      opacity: 0.6;
    }
  }
</style>
