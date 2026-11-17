<script lang="ts">
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { attempt } from '../lib/store.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { channelId, onclose }: { channelId: string; onclose: () => void } = $props();

  let question = $state('');
  let options = $state(['', '']);
  let multi = $state(false);
  let hours = $state(24);

  async function create(e: SubmitEvent) {
    e.preventDefault();
    const poll = { question, options: options.filter((o) => o.trim()), multi, hours };
    if (await attempt(api('POST', `/channels/${channelId}/messages`, { content: '', poll }))) onclose();
  }
</script>

<Modal title={t('Umfrage erstellen')} {onclose}>
  <form class="stack" onsubmit={create}>
    <label class="field"><span>{t('Frage')}</span><input class="input" bind:value={question} maxlength="200" required /></label>
    <div class="field">
      <span>{t('Antworten')}</span>
      {#each options as _, i}
        <div class="row">
          <input class="input" bind:value={options[i]} maxlength="80" placeholder={t('Antwort {n}', { n: i + 1 })} required={i < 2} />
          {#if options.length > 2}<button type="button" class="icon" onclick={() => (options = options.filter((_, j) => j !== i))} aria-label={t('Antwort entfernen')}><Icon name="close" size={16} /></button>{/if}
        </div>
      {/each}
      {#if options.length < 10}<button type="button" class="btn small add" onclick={() => (options = [...options, ''])}><Icon name="plus" size={14} />{t('Antwort hinzufügen')}</button>{/if}
    </div>
    <label class="row"><input type="checkbox" bind:checked={multi} /> {t('Mehrere Antworten erlauben')}</label>
    <label class="field"><span>{t('Laufzeit')}</span>
      <select class="input" bind:value={hours}>
        <option value={1}>{t('1 Stunde')}</option>
        <option value={24}>{t('1 Tag')}</option>
        <option value={168}>{t('7 Tage')}</option>
        <option value={0}>{t('Unbegrenzt')}</option>
      </select>
    </label>
    <button class="btn primary">{t('Umfrage erstellen')}</button>
  </form>
</Modal>

<style>
  .icon {
    padding: 6px;
    color: var(--muted);
  }
  .add {
    justify-self: start;
  }
  input[type='checkbox'] {
    width: 1.1em;
    height: 1.1em;
    accent-color: var(--accent);
  }
</style>
