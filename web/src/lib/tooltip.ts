let tip: HTMLDivElement | null = null;
let owner: HTMLElement | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;

function hide() {
  if (timer) clearTimeout(timer);
  timer = null;
  tip?.remove();
  tip = null;
  owner = null;
}

function place(el: HTMLElement) {
  if (!tip) return;
  const r = el.getBoundingClientRect();
  const t = tip.getBoundingClientRect();
  if (el.closest('.dock') && r.right + t.width + 12 < window.innerWidth) {
    tip.style.top = `${Math.max(4, r.top + r.height / 2 - t.height / 2)}px`;
    tip.style.left = `${r.right + 10}px`;
    return;
  }
  let top = r.top - t.height - 8;
  if (top < 4) top = r.bottom + 8;
  const left = Math.min(Math.max(4, r.left + r.width / 2 - t.width / 2), window.innerWidth - t.width - 4);
  tip.style.top = `${top}px`;
  tip.style.left = `${left}px`;
}

function show(el: HTMLElement) {
  const text = el.dataset.tip;
  if (!text || !el.isConnected) return;
  tip = document.createElement('div');
  tip.className = 'tooltip';
  tip.setAttribute('role', 'tooltip');
  tip.textContent = text;
  document.body.append(tip);
  place(el);
}

document.addEventListener('pointerover', (e) => {
  if (e.pointerType === 'touch') return;
  const el = (e.target as HTMLElement | null)?.closest('[title], [data-tip]') as HTMLElement | null;
  if (el === owner) return;
  hide();
  if (!el) return;
  const title = el.getAttribute('title');
  if (title) {
    el.dataset.tip = title;
    if (!el.getAttribute('aria-label') && !el.textContent?.trim()) el.setAttribute('aria-label', title);
    el.removeAttribute('title');
  }
  owner = el;
  timer = setTimeout(() => show(el), 350);
});

document.addEventListener('pointerout', (e) => {
  if (owner && !owner.contains(e.relatedTarget as Node | null)) hide();
});

for (const ev of ['pointerdown', 'keydown', 'scroll', 'blur']) window.addEventListener(ev, hide, true);
