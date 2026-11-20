import { readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const load = (path) => JSON.parse(readFileSync(require.resolve(`emojibase-data/${path}`), 'utf8'))

const MAX_VERSION = 15.1
const CATEGORIES = [
  ['people', [0, 1]],
  ['nature', [3]],
  ['food', [4]],
  ['activities', [6]],
  ['travel', [5]],
  ['objects', [7]],
  ['symbols', [8]],
  ['flags', [9]],
]

const english = load('en/data.json')
const versions = new Map(english.map((e) => [e.hexcode, e.version]))
const englishWords = new Map(english.map((e) => [e.hexcode, [e.label, ...(e.tags ?? [])]]))
const shortcodes = [load('en/shortcodes/github.json'), load('en/shortcodes/iamcal.json'), load('en/shortcodes/emojibase.json')]

const codesFor = (hex) => {
  const out = []
  for (const set of shortcodes) {
    const v = set[hex]
    if (v) out.push(...(Array.isArray(v) ? v : [v]))
  }
  return [...new Set(out)]
}

const words = (list) =>
  [...new Set(list.flatMap((w) => w.toLowerCase().split(/[\s:,;()“”"']+/)).filter((w) => w.length > 1))].join(' ')

for (const lang of ['de', 'en', 'da']) {
  const data = load(`${lang}/compact.json`)
  const out = CATEGORIES.map(([id, groups]) => {
    const emojis = data
      .filter((e) => groups.includes(e.group) && (versions.get(e.hexcode) ?? 99) <= MAX_VERSION)
      .sort((a, b) => a.group - b.group || a.order - b.order)
      .map((e) => {
        const codes = codesFor(e.hexcode)
        const search = words([e.label, ...(e.tags ?? []), ...(englishWords.get(e.hexcode) ?? []), ...codes])
        return [e.unicode, codes[0] ?? e.label.toLowerCase().replace(/\s+/g, '_'), e.label, search]
      })
    return [id, emojis]
  })
  writeFileSync(new URL(`../src/lib/emoji/${lang}.json`, import.meta.url), JSON.stringify(out))
  console.log(lang, out.reduce((n, [, e]) => n + e.length, 0), 'emojis')
}
