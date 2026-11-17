<script lang="ts">
  import { api } from '../lib/api';
  import { locale, t } from '../lib/i18n.svelte';
  import { app, attempt, type Message } from '../lib/store.svelte';

  let { message }: { message: Message } = $props();

  const poll = $derived(message.poll!);
  const me = $derived(app.me?.id ?? '');
  const voters = $derived(new Set(poll.options.flatMap((o) => o.voters)).size);
  const total = $derived(poll.options.reduce((n, o) => n + o.voters.length, 0));
  const closed = $derived(!!poll.expires_at && Date.parse(poll.expires_at) < Date.now());
  const date = new Intl.DateTimeFormat(locale(), { dateStyle: 'medium', timeStyle: 'short' });

  function vote(id: string) {
    const mine = poll.options.filter((o) => o.voters.includes(me)).map((o) => o.id);
    const next = mine.includes(id) ? mine.filter((x) => x !== id) : poll.multi ? [...mine, id] : [id];
    attempt(api('POST', `/channels/${message.channel_id}/messages/${message.id}/poll`, { option_ids: next }));
  }
</script>

<div class="poll">
  <strong class="question">{poll.question}</strong>
  <small class="muted">{poll.multi ? t('poll_card_multiple_answers_allowed') : t('poll_card_one_answer')}</small>
  {#each poll.options as o (o.id)}
    {@const share = total ? o.voters.length / total : 0}
    {@const mine = o.voters.includes(me)}
    <button class="option" class:mine disabled={closed} onclick={() => vote(o.id)} aria-pressed={mine}>
      <i style:width="{share * 100}%"></i>
      <span class="grow">{o.text}</span>
      <span class="count">{o.voters.length} ({Math.round(share * 100)} %)</span>
    </button>
  {/each}
  <small class="muted">
    {voters === 1 ? t('poll_card_1_vote') : t('poll_card_n_votes', { n: voters })}{#if poll.expires_at}, {closed ? t('poll_card_ended') : t('poll_card_open_until_date', { date: date.format(new Date(poll.expires_at)) })}{/if}
  </small>
</div>

<style>
  .poll {
    display: grid;
    gap: 6px;
    width: min(420px, 100%);
    padding: 12px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    text-align: left;
  }
  .question {
    font-size: 1.05em;
    overflow-wrap: anywhere;
  }
  .option {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line);
    overflow: hidden;
    text-align: left;
  }
  .option:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .option:disabled {
    cursor: default;
  }
  .option.mine {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .option i {
    position: absolute;
    inset: 0 auto 0 0;
    background: color-mix(in srgb, var(--accent) 24%, transparent);
  }
  .option span {
    position: relative;
    overflow-wrap: anywhere;
  }
  .count {
    flex: none;
    font-size: 0.85em;
    color: var(--muted);
  }
</style>
