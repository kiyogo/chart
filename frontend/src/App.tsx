import { Show, createSignal, onMount } from 'solid-js';
import { api } from './api/client';
import { createAndStoreUserKeys, getUserKeys } from './crypto/e2ee';
import { Chat } from './pages/Chat';
import { Login } from './pages/Login';
import { SetName } from './pages/SetName';
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
    <Show when={status() !== 'loading'} fallback={<div class="empty-chat" />}>
      <Show when={status() === 'unauthenticated'}>
        <Login />
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
  );
}
