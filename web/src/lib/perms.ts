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
};

export const PERMISSIONS: { bit: number; name: string; hint: string }[] = [
  { bit: P.VIEW_CHANNEL, name: 'Kanäle ansehen', hint: 'Kanäle sehen und Nachrichten lesen' },
  { bit: P.SEND_MESSAGES, name: 'Nachrichten senden', hint: 'In Textkanälen schreiben' },
  { bit: P.ATTACH_FILES, name: 'Dateien anhängen', hint: 'Bilder und Dateien hochladen' },
  { bit: P.ADD_REACTIONS, name: 'Reaktionen hinzufügen', hint: '' },
  { bit: P.USE_EXTERNAL_EMOJIS, name: 'Externe Emojis verwenden', hint: 'Emojis und Sticker aus anderen Servern posten' },
  { bit: P.MANAGE_MESSAGES, name: 'Nachrichten verwalten', hint: 'Nachrichten anderer löschen' },
  { bit: P.MANAGE_CHANNELS, name: 'Kanäle verwalten', hint: 'Kanäle anlegen, ändern und löschen' },
  { bit: P.MANAGE_ROLES, name: 'Rollen verwalten', hint: 'Rollen unterhalb der eigenen anlegen, ändern und vergeben' },
  { bit: P.MANAGE_GUILD, name: 'Server verwalten', hint: 'Name, Beschreibung, Icon und Einladungen ändern' },
  { bit: P.MANAGE_EMOJIS, name: 'Emojis und Sticker verwalten', hint: 'Eigene Emojis und Sticker hochladen und löschen' },
  { bit: P.CONNECT, name: 'Sprachkanälen beitreten', hint: '' },
  { bit: P.SPEAK, name: 'Sprechen', hint: 'Mikrofon in Sprachkanälen nutzen' },
  { bit: P.STREAM, name: 'Bildschirm teilen', hint: 'Bildschirm oder Kamera in Sprachkanälen übertragen' },
  { bit: P.MODERATE_MEMBERS, name: 'Timeouts vergeben', hint: 'Mitglieder vorübergehend stummschalten' },
  { bit: P.KICK_MEMBERS, name: 'Mitglieder entfernen', hint: 'Mitglieder mit niedrigerer Rolle kicken' },
  { bit: P.BAN_MEMBERS, name: 'Mitglieder bannen', hint: 'Mitglieder dauerhaft aussperren' },
  { bit: P.CREATE_INVITE, name: 'Einladungen erstellen', hint: 'Einladungscodes erzeugen' },
  { bit: P.CHANGE_NICKNAME, name: 'Eigenen Spitznamen ändern', hint: '' },
  { bit: P.MANAGE_NICKNAMES, name: 'Spitznamen verwalten', hint: 'Spitznamen anderer ändern' },
  { bit: P.ADMINISTRATOR, name: 'Administrator', hint: 'Alle Berechtigungen, umgeht Kanalregeln' },
];

export const CHANNEL_PERMISSIONS = PERMISSIONS.filter((p) => [P.VIEW_CHANNEL, P.SEND_MESSAGES, P.ATTACH_FILES, P.ADD_REACTIONS, P.USE_EXTERNAL_EMOJIS, P.MANAGE_MESSAGES, P.CONNECT, P.SPEAK, P.STREAM].includes(p.bit));

export function has(perms: number | undefined, bit: number) {
  return ((perms ?? 0) & bit) === bit;
}
