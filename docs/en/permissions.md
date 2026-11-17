# Roles and permissions

[Deutsch](../de/rechte.md) · [All docs](../README.md)

## Server roles

Every server has the role `@everyone`, which every member has. Further roles are created under **Server settings → Roles**. Roles higher in the list win and can manage the ones below them. Nobody can edit or give a role that is at or above their own highest role, except the owner.

| Permission | Allows |
|---|---|
| View channels | See channels and read messages |
| Send messages | Write in text channels |
| Attach files | Upload images and files |
| Add reactions | React to messages |
| Use external emojis | Post emojis and stickers from other servers |
| Pin messages | Pin and unpin messages |
| Manage messages | Delete messages of others |
| Manage channels | Create, edit and delete channels |
| Manage roles | Create, edit and give roles below your own |
| Manage server | Name, description, icon, invites, integrations, audit log |
| Manage emojis and stickers | Upload and delete emojis and stickers |
| Join voice channels | Connect to voice |
| Speak | Use the microphone |
| Share screen | Share screen or camera |
| Time out members | Mute members temporarily |
| Kick members | Remove members with a lower role |
| Ban members | Keep members out permanently |
| Create invites | Create invite links |
| Change own nickname | |
| Manage nicknames | Change nicknames of others |
| Administrator | Every permission, ignores channel rules |

## Channel rules

In the channel settings, rules per role or per member allow or deny single permissions for that channel. Member rules beat role rules, denials of `@everyone` are overridden by allowing roles. Threads follow the rules of their channel.

## Group chats

Groups have three fixed roles: administrator (orange), moderator (green) and member (grey). The group settings decide which of them may invite, edit the group, remove members, delete messages of others and pin messages.
