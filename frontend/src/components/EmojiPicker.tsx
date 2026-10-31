// Compact Unicode Emoji Picker

const EMOJIS = [
  '😀', '😂', '🥹', '❤️', '👍', '🎉',
  '😭', '😍', '🔥', '😊', '🥳', '🙏',
  '👏', '✨', '🚀', '💯', '😎', '🙌',
  '🤝', '🤩', '😴', '🤔', '✌️', '💪'
];

export function EmojiPicker(props: { onSelect: (emoji: string) => void; onClose: () => void }) {
  return (
    <div
      class="emoji-popup"
      role="dialog"
      aria-label="Chọn emoji"
      onClick={(e) => e.stopPropagation()}
    >
      {EMOJIS.map((emoji) => (
        <button
          type="button"
          class="emoji-btn"
          onClick={() => {
            props.onSelect(emoji);
          }}
          aria-label={emoji}
        >
          {emoji}
        </button>
      ))}
    </div>
  );
}
