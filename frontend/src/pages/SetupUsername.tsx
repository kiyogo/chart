import { createSignal } from 'solid-js';
import { api } from '../api/client';
import { LogoIcon } from '../components/Icons';
import { createAndStoreUserKeys, getUserKeys } from '../crypto/e2ee';

export function SetupUsername(props: { onComplete: () => void }) {
  const [name, setName] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    const trimmed = name().trim();
    if (trimmed.length < 2 || trimmed.length > 32) {
      setError('Tên phải từ 2 đến 32 ký tự');
      return;
    }

    setLoading(true);
    setError('');

    try {
      await api.users.setName(trimmed);

      let keys = await getUserKeys();
      if (!keys) {
        keys = await createAndStoreUserKeys();
      }

      try {
        await api.keys.register(keys.ecdhRawB64, keys.ecdsaRawB64);
      } catch (err: any) {
        if (err.message !== 'keys_already_registered') {
          throw err;
        }
      }

      props.onComplete();
    } catch (err: any) {
      if (err.message === 'username_taken') {
        setError('Tên này đã được sử dụng');
      } else {
        setError('Không thể lưu tên. Vui lòng thử lại');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="auth-container">
      <form class="auth-box" onSubmit={handleSubmit}>
        <div class="auth-logo">
          <LogoIcon size={44} />
        </div>
        <h2 class="auth-title">Đặt tên của bạn</h2>
        <input
          type="text"
          placeholder="Nhập tên người dùng"
          value={name()}
          onInput={(e) => {
            setName(e.currentTarget.value);
            setError('');
          }}
          disabled={loading()}
          autofocus
          aria-label="Tên người dùng"
        />
        {error() && <div class="error-text">{error()}</div>}
        <button
          type="submit"
          class="auth-btn-primary"
          disabled={loading()}
          aria-label="Tiếp tục"
        >
          {loading() ? 'Đang lưu' : 'Tiếp tục'}
        </button>
      </form>
    </div>
  );
}
