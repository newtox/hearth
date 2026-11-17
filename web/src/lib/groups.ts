import { app, type Dm } from './store.svelte';

export const GROUP_ROLES = [
  { id: 0, name: 'Mitglied', color: '#9aa3ad' },
  { id: 1, name: 'Moderator', color: '#3fbf7f' },
  { id: 2, name: 'Administrator', color: '#f0903a' },
];

export const GROUP_RULES: { key: 'invite' | 'edit' | 'kick' | 'delete'; name: string }[] = [
  { key: 'invite', name: 'Freunde hinzufügen' },
  { key: 'edit', name: 'Name und Bild ändern' },
  { key: 'kick', name: 'Mitglieder entfernen' },
  { key: 'delete', name: 'Nachrichten anderer löschen' },
];

export function groupRole(dm: Dm, userId: string | undefined) {
  return (userId && dm.roles?.[userId]) || 0;
}

export function groupMay(dm: Dm, rule: 'invite' | 'edit' | 'kick' | 'delete') {
  return groupRole(dm, app.me?.id) >= (dm.settings?.[rule] ?? 2);
}
