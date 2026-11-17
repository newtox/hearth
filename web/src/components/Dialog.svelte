<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { dialog } from '../lib/dialog.svelte';
  import Icon from './Icon.svelte';

  const d = $derived(dialog.current);
  let value = $state('');
  let field = $state<HTMLInputElement>();
  let confirmButton = $state<HTMLButtonElement>();

  $effect(() => {
    if (!d) return;
    value = d.input?.value ?? '';
    queueMicrotask(() => (field ?? confirmButton)?.focus());
  });

  const blocked = $derived(!!d?.input?.match && value !== d.input.match);

  function cancel() {
    d?.resolve(null);
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (d && !blocked) d.resolve(d.input ? value : '');
  }

  function key(e: KeyboardEvent) {
    if (!d || e.key !== 'Escape') return;
    e.stopImmediatePropagation();
    e.preventDefault();
    cancel();
  }
</script>

<svelte:window onkeydowncapture={key} />

{#if d}
  <div class="backdrop" onclick={(e) => e.target === e.currentTarget && cancel()} role="presentation">
    <form class="panel dialog" role="alertdialog" aria-modal="true" aria-labelledby="dialog-title" onsubmit={submit}>
      <header class="row">
        <h2 id="dialog-title" class="grow">{d.title}</h2>
        <button type="button" class="x" onclick={cancel} aria-label={t('Schließen')}><Icon name="close" /></button>
      </header>
      <div class="body stack">
        {#if d.text}<p class="muted">{d.text}</p>{/if}
        {#if d.preview}
          <div class="preview">
            <strong>{d.preview.name}</strong>
            <p>{d.preview.text}</p>
          </div>
        {/if}
        {#if d.input}
          <label class="field">
            {#if d.input.label}<span>{d.input.label}</span>{/if}
            <input class="input" bind:this={field} bind:value type={d.input.type ?? 'text'} placeholder={d.input.placeholder} maxlength="512" />
          </label>
        {/if}
        {#if d.tip}<p class="tip"><strong>{t('Tipp:')}</strong> {d.tip}</p>{/if}
      </div>
      <footer class="row">
        <button type="button" class="btn grow" onclick={cancel}>{t('Abbrechen')}</button>
        <button class="btn grow" class:primary={!d.danger} class:danger={d.danger} bind:this={confirmButton} disabled={blocked}>{d.confirm}</button>
      </footer>
    </form>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 95;
    display: grid;
    place-items: center;
    padding: 16px;
    grid-template-columns: minmax(0, 1fr);
    justify-items: center;
    background: rgba(8, 5, 20, 0.6);
  }
  .dialog {
    width: min(440px, 100%);
    max-height: min(92dvh, 640px);
    display: flex;
    flex-direction: column;
  }
  header {
    padding: 18px 20px 6px;
    align-items: flex-start;
  }
  h2 {
    overflow-wrap: anywhere;
  }
  .x {
    padding: 6px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .x:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .body {
    padding: 4px 20px 16px;
    overflow-y: auto;
    min-height: 0;
  }
  .preview {
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    border: 1px solid var(--line);
    max-height: 160px;
    overflow: hidden;
  }
  .preview p {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    margin-top: 2px;
  }
  .tip {
    font-size: 0.85em;
    color: var(--muted);
  }
  .tip strong {
    color: var(--ok);
  }
  footer {
    padding: 0 20px 18px;
    gap: 10px;
  }
  footer .btn {
    justify-content: center;
  }
  footer .btn.danger {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
  footer .btn.danger:hover {
    filter: brightness(1.08);
  }
</style>
