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

<Modal title={t('common_create_poll')} {onclose}>
  <form class="stack" onsubmit={create}>
    <label class="field"><span>{t('poll_modal_question')}</span><input class="input" bind:value={question} maxlength="200" required /></label>
    <div class="field">
      <span>{t('poll_modal_answers')}</span>
      {#each options as _, i}
        <div class="row">
          <input class="input" bind:value={options[i]} maxlength="80" placeholder={t('poll_modal_answer_n', { n: i + 1 })} required={i < 2} />
          {#if options.length > 2}<button type="button" class="icon" onclick={() => (options = options.filter((_, j) => j !== i))} aria-label={t('poll_modal_remove_answer')}><Icon name="close" size={16} /></button>{/if}
        </div>
      {/each}
      {#if options.length < 10}<button type="button" class="btn small add" onclick={() => (options = [...options, ''])}><Icon name="plus" size={14} />{t('poll_modal_add_answer')}</button>{/if}
    </div>
    <label class="row"><input type="checkbox" bind:checked={multi} /> {t('poll_modal_allow_multiple_answers')}</label>
    <label class="field"><span>{t('poll_modal_duration')}</span>
      <select class="input" bind:value={hours}>
        <option value={1}>{t('common_1_hour')}</option>
        <option value={24}>{t('common_1_day')}</option>
        <option value={168}>{t('common_7_days')}</option>
        <option value={0}>{t('common_unlimited')}</option>
      </select>
    </label>
    <button class="btn primary">{t('common_create_poll')}</button>
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
