<script lang="ts">
  import { ask } from '../lib/dialog.svelte';
  import { memberMenu, openMenu } from '../lib/menus';
  import { copy } from '../lib/moderation';
  import { locale, t } from '../lib/i18n.svelte';
  import { tick } from 'svelte';
  import { api, asset, upload } from '../lib/api';
  import { CUSTOM, STICKER } from '../lib/markdown';
  import { has } from '../lib/perms';
  import { app, attempt, channel, dmTitle, select, findEmoji, guild, loadMessages, memberColor, memberName, partner, toast, P, type Attachment, type Dm, type Emoji, type Message, presenceOf, jumpTo, mentionsMe, runSearch, encodeMentions, decodeMentions, plainText, searchPeople } from '../lib/store.svelte';
  import { GROUP_ROLES, groupMay, groupRole } from '../lib/groups';
  import { isBlocked } from '../lib/store.svelte';
  import CallBar from './CallBar.svelte';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import EmojiPicker from './EmojiPicker.svelte';
  import Icon from './Icon.svelte';
  import SearchBox from './SearchBox.svelte';
  import { joinVoice, voice } from '../lib/voice.svelte';
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
      return u ? memberName(g, u) : t('chat_someone');
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


  let draft = $state('');
  let pending = $state<Attachment[]>([]);
  let uploading = $state(0);
  let picker = $state<null | { message: Message | null }>(null);
  let editing = $state<string | null>(null);
  let editText = $state('');
  let list = $state<HTMLDivElement>();
  let input = $state<HTMLTextAreaElement>();

  let replying = $state<Message | null>(null);
  const canPin = $derived(!!ch && (g ? has(perms, P.PIN_MESSAGES) : direct?.kind === 'group' ? groupMay(direct, 'pin') : !!direct));

  function startReply(m: Message) {
    replying = m;
    input?.focus();
  }

  function togglePin(m: Message) {
    attempt(api(m.pinned ? 'DELETE' : 'PUT', `/channels/${m.channel_id}/pins/${m.id}`));
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
  const shortDate = new Intl.DateTimeFormat(locale(), { day: '2-digit', month: '2-digit', year: 'numeric' });
  const full = new Intl.DateTimeFormat(locale(), { dateStyle: 'full', timeStyle: 'short' });

  function stamp(iso: string) {
    const d = new Date(iso);
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const at = time.format(d);
    if (d >= start) return t('chat_today_at', { time: at });
    if (d >= new Date(start.getTime() - 86400000)) return t('chat_yesterday_at', { time: at });
    return `${shortDate.format(d)} ${at}`;
  }

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
        toast(t('chat_name_larger_than_25_mb', { name: f.name }));
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
    return !!prev && !m.reply_to && m.kind !== 'call' && prev.kind !== 'call' && prev.author.id === m.author.id && Date.parse(m.created_at) - Date.parse(prev.created_at) < 5 * 60000 && !newDay(m, prev);
  }

  function callLength(m: Message) {
    const secs = Math.max(0, (Date.parse(m.call?.ended_at ?? m.created_at) - Date.parse(m.created_at)) / 1000);
    if (secs < 60) return t('chat_call_seconds');
    const mins = Math.round(secs / 60);
    if (mins < 60) return mins === 1 ? t('chat_call_one_minute') : t('chat_call_n_minutes', { n: mins });
    const h = Math.floor(mins / 60);
    const rest = mins % 60;
    return rest ? t('chat_call_hours_minutes', { h, m: rest }) : h === 1 ? t('chat_call_one_hour') : t('chat_call_n_hours', { n: h });
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
    const ok = await attempt(api('POST', `/channels/${ch.id}/messages`, { content: encodeMentions(encode(raw), searchPeople()), attachment_ids: files.map((f) => f.id), reply_to: reply?.id ?? null }));
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
    editText = decodeMentions(m.content);
  }

  async function saveEdit(m: Message) {
    const content = encodeMentions(encode(editText.trim()), searchPeople());
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
      has(perms, P.SEND_MESSAGES) && { label: t('chat_reply'), icon: 'reply', action: () => startReply(m) },
      has(perms, P.ADD_REACTIONS) && { label: t('chat_react'), icon: 'smile', action: () => (picker = { message: m }) },
      own && !!m.content && !m.poll && !m.content.match(STICKER) && { label: t('common_edit'), icon: 'edit', action: () => startEdit(m) },
      'sep',
      !!m.content && !m.content.match(STICKER) && { label: t('chat_copy_text'), icon: 'copy', action: () => copy(decodeMentions(m.content)) },
      !own && { label: t('common_mention'), icon: 'at', action: () => (app.insert = `@${m.author.username} `) },
      canPin && { label: m.pinned ? t('common_unpin') : t('chat_pin'), icon: 'pin', action: () => togglePin(m) },
      { label: t('chat_copy_message_id'), icon: 'id', action: () => copy(m.id) },
      'sep',
      (own || (g && has(perms, P.MANAGE_MESSAGES)) || groupDelete) && { label: t('chat_delete_message'), icon: 'trash', danger: true, action: () => remove(m) },
    ]);
  }

  async function remove(m: Message, e?: MouseEvent) {
    const preview = { name: memberName(g, m.author), text: m.content.match(STICKER) ? t('common_stickers') : plainText(m.content) || (m.poll ? t('chat_poll') : m.attachments.map((a) => a.filename).join(', ')) };
    if (e?.shiftKey || (await ask({ title: t('chat_delete_message'), text: t('chat_sure_want_delete_message'), preview, confirm: t('common_delete'), danger: true, tip: t('chat_hold_shift_while_clicking_trash') }))) attempt(api('DELETE', `/channels/${m.channel_id}/messages/${m.id}`));
  }
</script>

{#snippet headerTools()}
  <button class="icon headtool" class:on={app.side === 'pins'} onclick={() => ((app.side = app.side === 'pins' ? null : 'pins'), app.side && (app.pane = 'members'))} aria-label={t('chat_pinned_messages')} title={t('chat_pinned_messages')} aria-pressed={app.side === 'pins'}><Icon name="pin" /></button>
  <button class="icon headtool searchtool" class:on={app.side === 'search'} onclick={() => ((app.side = 'search'), (app.pane = 'members'))} aria-label={t('common_search_messages')}><Icon name="search" /></button>
  <span class="searchwrap"><SearchBox /></span>
{/snippet}

<section class="panel pane-chat">
  <header class="row">
    <button class="icon mobile" onclick={() => (app.pane = 'side')} aria-label={t('common_channel_list')}><Icon name="menu" /></button>
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
        <Icon name="hash" />
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
        <button class="btn small older" onclick={older}>{t('chat_load_older_messages')}</button>
      {:else if app.messages[ch.id]}
        <p class="start muted">{dmUser ? t('chat_start_conversation_name', { name: title }) : direct ? t('chat_start_group_name', { name: title }) : t('chat_start_name', { name: title })}</p>
      {/if}
      {#each messages as m, i (m.id)}
        {@const own = m.author.id === app.me?.id}
        {@const joined = grouped(m, messages[i - 1])}
        {#if newDay(m, messages[i - 1])}
          <div class="day"><span>{day.format(new Date(m.created_at))}</span></div>
        {/if}
        {#if m.kind === 'call'}
          {@const call = m.call}
          {@const live = !!call && !call.ended_at}
          {@const alone = !!call && call.participants.length <= 1}
          <div class="callnote" class:flash={app.highlight === m.id} data-id={m.id}>
            <span class="phone" class:live class:missed={!live && alone}><Icon name="phone" size={16} /></span>
            <span class="grow">
              {#if live}{t('chat_call_started', { name: memberName(g, m.author) })} <strong>{t('chat_call_live')}</strong>
              {:else if alone && m.author.id !== app.me?.id}{t('chat_call_missed', { name: memberName(g, m.author) })}
              {:else if alone}{t('chat_call_unanswered', { name: memberName(g, m.author) })}
              {:else}{t('chat_call_lasted', { name: memberName(g, m.author), duration: callLength(m) })}{/if}
              <time class="muted" datetime={m.created_at}>{time.format(new Date(m.created_at))}</time>
            </span>
            {#if live && direct && voice.channelId !== direct.id}<button class="btn small primary" onclick={() => joinVoice(direct)}><Icon name="phone" size={14} />{t('common_join')}</button>{/if}
          </div>
        {:else}
        <article class="msg" class:own class:joined class:mention={!own && mentionsMe(m, !!g)} class:flash={app.highlight === m.id} data-id={m.id} oncontextmenu={(e) => editing !== m.id && messageMenu(e, m)}>
          <button class="face" onclick={() => (app.profile = m.author.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.author.id, g))} aria-label={t('chat_profile_name', { name: m.author.display_name })} tabindex={joined ? -1 : 0}>
            {#if !joined}<Avatar id={m.author.id} name={m.author.display_name} image={m.author.avatar} size={34} />{:else}<time class="side" datetime={m.created_at} title={full.format(new Date(m.created_at))}>{time.format(new Date(m.created_at))}</time>{/if}
          </button>
          <div class="content">
            {#if !joined}
              <div class="meta">
                <button class="name" style:color={nameColor(m.author.id)} onclick={() => (app.profile = m.author.id)} oncontextmenu={(e) => openMenu(e, memberMenu(m.author.id, g))}>{memberName(g, m.author)}</button>
                <time datetime={m.created_at} title={full.format(new Date(m.created_at))}>{stamp(m.created_at)}</time>
                {#if m.pinned}<span class="pinmark" title={t('common_pinned')}><Icon name="pin" size={12} /></span>{/if}
              </div>
            {/if}
            {#if m.reply}
              {#if m.reply.deleted}
                <p class="quote muted"><Icon name="reply" size={13} /><em>{t('chat_original_message_was_deleted')}</em></p>
              {:else}
                <button class="quote" onclick={() => jumpTo(g?.id ?? null, m.channel_id, m.reply!.id)}>
                  <Icon name="reply" size={13} />
                  <Avatar id={m.reply.author!.id} name={m.reply.author!.display_name} image={m.reply.author!.avatar} size={16} />
                  <strong style:color={nameColor(m.reply.author!.id)}>{memberName(g, m.reply.author!)}</strong>
                  <span class="ellipsis">{m.reply.content?.match(STICKER) ? t('common_stickers') : (m.reply.content ? plainText(m.reply.content) : m.reply.attachments ? t('common_file') : '')}</span>
                </button>
              {/if}
            {/if}
            {#if editing === m.id}
              <textarea class="input" bind:value={editText} onkeydown={(e) => editKey(e, m)} rows="2"></textarea>
              <div class="row hint">
                <button class="btn small primary" onclick={() => saveEdit(m)}>{t('common_save')}</button>
                <button class="btn small" onclick={() => (editing = null)}>{t('common_cancel')}</button>
              </div>
            {:else if !own && isBlocked(m.author.id) && !revealed.includes(m.id)}
              <button class="hidden muted" onclick={() => (revealed = [...revealed, m.id])}>{t('chat_message_from_blocked_user_show')}</button>
            {:else}
              {@const sticker = m.content.match(STICKER)}
              {#if sticker}
                <img class="sticker" src={asset(sticker[2])} alt={sticker[1]} title={sticker[1]} />
              {:else if m.content}
                <div class="bubble"><Markdown text={m.content} />{#if m.edited_at}<small class="edited"> {t('chat_edited')}</small>{/if}</div>
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
              {#if has(perms, P.SEND_MESSAGES)}<button class="icon" onclick={() => startReply(m)} aria-label={t('chat_reply')} title={t('chat_reply')}><Icon name="reply" size={15} /></button>{/if}
              {#if has(perms, P.ADD_REACTIONS)}<button class="icon" onclick={() => (picker = { message: m })} aria-label={t('chat_react')}><Icon name="smile" size={15} /></button>{/if}
              {#if own && m.content && !m.poll && !m.content.match(STICKER)}<button class="icon" onclick={() => startEdit(m)} aria-label={t('common_edit')}><Icon name="edit" size={15} /></button>{/if}
              {#if own || (g && has(perms, P.MANAGE_MESSAGES)) || groupDelete}<button class="icon" onclick={(e) => remove(m, e)} aria-label={t('common_delete')}><Icon name="trash" size={15} /></button>{/if}
            </div>
          {/if}
        </article>
        {/if}
      {/each}
    </div>
    <p class="typing muted" aria-live="polite">
      {#if typers.length === 1}{t('chat_name_typing', { name: typers[0] })}{:else if typers.length > 1}{t('chat_names_typing', { names: typers.slice(0, 3).join(', ') })}{/if}
    </p>
    {#if has(perms, P.SEND_MESSAGES)}
      {#if pending.length || uploading}
        <div class="pending">
          {#each pending as a (a.id)}
            <span class="chip">
              {#if a.is_image}<img src={asset(a.file)} alt="" />{:else}<Icon name="clip" size={14} />{/if}
              <span class="ellipsis">{a.filename}</span>
              <button onclick={() => (pending = pending.filter((x) => x.id !== a.id))} aria-label={t('common_remove_name', { name: a.filename })}><Icon name="close" size={13} /></button>
            </span>
          {/each}
          {#if uploading}<span class="muted">{t('chat_uploading')}</span>{/if}
        </div>
      {/if}
      {#if replying}
        <div class="replying row">
          <Icon name="reply" size={14} />
          <span class="grow ellipsis">{t('chat_replying_name', { name: memberName(g, replying.author) })}</span>
          <button class="icon" onclick={() => (replying = null)} aria-label={t('chat_cancel_reply')} title={t('chat_cancel_reply')}><Icon name="close" size={14} /></button>
        </div>
      {/if}
      <form class="composer row" onsubmit={(e) => (e.preventDefault(), send())}>
        {#if has(perms, P.ATTACH_FILES)}
          <label class="icon tool" title={t('chat_attach_file')}>
            <Icon name="clip" />
            <input type="file" multiple hidden onchange={(e) => (attach(e.currentTarget.files), (e.currentTarget.value = ''))} />
          </label>
        {/if}
        <textarea class="input" rows="1" maxlength="4000" bind:this={input} bind:value={draft} onkeydown={keydown} {oninput} onpaste={paste} placeholder={t('chat_message_target', { message: w('message'), target: direct ? title : '#' + title })}></textarea>
        <button type="button" class="icon tool" onclick={() => (pollOpen = true)} aria-label={t('common_create_poll')} title={t('common_create_poll')}><Icon name="poll" /></button>
        <button type="button" class="icon tool" onclick={() => (picker = { message: null })} aria-label={t('chat_emojis_stickers')}><Icon name="smile" /></button>
        <button class="btn primary" disabled={(!draft.trim() && !pending.length) || uploading > 0} aria-label={w('send')}><Icon name="send" /><span class="send-label">{w('send')}</span></button>
      </form>
    {:else}
      <p class="readonly muted">{t('chat_not_allowed_write_channel')}</p>
    {/if}
    {#if pollOpen}<PollModal channelId={ch.id} onclose={() => (pollOpen = false)} />{/if}
    {#if picker}<EmojiPicker stickers={!picker.message} {external} onpick={pick} onclose={() => (picker = null)} />{/if}
  {:else}
    <div class="blank">
      <h2>{app.guildId ? t('chat_no_channel_visible') : t('chat_welcome_fire')}</h2>
      <p class="muted">
        {#if app.guildId}{t('chat_there_no_text_channel_here')}
        {:else if app.guilds.length}{t('chat_pick_server_open_conversation', { server: w('server') })}
        {:else}{t('chat_create_first_server_join_one', { server: w('server') })}{/if}
      </p>
      {#if !app.guilds.length}<button class="btn primary" onclick={() => (app.modal = { kind: 'create' })}>{t('common_create_join_server', { server: w('server') })}</button>{/if}
    </div>
  {/if}
</section>

<style>
  .headtool.on {
    color: var(--accent);
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
  .searchtool {
    display: none;
  }
  .searchwrap {
    display: contents;
  }
  @media (max-width: 700px) {
    .searchwrap {
      display: none;
    }
    .searchtool {
      display: inline-flex;
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
    margin: 10px -16px 0;
    padding: 2px 16px;
  }
  .msg:hover {
    background: color-mix(in srgb, var(--ink) 4%, transparent);
  }
  .msg.joined {
    margin-top: 1px;
  }
  .face {
    width: 34px;
    flex: none;
    text-align: center;
  }
  .side {
    display: block;
    padding-top: 9px;
    font-size: 0.68em;
    opacity: 0;
  }
  .msg:hover .side {
    opacity: 1;
  }
  .content {
    min-width: 0;
    max-width: min(78%, 720px);
  }
  .meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 3px;
    font-size: 0.88em;
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
  }
  :global([data-theme='pergament']) .bubble {
    border: 1px solid var(--line);
    border-left: 4px solid var(--accent);
    border-radius: 2px;
  }
  :global([data-theme='pergament']) .own .bubble {
    border-left-color: #9a7636;
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
    .content {
      max-width: 88%;
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
      display: none;
    }
  }
  .composer textarea::placeholder {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .callnote {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 12px 0 4px;
    padding: 8px 12px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    font-size: 0.92em;
  }
  .callnote time {
    margin-left: 6px;
    font-size: 0.85em;
  }
  .callnote .phone {
    display: inline-grid;
    place-items: center;
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 50%;
    background: color-mix(in srgb, var(--muted) 20%, transparent);
    color: var(--muted);
  }
  .callnote .phone.live {
    background: color-mix(in srgb, var(--ok) 22%, transparent);
    color: var(--ok);
  }
  .callnote .phone.missed {
    background: color-mix(in srgb, var(--danger) 18%, transparent);
    color: var(--danger);
  }
  .callnote.flash {
    animation: flash 2s ease-out;
  }
</style>
