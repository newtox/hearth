<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import { app, attempt, putGuild, select, type Guild } from '../lib/store.svelte';
  import { w } from '../lib/theme.svelte';
  import Avatar from './Avatar.svelte';
  import Modal from './Modal.svelte';

  let { code: given = '' }: { code?: string } = $props();
  const initial = untrack(() => given);

  let name = $state('');
  let code = $state(initial);
  let preview = $state<{ guild_id: string; name: string; icon: string | null; members: number } | null>(null);

  function parse(v: string) {
    return v.trim().split('/').pop() ?? '';
  }

  async function finish(g: Guild | undefined) {
    if (!g) return;
    putGuild(g);
    app.modal = null;
    select(g.id);
    app.pane = 'chat';
  }

  async function create(e: SubmitEvent) {
    e.preventDefault();
    finish(await attempt(api<Guild>('POST', '/guilds', { name, category: t('Allgemeines'), channel: t('allgemein') })));
  }

  async function look(e: SubmitEvent) {
    e.preventDefault();
    preview = (await attempt(api('GET', `/invites/${encodeURIComponent(parse(code))}`))) ?? null;
  }

  async function join() {
    finish(await attempt(api<Guild>('POST', `/invites/${encodeURIComponent(parse(code))}`)));
  }

  if (initial) attempt(api('GET', `/invites/${encodeURIComponent(initial)}`)).then((p) => (preview = p ?? null));
</script>

<Modal title={t('{server} erstellen oder beitreten', { server: w('server') })} onclose={() => (app.modal = null)}>
  <div class="stack">
    <form class="stack" onsubmit={look}>
      <label class="field">
        <span>{t('Einladungscode oder Link')}</span>
        <div class="row">
          <input class="input" bind:value={code} placeholder={t('z. B. aB3dE5fG7h')} required />
          <button class="btn">{t('Prüfen')}</button>
        </div>
      </label>
    </form>
    {#if preview}
      <div class="row found">
        <Avatar id={preview.guild_id} name={preview.name} image={preview.icon} size={44} square />
        <div class="grow">
          <strong class="ellipsis">{preview.name}</strong>
          <p class="muted">{preview.members} {w('members')}</p>
        </div>
        <button class="btn primary" onclick={join}>{t('Beitreten')}</button>
      </div>
    {/if}
    <hr />
    <form class="stack" onsubmit={create}>
      <label class="field">
        <span>{t('Oder einen neuen {server} gründen', { server: w('server') })}</span>
        <div class="row">
          <input class="input" bind:value={name} placeholder={t('Name')} maxlength="60" required />
          <button class="btn primary">{t('Erstellen')}</button>
        </div>
      </label>
    </form>
  </div>
</Modal>

<style>
  .found {
    padding: 10px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
  hr {
    width: 100%;
    border: 0;
    border-top: 1px solid var(--line);
    margin: 0;
  }
</style>
