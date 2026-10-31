import { Show, createSignal } from 'solid-js';

// Minimalist Avatar component with image and letter fallback
export function Avatar(props: {
  name: string;
  avatarUrl?: string | null;
  size?: number;
  class?: string;
}) {
  const [imgFailed, setImgFailed] = createSignal(false);
  const size = () => props.size || 36;

  const initial = () => {
    const trimmed = (props.name || '').trim();
    if (!trimmed) return '?';
    return trimmed.charAt(0).toUpperCase();
  };

  // Modern curated color palette
  const palette = [
    '#2563EB', // Blue
    '#7C3AED', // Violet
    '#059669', // Emerald
    '#D97706', // Amber
    '#E11D48', // Rose
    '#0891B2', // Cyan
    '#4F46E5', // Indigo
    '#0D9488', // Teal
  ];

  const bgColor = () => {
    const str = props.name || '';
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = (hash << 5) - hash + str.charCodeAt(i);
      hash |= 0;
    }
    const idx = Math.abs(hash) % palette.length;
    return palette[idx];
  };

  const fontSize = () => {
    const s = size();
    if (s <= 28) return '11px';
    if (s <= 36) return '14px';
    if (s <= 42) return '16px';
    return '24px';
  };

  return (
    <Show
      when={props.avatarUrl && !imgFailed()}
      fallback={
        <div
          class={`user-avatar ${props.class || ''}`}
          style={{
            width: `${size()}px`,
            height: `${size()}px`,
            'min-width': `${size()}px`,
            'min-height': `${size()}px`,
            'background-color': bgColor(),
            'font-size': fontSize(),
          }}
          aria-hidden="true"
        >
          {initial()}
        </div>
      }
    >
      <img
        src={props.avatarUrl!}
        alt={props.name}
        class={`user-avatar ${props.class || ''}`}
        style={{
          width: `${size()}px`,
          height: `${size()}px`,
          'min-width': `${size()}px`,
          'min-height': `${size()}px`,
          'object-fit': 'cover',
        }}
        onError={() => setImgFailed(true)}
      />
    </Show>
  );
}
