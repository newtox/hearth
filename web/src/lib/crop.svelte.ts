export type CropShape = 'circle' | 'square' | 'banner';

type Pending = { file: File; url: string; shape: CropShape; resolve: (file: File | null) => void };

export const crop = $state<{ current: Pending | null }>({ current: null });

export function cropImage(file: File, shape: CropShape) {
  crop.current?.resolve(null);
  return new Promise<File | null>((resolve) => {
    const url = URL.createObjectURL(file);
    crop.current = {
      file,
      url,
      shape,
      resolve: (result) => {
        URL.revokeObjectURL(url);
        crop.current = null;
        resolve(result);
      },
    };
  });
}

export async function pickImage(e: Event, shape: CropShape) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  return file ? cropImage(file, shape) : null;
}
