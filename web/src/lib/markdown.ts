export type Node =
  | { t: 'text'; v: string }
  | { t: 'bold' | 'italic' | 'strike'; c: Node[] }
  | { t: 'code' | 'block' | 'link'; v: string }
  | { t: 'emoji'; name: string; file: string }
  | { t: 'mention'; v: string; id?: string };

const INLINE = /(`[^`\n]+`)|(\*\*[^*]+?\*\*)|(~~[^~]+?~~)|(\*[^*\n]+?\*)|(https?:\/\/[^\s<]+)|<:([a-z0-9_]{2,32}):([a-z0-9]+\.[a-z0-9]+)>|(?<![\w@])@(everyone|[a-z0-9_.]{2,32})(?![\w.])|<@([0-9a-f-]{36})>/gi;
export const MENTION_ID = /<@([0-9a-f-]{36})>/gi;
export const STICKER = /^<sticker:([a-z0-9_]{2,32}):([a-z0-9]+\.[a-z0-9]+)>$/i;
export const CUSTOM = /^<:([a-z0-9_]{2,32}):([a-z0-9]+\.[a-z0-9]+)>$/i;

function inline(text: string): Node[] {
  const out: Node[] = [];
  let last = 0;
  for (const m of text.matchAll(INLINE)) {
    if (m.index > last) out.push({ t: 'text', v: text.slice(last, m.index) });
    if (m[1]) out.push({ t: 'code', v: m[1].slice(1, -1) });
    else if (m[2]) out.push({ t: 'bold', c: inline(m[2].slice(2, -2)) });
    else if (m[3]) out.push({ t: 'strike', c: inline(m[3].slice(2, -2)) });
    else if (m[4]) out.push({ t: 'italic', c: inline(m[4].slice(1, -1)) });
    else if (m[5]) out.push({ t: 'link', v: m[5] });
    else if (m[8]) out.push({ t: 'mention', v: m[8].toLowerCase() });
    else if (m[9]) out.push({ t: 'mention', v: '', id: m[9].toLowerCase() });
    else out.push({ t: 'emoji', name: m[6], file: m[7] });
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ t: 'text', v: text.slice(last) });
  return out;
}

export function parse(text: string): Node[] {
  const out: Node[] = [];
  const parts = text.split(/```(?:[a-z0-9]*\n)?([\s\S]*?)```/g);
  parts.forEach((part, i) => {
    if (i % 2) out.push({ t: 'block', v: part.replace(/\n$/, '') });
    else if (part) out.push(...inline(i ? part.replace(/^\n/, '') : part));
  });
  return out;
}

export function onlyEmoji(nodes: Node[]) {
  const emojis = nodes.filter((n) => n.t === 'emoji').length;
  return emojis > 0 && emojis <= 6 && nodes.every((n) => n.t === 'emoji' || (n.t === 'text' && !n.v.trim()));
}
