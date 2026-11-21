<script lang="ts">
  import { ask } from '../lib/dialog.svelte';
  import { t } from '../lib/i18n.svelte';
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import { CHANNEL_PERMISSIONS } from '../lib/perms';
  import { app, attempt, can, guild, P, type Channel } from '../lib/store.svelte';
  import { openBelow } from '../lib/menus';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { modal: live }: { modal: { channel: Channel | null; parent?: string | null; type?: 'text' | 'category' | 'voice' } } = $props();
  const modal = untrack(() => live);

  const g = $derived(guild()!);
  const existing = $derived(modal.channel ? (g.channels.find((c) => c.id === modal.channel!.id) ?? modal.channel) : null);
  let kind = $state<'text' | 'category' | 'voice'>(modal.channel?.kind ?? modal.type ?? 'text');
  const voiceBits = [P.VIEW_CHANNEL, P.CONNECT, P.SPEAK, P.STREAM];
  const rules = $derived(CHANNEL_PERMISSIONS.filter((p) => (kind === 'voice' ? voiceBits.includes(p.bit) : p.bit === P.VIEW_CHANNEL || !voiceBits.includes(p.bit))));
  const types = [
    { id: 'text' as const, icon: 'hash', name: 'channel_editor_text_channel', hint: 'channel_editor_messages_images_polls_emojis' },
    { id: 'voice' as const, icon: 'volume', name: 'channel_editor_voice_channel', hint: 'channel_editor_talk_together_share_camera_screen' },
  ];
  const categories = $derived(g.channels.filter((c) => c.kind === 'category'));

  let name = $state(modal.channel?.name ?? '');
  let topic = $state(modal.channel?.topic ?? '');
  let parent = $state<string>(modal.channel?.parent_id ?? modal.parent ?? '');

  type Rule = { target_id: string; kind: string; allow: number; deny: number };
  let draft = $state<Rule[]>(untrack(() => (modal.channel ? (guild()!.channels.find((c) => c.id === modal.channel!.id) ?? modal.channel).overwrites.map((o) => ({ ...o })) : [])));
  const key = (list: Rule[]) => JSON.stringify([...list].filter((o) => o.allow || o.deny).sort((x, y) => x.target_id.localeCompare(y.target_id)).map((o) => [o.target_id, o.kind, o.allow, o.deny]));
  const rulesDirty = $derived(!!existing && key(draft) !== key(existing.overwrites));
  const dirty = $derived(existing ? name !== existing.name || topic !== (existing.topic ?? '') || parent !== (existing.parent_id ?? '') || rulesDirty : !!name.trim() || !!topic.trim());
  let saving = $state(false);

  function close() {
    app.modal = null;
  }

  async function dismiss() {
    if (!dirty || (await ask({ title: t('unsaved_title'), text: t('unsaved_text'), confirm: t('unsaved_discard'), danger: true }))) close();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    saving = true;
    try {
      const body = { name, topic, kind, parent_id: kind !== 'category' ? parent || null : null };
      if (!existing) {
        if (await attempt(api('POST', `/guilds/${g.id}/channels`, body))) close();
        return;
      }
      if ((name !== existing.name || topic !== (existing.topic ?? '') || parent !== (existing.parent_id ?? '')) && !(await attempt(api('PATCH', `/channels/${existing.id}`, body)))) return;
      if (rulesDirty && !(await saveRules(existing))) return;
      close();
    } finally {
      saving = false;
    }
  }

  async function saveRules(ch: Channel) {
    const cat = kind !== 'category' && parent ? g.channels.find((c) => c.id === parent) : undefined;
    if (cat && key(draft) === key(cat.overwrites)) return !!(await attempt(api('POST', `/channels/${ch.id}/sync`)));
    const before = new Map(ch.overwrites.map((o) => [o.target_id, o]));
    const after = new Map(draft.filter((o) => o.allow || o.deny).map((o) => [o.target_id, o]));
    for (const id of before.keys()) {
      if (!after.has(id) && !(await attempt(api('DELETE', `/channels/${ch.id}/overwrites/${id}`)))) return false;
    }
    for (const [id, o] of after) {
      const old = before.get(id);
      if (old && old.allow === o.allow && old.deny === o.deny) continue;
      if (!(await attempt(api('PUT', `/channels/${ch.id}/overwrites/${id}`, { kind: o.kind, allow: o.allow, deny: o.deny })))) return false;
    }
    return true;
  }

  function reset() {
    if (!existing) return;
    name = existing.name;
    topic = existing.topic ?? '';
    parent = existing.parent_id ?? '';
    draft = existing.overwrites.map((o) => ({ ...o }));
    extra = [];
    selected = everyone.id;
  }

  async function remove() {
    if (existing && (await ask({ title: t(kind === 'category' ? 'channel_editor_delete_category' : 'channel_editor_delete_channel'), text: t(kind === 'category' ? 'channel_editor_name_deleted_channels_kept' : 'channel_editor_name_deleted_all_its_messages', { name: existing.name }), confirm: t('common_delete'), danger: true }))) {
      await attempt(api('DELETE', `/channels/${existing.id}`));
      close();
    }
  }

  const everyone = $derived(g.roles.find((r) => r.is_default)!);
  const parentCat = $derived(existing?.parent_id ? g.channels.find((c) => c.id === existing!.parent_id) : undefined);
  const synced = $derived(!!existing && !!parentCat && key(draft) === key(parentCat.overwrites));

  let extra = $state<{ id: string; kind: 'role' | 'member' }[]>([]);
  let selected = $state('');
  const targets = $derived.by(() => {
    const set = new Map<string, 'role' | 'member'>();
    set.set(everyone.id, 'role');
    for (const o of draft) set.set(o.target_id, o.kind as 'role' | 'member');
    for (const x of extra) set.set(x.id, x.kind);
    const roles = g.roles.filter((r) => set.get(r.id) === 'role').sort((x, y) => y.position - x.position);
    const members = g.members.filter((m) => set.get(m.id) === 'member');
    return [
      ...roles.map((r) => ({ id: r.id, kind: 'role' as const, name: r.name, color: r.color })),
      ...members.map((m) => ({ id: m.id, kind: 'member' as const, name: m.nickname || m.display_name, color: null })),
    ];
  });
  const current = $derived(targets.find((x) => x.id === selected) ?? targets[0]);
  const groups = $derived(
    [
      { title: 'channel_editor_group_general', bits: [P.VIEW_CHANNEL] },
      { title: 'channel_editor_group_text', bits: [P.SEND_MESSAGES, P.ATTACH_FILES, P.ADD_REACTIONS, P.USE_EXTERNAL_EMOJIS, P.PIN_MESSAGES, P.MANAGE_MESSAGES] },
      { title: 'channel_editor_group_voice', bits: [P.CONNECT, P.SPEAK, P.STREAM] },
    ]
      .filter((x) => kind === 'category' || (kind === 'voice' ? x.title !== 'channel_editor_group_text' : x.title !== 'channel_editor_group_voice'))
      .map((x) => ({ title: x.title, perms: CHANNEL_PERMISSIONS.filter((p) => x.bits.includes(p.bit)) })),
  );

  function rule(targetId: string, bit: number) {
    const o = draft.find((x) => x.target_id === targetId);
    return o && o.allow & bit ? 1 : o && o.deny & bit ? -1 : 0;
  }

  function setState(targetId: string, bit: number, value: number, kind: 'role' | 'member' = 'role') {
    const o = draft.find((x) => x.target_id === targetId);
    let allow = (o?.allow ?? 0) & ~bit;
    let deny = (o?.deny ?? 0) & ~bit;
    if (value === 1) allow |= bit;
    if (value === -1) deny |= bit;
    if (o) {
      o.allow = allow;
      o.deny = deny;
    } else draft.push({ target_id: targetId, kind, allow, deny });
  }

  function pick(id: string, kind: 'role' | 'member') {
    if (!targets.some((x) => x.id === id)) extra = [...extra, { id, kind }];
    selected = id;
  }

  function addTarget(e: MouseEvent) {
    const roles = g.roles.filter((r) => !r.is_default && !targets.some((x) => x.id === r.id)).sort((x, y) => y.position - x.position);
    const members = g.members.filter((m) => !targets.some((x) => x.id === m.id));
    openBelow(e.currentTarget as HTMLElement, [
      ...roles.map((r) => ({ label: r.name, color: r.color ?? 'var(--muted)', action: () => pick(r.id, 'role') })),
      members.length > 0 && 'sep',
      members.length > 0 && { label: t('channel_editor_members'), icon: 'users', items: members.map((m) => ({ label: m.nickname || m.display_name, action: () => pick(m.id, 'member') })) },
    ]);
  }

  function removeTarget() {
    if (!current || current.id === everyone.id) return;
    const id = current.id;
    draft = draft.filter((o) => o.target_id !== id);
    extra = extra.filter((x) => x.id !== id);
    selected = everyone.id;
  }

  async function syncNow() {
    if (parentCat && (await ask({ title: t('channel_editor_sync_title'), text: t('channel_editor_sync_text', { name: parentCat.name }), confirm: t('channel_editor_sync_now') }))) {
      draft = parentCat.overwrites.map((o) => ({ ...o }));
      extra = [];
      selected = everyone.id;
    }
  }

  const states = [
    { value: -1, icon: 'close', label: 'channel_editor_deny', cls: 'no' },
    { value: 0, icon: 'slash', label: 'channel_editor_neutral', cls: '' },
    { value: 1, icon: 'check', label: 'channel_editor_allow', cls: 'yes' },
  ];
</script>

<Modal title={existing ? t(kind === 'category' ? 'common_edit_category' : kind === 'voice' ? 'channel_editor_edit_voice_channel' : 'channel_editor_edit_channel') : t(kind === 'category' ? 'channel_editor_create_category' : kind === 'voice' ? 'channel_editor_create_voice_channel' : 'common_create_channel')} onclose={dismiss} wide={!!existing && can(P.MANAGE_ROLES)}>
  <div class="stack">
    <form class="stack" id="channel-form" onsubmit={save}>
      {#if !existing && kind !== 'category'}
        <div class="field" role="radiogroup" aria-label={t('channel_editor_channel_type')}>
          <span>{t('channel_editor_channel_type')}</span>
          {#each types as k (k.id)}
            <label class="type" class:on={kind === k.id}>
              <Icon name={k.icon} size={22} />
              <span class="grow"><strong>{t(k.name)}</strong><small class="muted">{t(k.hint)}</small></span>
              <input type="radio" name="kind" value={k.id} bind:group={kind} />
            </label>
          {/each}
        </div>
      {/if}
      <label class="field">
        <span>{t('common_name')}</span>
        <input class="input" bind:value={name} maxlength="60" required />
      </label>
      {#if kind === 'text'}
        <label class="field">
          <span>{t('channel_editor_topic')}</span>
          <input class="input" bind:value={topic} maxlength="300" placeholder={t('channel_editor_what_about')} />
        </label>
      {/if}
      {#if kind !== 'category'}
        <label class="field">
          <span>{t('channel_editor_category')}</span>
          <select class="input" bind:value={parent}>
            <option value="">{t('channel_editor_none')}</option>
            {#each categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </label>
      {/if}
      {#if !existing}
        <div class="row">
          <button class="btn primary">{t('channel_editor_create')}</button>
        </div>
      {/if}
    </form>

    {#if existing && can(P.MANAGE_ROLES)}
      <section class="perms">
        <div>
          <h3>{t('channel_editor_permissions')}</h3>
          <p class="muted">{t(kind === 'category' ? 'channel_editor_category_rules_hint' : 'channel_editor_rules_hint')}</p>
        </div>
        {#if parentCat}
          <div class="sync row" class:ok={synced}>
            <Icon name={synced ? 'check' : 'link'} size={16} />
            <span class="grow">{t(synced ? 'channel_editor_synced' : 'channel_editor_not_synced', { name: parentCat.name })}</span>
            {#if !synced}<button class="btn small" onclick={syncNow}>{t('channel_editor_sync_now')}</button>{/if}
          </div>
        {/if}
        <div class="split">
          <div class="targets">
            <div class="row thead">
              <span class="grow muted small">{t('channel_editor_roles_members')}</span>
              <button class="icon" onclick={addTarget} aria-haspopup="menu" aria-label={t('channel_editor_add_target')} title={t('channel_editor_add_target')}><Icon name="plus" size={16} /></button>
            </div>
            {#each targets as x (x.id)}
              <button class="target row" class:on={current?.id === x.id} onclick={() => (selected = x.id)}>
                {#if x.kind === 'role'}<span class="swatch" style:background={x.color ?? 'var(--muted)'}></span>{:else}<Icon name="user" size={14} />{/if}
                <span class="ellipsis">{x.name}</span>
              </button>
            {/each}
          </div>
          {#if current}
            <div class="perm-list">
              <div class="row phead">
                <strong class="grow ellipsis">{current.name}</strong>
                {#if current.id !== everyone.id}<button class="btn small" onclick={removeTarget}>{t('common_remove')}</button>{/if}
              </div>
              {#each groups as grp (grp.title)}
                <h4>{t(grp.title)}</h4>
                {#each grp.perms as p (p.bit)}
                  {@const s = rule(current.id, p.bit)}
                  <div class="perm row">
                    <div class="grow">
                      <strong>{t(p.name)}</strong>
                      {#if p.hint}<small class="muted">{t(p.hint)}</small>{/if}
                    </div>
                    <div class="tri" role="radiogroup" aria-label={t(p.name)}>
                      {#each states as st (st.value)}
                        <button class={st.cls} class:on={s === st.value} role="radio" aria-checked={s === st.value} aria-label={t(st.label)} title={t(st.label)} onclick={() => setState(current.id, p.bit, st.value, current.kind)}><Icon name={st.icon} size={15} /></button>
                      {/each}
                    </div>
                  </div>
                {/each}
              {/each}
            </div>
          {/if}
        </div>
      </section>
    {/if}
    {#if existing}
      <div class="row actions">
        <button class="btn primary" form="channel-form" disabled={!dirty || saving}>{t('common_save_changes')}</button>
        <button type="button" class="btn" disabled={!dirty || saving} onclick={reset}>{t('unsaved_reset')}</button>
        <span class="grow"></span>
        <button type="button" class="btn danger" onclick={remove}>{t('common_delete')}</button>
      </div>
    {/if}
  </div>
</Modal>

<style>
  .type {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    cursor: pointer;
    border: 1px solid transparent;
  }
  .type.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, var(--panel-2));
  }
  .type strong,
  .type small {
    display: block;
  }
  .type input {
    accent-color: var(--accent);
    width: 18px;
    height: 18px;
  }
  .actions {
    flex-wrap: wrap;
    position: sticky;
    bottom: -1px;
    padding: 10px 0 2px;
    background: var(--panel);
    border-top: 1px solid var(--line);
    box-shadow: 0 24px 0 var(--panel);
    z-index: 1;
  }
  .perms {
    display: grid;
    gap: 12px;
    min-width: 0;
  }
  .perms > *,
  .perm > .grow {
    min-width: 0;
  }
  .perm strong,
  .perm small {
    overflow-wrap: anywhere;
  }
  .perms h3 {
    margin: 0 0 4px;
  }
  .perms p {
    margin: 0;
  }
  .sync {
    flex-wrap: wrap;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    border: 1px solid var(--line);
    color: var(--muted);
  }
  .sync.ok {
    color: var(--ok);
  }
  .sync span {
    flex: 1 1 180px;
    color: var(--ink);
  }
  .split {
    display: grid;
    grid-template-columns: minmax(140px, 200px) minmax(0, 1fr);
    gap: 16px;
    min-width: 0;
  }
  .targets {
    display: grid;
    align-content: start;
    gap: 2px;
    min-width: 0;
  }
  .thead {
    padding: 0 4px 4px 8px;
  }
  .target {
    gap: 8px;
    min-width: 0;
    padding: 6px 8px;
    border-radius: var(--radius-s);
    color: var(--muted);
    text-align: left;
  }
  .target:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .target.on {
    background: var(--panel-2);
    color: var(--ink);
    font-weight: 600;
  }
  .icon {
    display: grid;
    place-items: center;
    padding: 4px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .icon:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .perm-list {
    min-width: 0;
  }
  .phead {
    gap: 10px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--line);
  }
  h4 {
    margin: 16px 0 4px;
    font-size: 0.8em;
    color: var(--muted);
  }
  .perm {
    gap: 14px;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
  }
  .perm strong,
  .perm small {
    display: block;
  }
  .perm small {
    margin-top: 2px;
    line-height: 1.35;
  }
  .swatch {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .tri {
    flex: none;
    display: inline-flex;
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    overflow: hidden;
  }
  .tri button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 28px;
    color: var(--muted);
  }
  .tri button + button {
    border-left: 1px solid var(--line);
  }
  .tri button:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .tri button.on {
    background: var(--panel-2);
    color: var(--ink);
  }
  .tri button.on.yes {
    background: var(--ok);
    color: #06200f;
  }
  .tri button.on.no {
    background: var(--danger);
    color: #2a0006;
  }
  :global([data-theme='hell']) .tri button.on.yes,
  :global([data-theme='hell']) .tri button.on.no,
  :global([data-theme='pergament']) .tri button.on.yes,
  :global([data-theme='pergament']) .tri button.on.no {
    color: #fff;
  }
  @media (max-width: 640px) {
    .split {
      grid-template-columns: minmax(0, 1fr);
    }
    .targets {
      grid-auto-flow: row;
    }
  }
</style>
