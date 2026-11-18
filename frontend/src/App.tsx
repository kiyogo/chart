import { Show, createSignal, onMount } from 'solid-js';
import { api } from './api/client';
import { createAndStoreUserKeys, getUserKeys } from './crypto/e2ee';
import { Chat } from './pages/Chat';
import { Login } from './pages/Login';
import { SetupUsername } from './pages/SetupUsername';
import { wsClient } from './websocket/ws';

export function App() {
  const [status, setStatus] = createSignal<'loading' | 'unauthenticated' | 'needs_name' | 'ready'>('loading');
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

      let newlyGenerated = false;
      let keys = await getUserKeys(me.id);
      if (!keys) {
        keys = await createAndStoreUserKeys(me.id);
        newlyGenerated = true;
      }

      if (!me.has_keys || newlyGenerated) {
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

  onMount(() => {
    checkAuth();
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
      <main class="app-content">
        <Show when={status() !== 'loading'} fallback={<div class="empty-chat" />}>
          <Show when={status() === 'unauthenticated'}>
            <Login onLoginSuccess={checkAuth} />
          </Show>
          <Show when={status() === 'needs_name'}>
            <SetupUsername onComplete={checkAuth} />
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
