import { app, type Dm } from './store.svelte';

export const GROUP_ROLES = [
  { id: 0, name: 'group_member', color: '#9aa3ad' },
  { id: 1, name: 'group_moderator', color: '#3fbf7f' },
  { id: 2, name: 'common_administrator', color: '#f0903a' },
];

export const GROUP_RULES: { key: 'invite' | 'edit' | 'kick' | 'delete' | 'pin'; name: string }[] = [
  { key: 'invite', name: 'common_add_friends' },
  { key: 'edit', name: 'group_change_name_picture' },
  { key: 'kick', name: 'common_kick_members' },
  { key: 'delete', name: 'common_delete_other_people_s_messages' },
  { key: 'pin', name: 'common_pin_messages' },
];

export function groupRole(dm: Dm, userId: string | undefined) {
  return (userId && dm.roles?.[userId]) || 0;
}

export function groupMay(dm: Dm, rule: 'invite' | 'edit' | 'kick' | 'delete' | 'pin') {
  return groupRole(dm, app.me?.id) >= (dm.settings?.[rule] ?? 2);
}
