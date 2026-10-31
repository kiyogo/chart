import { GoogleIcon, LogoIcon } from '../components/Icons';

export function Login() {
  const handleLogin = () => {
    window.location.href = '/api/auth/google';
  };

  return (
    <div class="auth-container">
      <div class="auth-box">
        <div class="auth-logo">
          <LogoIcon size={44} />
        </div>
        <h1 class="auth-title">chatw</h1>
        <button
          type="button"
          class="auth-btn-primary"
          onClick={handleLogin}
          aria-label="Tiếp tục với Google"
        >
          <GoogleIcon />
          <span>Tiếp tục với Google</span>
        </button>
      </div>
    </div>
  );
}
