import { Show, createSignal, onMount } from 'solid-js';
import { API_BASE, api } from '../api/client';
import { GoogleIcon, LogoIcon } from '../components/Icons';

interface LoginProps {
  onLoginSuccess?: () => void;
}

export function Login(props: LoginProps) {
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [devUsername, setDevUsername] = createSignal('');
  const [providers, setProviders] = createSignal<{ google: boolean; dev: boolean } | null>(null);

  onMount(async () => {
    try {
      setProviders(await api.auth.providers());
    } catch {
      setError('Không kết nối được máy chủ');
    }
  });

  const handleGoogleLogin = () => {
    if (!providers()?.google) {
      setError('Đăng nhập Google chưa được cấu hình trên máy chủ (GOOGLE_CLIENT_ID).');
      return;
    }
    window.location.href = `${API_BASE}/api/auth/google`;
  };

  const handleDevLogin = async (e: Event) => {
    e.preventDefault();
    if (loading()) return;
    setLoading(true);
    setError(null);
    try {
      await api.auth.devLogin(devUsername().trim() || undefined);
      props.onLoginSuccess?.();
    } catch (err: any) {
      setError(err?.message || 'Đăng nhập không thành công');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="auth-container">
      <div class="auth-box">
        <div class="auth-logo">
          <LogoIcon size={44} />
        </div>
        <h1 class="auth-title">chatw</h1>
        <p class="auth-subtitle">Ứng dụng nhắn tin bảo mật E2EE</p>

        <button
          type="button"
          class="auth-btn-primary"
          onClick={handleGoogleLogin}
          disabled={providers() !== null && !providers()!.google}
          aria-label="Tiếp tục với Google"
        >
          <GoogleIcon />
          <span>Tiếp tục với Google</span>
        </button>

        <Show when={providers()?.dev}>
          <div class="auth-divider">
            <span>chế độ phát triển cục bộ</span>
          </div>

          <form class="auth-dev-form" onSubmit={handleDevLogin}>
            <input
              type="text"
              class="auth-input"
              placeholder="Tên tài khoản thử (ví dụ: alice)"
              maxLength={32}
              value={devUsername()}
              onInput={(e) => setDevUsername(e.currentTarget.value)}
            />
            <button type="submit" class="auth-btn-secondary" disabled={loading()}>
              {loading() ? 'Đang đăng nhập' : 'Đăng nhập nhanh'}
            </button>
          </form>
        </Show>

        <Show when={error()}>
          <div class="auth-error">{error()}</div>
        </Show>
      </div>
    </div>
  );
}
