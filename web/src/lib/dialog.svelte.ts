export type DialogOptions = {
  title: string;
  text?: string;
  confirm: string;
  danger?: boolean;
  tip?: string;
  preview?: { name: string; text: string };
  input?: { value?: string; label?: string; placeholder?: string; match?: string; type?: 'text' | 'number' };
};

type Pending = DialogOptions & { resolve: (value: string | null) => void };

export const dialog = $state<{ current: Pending | null }>({ current: null });

function open(options: DialogOptions) {
  dialog.current?.resolve(null);
  return new Promise<string | null>((resolve) => {
    dialog.current = {
      ...options,
      resolve: (value) => {
        dialog.current = null;
        resolve(value);
      },
    };
  });
}

export async function ask(options: Omit<DialogOptions, 'input'>) {
  return (await open(options)) !== null;
}

export function askText(options: DialogOptions & { input: NonNullable<DialogOptions['input']> }) {
  return open(options);
}
