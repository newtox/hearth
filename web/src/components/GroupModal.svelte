<script lang="ts">
  import { ask, askText } from '../lib/dialog.svelte';
  import { api, asset, upload } from '../lib/api';
  import { GROUP_ROLES, GROUP_RULES, groupMay, groupRole } from '../lib/groups';
  import { t } from '../lib/i18n.svelte';
  import { app, attempt, dmTitle, select, type Dm, presenceOf } from '../lib/store.svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { dm: given }: { dm: Dm | null } = $props();

  const dm = $derived(given ? (app.dms.find((d) => d.id === given.id) ?? null) : null);
  const candidates = $derived(app.friends.friends.filter((f) => !dm?.recipients.some((r) => r.id === f.id)));
  const owner = $derived(dm?.owner_id === app.me?.id);
  const myRole = $derived(dm ? groupRole(dm, app.me?.id) : 0);
  const admin = $derived(myRole === 2);

  async function pickIcon(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file && dm) await attempt(upload(`/channels/${dm.id}/group/icon`, file));
  }

  let name = $state('');
  let picked = $state<string[]>([]);

  $effect(() => {
    if (given && !dm) app.modal = null;
  });

  function toggle(id: string) {
    picked = picked.includes(id) ? picked.filter((x) => x !== id) : [...picked, id];
  }

  async function create(e: SubmitEvent) {
    e.preventDefault();
    const made = await attempt(api<Dm>('POST', '/users/@me/groups', { user_ids: picked, name }));
    if (!made) return;
    if (!app.dms.some((d) => d.id === made.id)) app.dms.unshift(made);
    app.modal = null;
    select(null, made.id);
  }

  async function rename() {
    const next = await askText({ title: t('group_modal_rename_group'), input: { value: dm!.name ?? '', label: t('group_modal_group_name'), placeholder: t('group_modal_leave_empty_use_members_names') }, confirm: t('common_save') });
    if (next !== null) await attempt(api('PATCH', `/channels/${dm!.id}/group`, { name: next }));
  }

  async function leave() {
    if (await ask({ title: t('group_modal_leave_group'), text: t('group_modal_only_get_back_if_someone'), confirm: t('common_leave'), danger: true })) await attempt(api('DELETE', `/channels/${dm!.id}/recipients/${app.me!.id}`));
  }
</script>

<Modal title={dm ? dmTitle(dm) : t('common_new_group')} onclose={() => (app.modal = null)}>
  {#if dm}
    <div class="stack">
      <div class="row wrap">
        <span class="picture">{#if dm.icon}<img src={asset(dm.icon)} alt="" />{:else}<Icon name="users" size={22} />{/if}</span>
        {#if groupMay(dm, 'edit')}
          <button class="btn small" onclick={rename}><Icon name="edit" size={14} />{t('group_modal_rename')}</button>
          <label class="btn small">{t('group_modal_change_picture')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={pickIcon} hidden /></label>
        {/if}
        <button class="btn small danger" onclick={leave}>{t('group_modal_leave_group')}</button>
      </div>
      <h3>{t('common_members')} ({dm.recipients.length}/10)</h3>
      {#each [...dm.recipients].sort((a, b) => groupRole(dm, b.id) - groupRole(dm, a.id)) as r (r.id)}
        {@const role = GROUP_ROLES[groupRole(dm, r.id)]}
        <div class="row line">
          <button class="row grow who" onclick={() => (app.profile = r.id)}>
            <Avatar id={r.id} name={r.display_name} image={r.avatar} size={32} status={presenceOf(r)} />
            <span class="ellipsis" style:color={role.color}>{r.display_name}</span>
            {#if r.id === dm.owner_id}<span class="crown" title={t('common_owner')}><Icon name="crown" size={14} /></span>{/if}
          </button>
          {#if admin && (r.id !== dm.owner_id || owner) && !(owner && r.id === app.me?.id)}
            <select class="input role" value={groupRole(dm, r.id)} onchange={(e) => attempt(api('PUT', `/channels/${dm.id}/recipients/${r.id}/role`, { role: Number(e.currentTarget.value) }))} aria-label={t('group_modal_role_name', { name: r.display_name })}>
              {#each GROUP_ROLES as o}<option value={o.id}>{t(o.name)}</option>{/each}
            </select>
          {:else}
            <span class="tag" style:--c={role.color}>{t(role.name)}</span>
          {/if}
          {#if r.id !== app.me?.id && r.id !== dm.owner_id && groupMay(dm, 'kick') && (owner || groupRole(dm, r.id) < myRole)}
            <button class="btn small" onclick={() => attempt(api('DELETE', `/channels/${dm.id}/recipients/${r.id}`))}>{t('common_remove')}</button>
          {/if}
        </div>
      {/each}
      {#if candidates.length && dm.recipients.length < 10 && groupMay(dm, 'invite')}
        <h3>{t('common_add_friends')}</h3>
        {#each candidates as f (f.id)}
          <div class="row line">
            <Avatar id={f.id} name={f.display_name} image={f.avatar} size={32} />
            <span class="grow ellipsis">{f.display_name}</span>
            <button class="btn small" onclick={() => attempt(api('PUT', `/channels/${dm.id}/recipients/${f.id}`))}><Icon name="plus" size={14} />{t('group_modal_add')}</button>
          </div>
        {/each}
      {/if}
      <h3>{t('common_permissions')}</h3>
      <p class="muted">{t('group_modal_lowest_role_allowed_do_each')}</p>
      {#each GROUP_RULES as rule (rule.key)}
        <label class="row line">
          <span class="grow">{t(rule.name)}</span>
          <select class="input role" value={dm.settings[rule.key]} disabled={!admin} onchange={(e) => attempt(api('PATCH', `/channels/${dm.id}/group`, { settings: { ...dm.settings, [rule.key]: Number(e.currentTarget.value) } }))}>
            {#each GROUP_ROLES as o}<option value={o.id}>{t(o.name)}</option>{/each}
          </select>
        </label>
      {/each}
    </div>
  {:else}
    <form class="stack" onsubmit={create}>
      <label class="field">
        <span>{t('group_modal_name_optional')}</span>
        <input class="input" bind:value={name} maxlength="60" />
      </label>
      <div>
        <h3>{t('group_modal_pick_friends')}</h3>
        <p class="muted">{t('group_modal_up_9_friends_only_add')}</p>
      </div>
      {#each app.friends.friends as f (f.id)}
        <label class="row line pick">
          <input type="checkbox" checked={picked.includes(f.id)} onchange={() => toggle(f.id)} disabled={!picked.includes(f.id) && picked.length >= 9} />
          <Avatar id={f.id} name={f.display_name} image={f.avatar} size={32} />
          <span class="grow ellipsis">{f.display_name}</span>
        </label>
      {:else}
        <p class="muted">{t('group_modal_have_no_friends_yet_start')}</p>
      {/each}
      <button class="btn primary" disabled={!picked.length}>{t('group_modal_create_group')}</button>
    </form>
  {/if}
</Modal>

<style>
  .line {
    padding: 4px 0;
  }
  .who {
    min-width: 0;
    text-align: left;
  }
  .crown {
    color: var(--accent);
  }
  .wrap {
    flex-wrap: wrap;
  }
  .picture {
    width: 48px;
    height: 48px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--panel-2);
    border: 1px solid var(--line);
    overflow: hidden;
  }
  .picture img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .role {
    width: auto;
    padding: 3px 8px;
    font-size: 0.88em;
  }
  .tag {
    padding: 1px 9px;
    border-radius: 99px;
    border: 1px solid var(--c);
    color: var(--c);
    font-size: 0.82em;
    font-weight: 600;
  }
  .pick {
    cursor: pointer;
  }
  input[type='checkbox'] {
    width: 1.1em;
    height: 1.1em;
    accent-color: var(--accent);
  }
</style>
