import { Show, createSignal, onMount } from 'solid-js';
import { DESKTOP_BACKEND_ORIGIN, api, isBundledTauriOrigin } from './api/client';
import { Titlebar } from './components/Titlebar';
import { createAndStoreUserKeys, getUserKeys } from './crypto/e2ee';
import { Chat } from './pages/Chat';
import { Login } from './pages/Login';
import { SetName } from './pages/SetName';
import { wsClient } from './websocket/ws';

export function App() {
  const [status, setStatus] = createSignal<'loading' | 'offline' | 'unauthenticated' | 'needs_name' | 'ready'>('loading');
  const [currentUser, setCurrentUser] = createSignal<{
    id: string;
    username: string;
    avatarUrl: string | null;
  } | null>(null);

  const checkAuth = async () => {
    try {
      const me = await api.auth.me();
      if (!me.username) {
        setCurrentUser({ id: me.id, username: '', avatarUrl: me.avatar_url });
        setStatus('needs_name');
        return;
      }

      // User has a username; ensure E2EE keys are set up
      let keys = await getUserKeys();
      if (!keys) {
        keys = await createAndStoreUserKeys();
      }

      if (!me.has_keys) {
        try {
          await api.keys.register(keys.ecdhRawB64, keys.ecdsaRawB64);
        } catch {
          // ignore if already registered
        }
      }

      setCurrentUser({
        id: me.id,
        username: me.username,
        avatarUrl: me.avatar_url,
      });
      setStatus('ready');
    } catch {
      setStatus('unauthenticated');
    }
  };

  /**
   * Desktop launcher: the bundled Tauri page cannot share first-party cookies with the
   * backend, and Google OAuth must return to the backend origin. So once the backend is
   * reachable we load the app from it; otherwise we stay here and show a retry screen
   * (the custom titlebar keeps the window movable/closable).
   */
  const connectDesktopBackend = async (retries = 6) => {
    setStatus('loading');
    for (let i = 0; i < retries; i++) {
      try {
        const res = await fetch(`${DESKTOP_BACKEND_ORIGIN}/api/auth/providers`, { cache: 'no-store' });
        if (res.ok) {
          window.location.replace(`${DESKTOP_BACKEND_ORIGIN}/`);
          return;
        }
      } catch {
        // Retry after delay
      }
      if (i < retries - 1) {
        await new Promise((r) => setTimeout(r, 600));
      }
    }
    setStatus('offline');
  };

  onMount(() => {
    if (isBundledTauriOrigin()) {
      connectDesktopBackend();
    } else {
      checkAuth();
    }
  });

  const handleLogout = async () => {
    try {
      await api.auth.logout();
    } catch {
      // ignore
    }
    wsClient.disconnect();
    setCurrentUser(null);
    setStatus('unauthenticated');
  };

  return (
    <div class="app-layout">
      <Titlebar />
      <main class="app-content">
        <Show when={status() !== 'loading'} fallback={<div class="empty-chat" />}>
          <Show when={status() === 'offline'}>
            <div class="auth-container">
              <div class="auth-box">
                <h1 class="auth-title">Không kết nối được máy chủ</h1>
                <p class="auth-subtitle">
                  Hãy khởi động backend chatw tại {DESKTOP_BACKEND_ORIGIN} rồi thử lại.
                </p>
                <button type="button" class="auth-btn-primary" onClick={() => connectDesktopBackend()}>
                  Thử lại
                </button>
              </div>
            </div>
          </Show>
          <Show when={status() === 'unauthenticated'}>
            <Login onLoginSuccess={checkAuth} />
          </Show>
          <Show when={status() === 'needs_name'}>
            <SetName onNameSet={checkAuth} />
          </Show>
          <Show when={status() === 'ready' && currentUser()}>
            <Chat
              currentUserId={currentUser()!.id}
              currentUsername={currentUser()!.username}
              currentUserAvatarUrl={currentUser()!.avatarUrl}
              onLogout={handleLogout}
              onAvatarUpdated={(newUrl) => {
                setCurrentUser((prev) => (prev ? { ...prev, avatarUrl: newUrl } : null));
              }}
            />
          </Show>
        </Show>
      </main>
    </div>
  );
}
