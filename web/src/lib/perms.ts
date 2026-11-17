export const P = {
  VIEW_CHANNEL: 1 << 0,
  SEND_MESSAGES: 1 << 1,
  MANAGE_MESSAGES: 1 << 2,
  MANAGE_CHANNELS: 1 << 3,
  MANAGE_ROLES: 1 << 4,
  MANAGE_GUILD: 1 << 5,
  KICK_MEMBERS: 1 << 6,
  BAN_MEMBERS: 1 << 7,
  CREATE_INVITE: 1 << 8,
  CHANGE_NICKNAME: 1 << 9,
  MANAGE_NICKNAMES: 1 << 10,
  ADMINISTRATOR: 1 << 11,
  ATTACH_FILES: 1 << 12,
  ADD_REACTIONS: 1 << 13,
  CONNECT: 1 << 15,
  SPEAK: 1 << 16,
  STREAM: 1 << 17,
  MODERATE_MEMBERS: 1 << 18,
  MANAGE_EMOJIS: 1 << 19,
  USE_EXTERNAL_EMOJIS: 1 << 20,
  PIN_MESSAGES: 1 << 21,
};

export const PERMISSIONS: { bit: number; name: string; hint: string }[] = [
  { bit: P.VIEW_CHANNEL, name: 'perm_view_channels', hint: 'perm_see_channels_read_messages' },
  { bit: P.SEND_MESSAGES, name: 'perm_send_messages', hint: 'perm_write_text_channels' },
  { bit: P.ATTACH_FILES, name: 'perm_attach_files', hint: 'perm_upload_images_files' },
  { bit: P.ADD_REACTIONS, name: 'perm_add_reactions', hint: '' },
  { bit: P.USE_EXTERNAL_EMOJIS, name: 'perm_use_external_emojis', hint: 'perm_post_emojis_stickers_from_other' },
  { bit: P.PIN_MESSAGES, name: 'common_pin_messages', hint: 'perm_pin_unpin_messages' },
  { bit: P.MANAGE_MESSAGES, name: 'perm_manage_messages', hint: 'common_delete_other_people_s_messages' },
  { bit: P.MANAGE_CHANNELS, name: 'perm_manage_channels', hint: 'perm_create_edit_delete_channels' },
  { bit: P.MANAGE_ROLES, name: 'perm_manage_roles', hint: 'perm_create_edit_assign_roles_below' },
  { bit: P.MANAGE_GUILD, name: 'perm_manage_server', hint: 'perm_change_name_description_icon_invites' },
  { bit: P.MANAGE_EMOJIS, name: 'perm_manage_emojis_stickers', hint: 'perm_upload_delete_custom_emojis_stickers' },
  { bit: P.CONNECT, name: 'perm_join_voice_channels', hint: '' },
  { bit: P.SPEAK, name: 'perm_speak', hint: 'perm_use_microphone_voice_channels' },
  { bit: P.STREAM, name: 'common_share_screen', hint: 'perm_stream_screen_camera_voice_channels' },
  { bit: P.MODERATE_MEMBERS, name: 'perm_time_out_members', hint: 'perm_temporarily_silence_members' },
  { bit: P.KICK_MEMBERS, name: 'common_kick_members', hint: 'perm_kick_members_lower_role' },
  { bit: P.BAN_MEMBERS, name: 'perm_ban_members', hint: 'perm_lock_members_out_permanently' },
  { bit: P.CREATE_INVITE, name: 'perm_create_invites', hint: 'perm_generate_invite_codes' },
  { bit: P.CHANGE_NICKNAME, name: 'perm_change_own_nickname', hint: '' },
  { bit: P.MANAGE_NICKNAMES, name: 'perm_manage_nicknames', hint: 'perm_change_other_people_s_nicknames' },
  { bit: P.ADMINISTRATOR, name: 'common_administrator', hint: 'perm_all_permissions_bypasses_channel_rules' },
];

export const CHANNEL_PERMISSIONS = PERMISSIONS.filter((p) => [P.VIEW_CHANNEL, P.SEND_MESSAGES, P.ATTACH_FILES, P.ADD_REACTIONS, P.USE_EXTERNAL_EMOJIS, P.PIN_MESSAGES, P.MANAGE_MESSAGES, P.CONNECT, P.SPEAK, P.STREAM].includes(p.bit));

export function has(perms: number | undefined, bit: number) {
  return ((perms ?? 0) & bit) === bit;
}
