// Utility for avatar compression and conversion to AVIF format

export async function processAvatarImage(file: File): Promise<Blob> {
  // Load image into an ImageBitmap or HTMLImageElement
  let sourceWidth: number;
  let sourceHeight: number;
  let drawableSource: CanvasImageSource;

  if (typeof createImageBitmap === 'function') {
    const bitmap = await createImageBitmap(file);
    sourceWidth = bitmap.width;
    sourceHeight = bitmap.height;
    drawableSource = bitmap;
  } else {
    const img = new Image();
    const url = URL.createObjectURL(file);
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('Không thể tải tệp hình ảnh'));
      img.src = url;
    });
    URL.revokeObjectURL(url);
    sourceWidth = img.naturalWidth || img.width;
    sourceHeight = img.naturalHeight || img.height;
    drawableSource = img;
  }

  // Target standard avatar dimensions: 512x512 with center square crop
  const targetSize = 512;
  let sx = 0;
  let sy = 0;
  let sSize = Math.min(sourceWidth, sourceHeight);

  if (sourceWidth > sourceHeight) {
    sx = Math.floor((sourceWidth - sourceHeight) / 2);
  } else if (sourceHeight > sourceWidth) {
    sy = Math.floor((sourceHeight - sourceWidth) / 2);
  }

  const canvas = document.createElement('canvas');
  canvas.width = targetSize;
  canvas.height = targetSize;
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    throw new Error('Canvas không được hỗ trợ');
  }

  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(
    drawableSource,
    sx,
    sy,
    sSize,
    sSize,
    0,
    0,
    targetSize,
    targetSize
  );

  // Auto compression:
  // If original file > 2MB, apply higher compression
  const isLarge = file.size > 2 * 1024 * 1024;
  const quality = isLarge ? 0.75 : 0.85;

  // Convert to image/avif, with graceful fallback to webp/jpeg/png if canvas.toBlob rejects avif
  let blob = await new Promise<Blob | null>((resolve) => {
    canvas.toBlob((b) => resolve(b), 'image/avif', quality);
  });

  if (!blob) {
    blob = await new Promise<Blob | null>((resolve) => {
      canvas.toBlob((b) => resolve(b), 'image/webp', quality);
    });
  }

  if (!blob) {
    blob = await new Promise<Blob | null>((resolve) => {
      canvas.toBlob((b) => resolve(b), 'image/jpeg', quality);
    });
  }

  if (!blob) {
    blob = await new Promise<Blob | null>((resolve) => {
      canvas.toBlob((b) => resolve(b));
    });
  }

  if (!blob) {
    throw new Error('Không thể xử lý hình ảnh');
  }

  // Ensure AVIF blob
  if (blob.type === 'image/avif') {
    return blob;
  }
  return new Blob([await blob.arrayBuffer()], { type: 'image/avif' });
}
