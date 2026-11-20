import { t } from './i18n.svelte';

type Field = HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;

const errors = new WeakMap<Field, HTMLElement>();

function message(el: Field) {
  const v = el.validity;
  if (v.valueMissing) return t('validation_required');
  if (v.typeMismatch && el.type === 'email') return t('validation_email');
  if (v.tooShort && 'minLength' in el) return t('validation_too_short', { n: el.minLength });
  if (v.tooLong && 'maxLength' in el) return t('validation_too_long', { n: el.maxLength });
  if (v.rangeUnderflow && 'min' in el) return t('validation_too_small', { n: el.min });
  if (v.rangeOverflow && 'max' in el) return t('validation_too_large', { n: el.max });
  return t('validation_invalid');
}

function show(el: Field) {
  let out = errors.get(el);
  if (!out?.isConnected) {
    out = document.createElement('span');
    out.className = 'field-error';
    out.setAttribute('role', 'alert');
    const label = el.closest('label');
    if (label) label.append(out);
    else if (el.form) el.form.after(out);
    else el.after(out);
    errors.set(el, out);
  }
  out.textContent = message(el);
  el.classList.add('invalid');
  el.setAttribute('aria-invalid', 'true');
}

function clear(el: Field) {
  errors.get(el)?.remove();
  errors.delete(el);
  el.classList.remove('invalid');
  el.removeAttribute('aria-invalid');
}

document.addEventListener(
  'invalid',
  (e) => {
    const el = e.target as Field;
    e.preventDefault();
    show(el);
    if (el.form?.querySelector(':invalid') === el) el.focus();
  },
  true,
);

document.addEventListener(
  'input',
  (e) => {
    const el = e.target as Field;
    if (errors.has(el) && el.checkValidity()) clear(el);
  },
  true,
);
