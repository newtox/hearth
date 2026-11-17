<script lang="ts">
  import { api } from '../lib/api';
  import { locale, t } from '../lib/i18n.svelte';
  import { openMenu } from '../lib/menus';
  import * as mod from '../lib/moderation';
  import { PERMISSIONS, P } from '../lib/perms';
  import { app, attempt, can, canModerate, guild, openDm, presenceOf } from '../lib/store.svelte';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { userId }: { userId: string } = $props();

  const g = $derived(guild()!);
  const m = $derived(g.members.find((x) => x.id === userId));
  const perms = $derived(m ? mod.memberPermissions(g, m) : 0);
  const granted = $derived(PERMISSIONS.filter((p) => perms === -1 || (perms & p.bit) === p.bit));
  const roles = $derived(m ? g.roles.filter((r) => !r.is_default && m.roles.includes(r.id)) : []);
  const DANGER = P.ADMINISTRATOR | P.MANAGE_GUILD | P.MANAGE_ROLES | P.MANAGE_CHANNELS | P.KICK_MEMBERS | P.BAN_MEMBERS | P.MODERATE_MEMBERS | P.MANAGE_MESSAGES;

  let summary = $state<{ messages: number; links: number; media: number; audit: number; created_at: string } | null>(null);

  $effect(() => {
    const id = userId;
    attempt(api<NonNullable<typeof summary>>('GET', `/guilds/${g.id}/members/${id}/summary`)).then((s) => {
      if (s && id === userId) summary = s;
    });
  });

  $effect(() => {
    if (!m) app.modal = null;
  });

  const date = new Intl.DateTimeFormat(locale(), { dateStyle: 'medium' });
  const number = new Intl.NumberFormat(locale());
  const close = () => (app.modal = null);

  function addRole(e: MouseEvent) {
    const list = g.roles.filter((r) => !r.is_default && !m!.roles.includes(r.id) && mod.roleEditable(g, r));
    openMenu(
      e,
      list.length
        ? list.map((r) => ({ label: r.name, color: r.color, action: () => mod.toggleRole(g, userId, r.id) }))
        : [{ label: t('mod_view_no_more_roles_available'), disabled: true }],
    );
  }
</script>

<Modal title={t('mod_view_moderator_view')} onclose={close}>
  {#if m}
    {@const label = mod.label(m)}
    <div class="stack view">
      <div class="row head">
        <Avatar id={m.id} name={m.display_name} image={m.avatar} size={52} status={presenceOf(m)} />
        <div class="grow who">
          <strong class="ellipsis">{label}</strong>
          <span class="muted ellipsis">@{m.username}</span>
        </div>
      </div>
      <div class="row actions">
        {#if m.id !== app.me?.id}<button class="act" onclick={() => (openDm(m.id), close())} title={t('common_send_message')} aria-label={t('common_send_message')}><Icon name="message" /></button>{/if}
        {#if canModerate(g, m.id, P.MODERATE_MEMBERS)}
          <button class="act" class:on={mod.timedOut(m)} onclick={() => (mod.timedOut(m) ? mod.endTimeout(g, m) : mod.timeout(g, m))} title={mod.timedOut(m) ? t('common_end_timeout') : t('common_timeout')} aria-label={mod.timedOut(m) ? t('common_end_timeout') : t('common_timeout')}><Icon name="clock" /></button>
        {/if}
        {#if canModerate(g, m.id, P.KICK_MEMBERS)}<button class="act danger" onclick={() => mod.kick(g, m)} title={t('common_remove')} aria-label={t('common_remove')}><Icon name="userMinus" /></button>{/if}
        {#if canModerate(g, m.id, P.BAN_MEMBERS)}<button class="act danger" onclick={() => mod.ban(g, m)} title={t('common_ban')} aria-label={t('common_ban')}><Icon name="ban" /></button>{/if}
        <button class="act" onclick={() => mod.copy(m.id)} title={t('common_copy_user_id')} aria-label={t('common_copy_user_id')}><Icon name="id" /></button>
      </div>

      <section>
        <h4>{t('mod_view_activity_server', { server: w('server') })}</h4>
        <div class="rows">
          <div class="line"><Icon name="message" size={17} /><span class="grow">{t('mod_view_messages')}</span><b>{summary ? number.format(summary.messages) : '…'}</b></div>
          <div class="line"><Icon name="link" size={17} /><span class="grow">{t('mod_view_links')}</span><b>{summary ? number.format(summary.links) : '…'}</b></div>
          <div class="line"><Icon name="image" size={17} /><span class="grow">{t('mod_view_media')}</span><b>{summary ? number.format(summary.media) : '…'}</b></div>
          {#if can(P.MANAGE_GUILD, g)}
            <button class="line" onclick={() => (app.modal = { kind: 'guild', tab: 'audit' })}><Icon name="list" size={17} /><span class="grow">{t('mod_view_audit_log_entries')}</span><b>{summary ? number.format(summary.audit) : '…'}</b><Icon name="right" size={15} /></button>
          {:else}
            <div class="line"><Icon name="list" size={17} /><span class="grow">{t('mod_view_audit_log_entries')}</span><b>{summary ? number.format(summary.audit) : '…'}</b></div>
          {/if}
        </div>
      </section>

      <section>
        <h4 class="row"><span class="grow">{t('common_permissions')}</span><small class="muted">{perms === -1 ? t('mod_view_all') : granted.length}</small></h4>
        {#if m.id === g.owner_id}
          <p class="muted">{t('mod_view_owners_always_have_every_permission')}</p>
        {:else if perms === -1}
          <p class="muted">{t('mod_view_has_administrator_through_role_therefore')}</p>
        {/if}
        <div class="chips">
          {#each granted as p (p.bit)}
            <span class="perm" class:hot={(p.bit & DANGER) !== 0}>{#if (p.bit & DANGER) !== 0}<Icon name="shield" size={13} />{/if}{t(p.name)}</span>
          {/each}
        </div>
      </section>

      <section>
        <h4>{w('roles')}</h4>
        <div class="chips">
          {#each roles as r (r.id)}
            <span class="role" style:--c={r.color ?? 'var(--muted)'}>
              <i></i>{r.name}
              {#if can(P.MANAGE_ROLES, g) && mod.roleEditable(g, r)}
                <button onclick={() => mod.toggleRole(g, m.id, r.id)} aria-label={t('common_remove_name', { name: r.name })}><Icon name="close" size={12} /></button>
              {/if}
            </span>
          {/each}
          {#if can(P.MANAGE_ROLES, g)}<button class="add" onclick={addRole} aria-label={t('mod_view_add_role')} title={t('mod_view_add_role')}><Icon name="plus" size={14} /></button>{/if}
          {#if !roles.length && !can(P.MANAGE_ROLES, g)}<span class="muted">{t('mod_view_no_roles')}</span>{/if}
        </div>
      </section>

      <section>
        <h4>{t('mod_view_account')}</h4>
        <div class="rows">
          <div class="line"><Icon name="user" size={17} /><span class="grow">{t('mod_view_registered')}</span><b>{summary ? date.format(new Date(summary.created_at)) : '…'}</b></div>
          <div class="line"><Icon name="userPlus" size={17} /><span class="grow">{t('mod_view_joined')}</span><b>{date.format(new Date(m.joined_at))}</b></div>
          {#if mod.timedOut(m)}
            <div class="line warn"><Icon name="clock" size={17} /><span class="grow">{t('mod_view_timed_out_until')}</span><b>{new Date(m.timeout_until!).toLocaleString(locale())}</b></div>
          {/if}
        </div>
      </section>
    </div>
  {/if}
</Modal>

<style>
  .view {
    gap: 14px;
  }
  .head {
    gap: 12px;
  }
  .who {
    display: grid;
    min-width: 0;
  }
  .who strong {
    font-size: 1.15em;
  }
  .actions {
    gap: 6px;
  }
  .act {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 9px 0;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    border: 1px solid var(--line);
    color: var(--ink);
  }
  .act:hover {
    border-color: var(--accent);
  }
  .act.on {
    color: var(--accent);
  }
  .act.danger:hover {
    border-color: var(--danger);
    color: var(--danger);
  }
  h4 {
    margin: 0 0 8px;
    font-family: var(--font-display);
  }
  .rows {
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    overflow: hidden;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 12px;
    text-align: left;
    color: var(--ink);
  }
  .line + .line {
    border-top: 1px solid var(--line);
  }
  .line :global(svg) {
    color: var(--muted);
    flex: none;
  }
  button.line:hover {
    background: var(--panel-2);
  }
  .line.warn,
  .line.warn :global(svg) {
    color: var(--danger);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .perm {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    border: 1px solid var(--line);
    font-size: 0.85em;
    color: var(--muted);
  }
  .perm.hot {
    color: var(--ink);
  }
  .perm.hot :global(svg) {
    color: var(--accent);
  }
  .role {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 6px 3px 9px;
    border-radius: var(--radius-s);
    background: color-mix(in srgb, var(--c) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--c) 45%, transparent);
    font-size: 0.88em;
    font-weight: 600;
  }
  .role i {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--c);
  }
  .role button {
    display: grid;
    place-items: center;
    padding: 2px;
    border-radius: 50%;
    color: var(--muted);
  }
  .role button:hover {
    color: var(--danger);
  }
  .add {
    display: grid;
    place-items: center;
    width: 28px;
    height: 26px;
    border-radius: var(--radius-s);
    border: 1px dashed var(--line);
    color: var(--muted);
  }
  .add:hover {
    color: var(--ink);
    border-color: var(--accent);
  }
</style>
