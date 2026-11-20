<script lang="ts">
  import { onMount } from 'svelte';
  import { t, locale } from '../lib/i18n.svelte';
  import { api } from '../lib/api';
  import { ask } from '../lib/dialog.svelte';
  import { attempt, toast } from '../lib/store.svelte';
  import Icon from './Icon.svelte';

  type Session = { id: string; created_at: string; last_seen: string; user_agent: string; client: string; ip: string; current: boolean };

  let sessions = $state<Session[] | null>(null);

  async function load() {
    sessions = (await attempt(api<Session[]>('GET', '/users/@me/sessions'))) ?? [];
  }

  onMount(load);

  function system(ua: string) {
    if (/Android/i.test(ua)) return 'Android';
    if (/iPhone|iPad|iPod/i.test(ua)) return 'iOS';
    if (/Windows/i.test(ua)) return 'Windows';
    if (/Macintosh|Mac OS X/i.test(ua)) return 'macOS';
    if (/CrOS/i.test(ua)) return 'ChromeOS';
    if (/Linux|X11/i.test(ua)) return 'Linux';
    return '';
  }

  function browser(ua: string) {
    if (/Edg\//.test(ua)) return 'Edge';
    if (/OPR\//.test(ua)) return 'Opera';
    if (/Firefox\//.test(ua)) return 'Firefox';
    if (/Chrome\//.test(ua)) return 'Chrome';
    if (/Safari\//.test(ua)) return 'Safari';
    return '';
  }

  function describe(s: Session) {
    const name = s.client === 'desktop' ? t('sessions_desktop_app') : s.client === 'android' ? t('sessions_android_app') : browser(s.user_agent) || t('sessions_unknown_device');
    const os = system(s.user_agent);
    const mobile = s.client === 'android' || /Android|iPhone|iPad|Mobile/i.test(s.user_agent);
    return { name: os && s.client !== 'android' ? `${name} · ${os}` : name, mobile };
  }

  function ago(iso: string) {
    const minutes = Math.round((Date.parse(iso) - Date.now()) / 60000);
    const rtf = new Intl.RelativeTimeFormat(locale(), { numeric: 'auto' });
    if (Math.abs(minutes) < 60) return rtf.format(Math.min(0, minutes), 'minute');
    const hours = Math.round(minutes / 60);
    if (Math.abs(hours) < 24) return rtf.format(hours, 'hour');
    return rtf.format(Math.round(hours / 24), 'day');
  }

  function day(iso: string) {
    return new Date(iso).toLocaleDateString(locale(), { day: 'numeric', month: 'long', year: 'numeric' });
  }

  async function end(s: Session) {
    if (!(await ask({ title: t('sessions_sign_out_device'), text: t('sessions_sign_out_device_text', { device: describe(s).name }), confirm: t('sessions_sign_out'), danger: true }))) return;
    if ((await attempt(api('DELETE', `/users/@me/sessions/${s.id}`))) !== undefined) {
      sessions = sessions?.filter((x) => x.id !== s.id) ?? null;
      toast(t('sessions_signed_out'));
    }
  }

  async function endOthers() {
    if (!(await ask({ title: t('sessions_sign_out_others'), text: t('sessions_sign_out_others_text'), confirm: t('sessions_sign_out'), danger: true }))) return;
    if ((await attempt(api('DELETE', '/users/@me/sessions'))) !== undefined) {
      sessions = sessions?.filter((x) => x.current) ?? null;
      toast(t('sessions_signed_out'));
    }
  }
</script>

<p class="muted">{t('sessions_intro')}</p>
{#if sessions}
  <ul class="list">
    {#each sessions as s (s.id)}
      {@const d = describe(s)}
      <li class="item">
        <span class="icon"><Icon name={d.mobile ? 'mobile' : 'monitor'} size={22} /></span>
        <div class="grow">
          <strong>{d.name}</strong>
          <small class="muted">
            {#if s.current}<span class="here">{t('sessions_this_device')}</span>{:else if !s.user_agent && !s.client}{t('sessions_from_before_list')}{:else}{t('sessions_last_active', { when: ago(s.last_seen) })}{/if}
            {#if s.ip}· {s.ip}{/if}
          </small>
          <small class="muted">{t('sessions_signed_in_on', { date: day(s.created_at) })}</small>
        </div>
        {#if !s.current}
          <button class="btn small" onclick={() => end(s)}>{t('sessions_sign_out')}</button>
        {/if}
      </li>
    {/each}
  </ul>
  {#if sessions.length > 1}
    <button class="btn danger" onclick={endOthers}>{t('sessions_sign_out_others')}</button>
  {/if}
{/if}

<style>
  .list {
    list-style: none;
    margin: 12px 0 18px;
    padding: 0;
    display: grid;
    gap: 8px;
    max-width: 560px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 50%;
    background: var(--panel);
    color: var(--muted);
  }
  .grow {
    flex: 1;
    min-width: 0;
    display: grid;
  }
  small {
    overflow-wrap: anywhere;
  }
  .here {
    color: var(--ok);
    font-weight: 600;
  }
</style>
