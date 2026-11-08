// Typed HTTP API Client for chatw

/**
 * Detect the backend base URL.
 * - In the browser (web app): use relative paths (proxied by Vite → backend).
 * - In Tauri desktop app: the webview loads from a tauri:// origin so we need
 *   an absolute URL. Read from VITE_API_BASE env var, or default to the
 *   local backend port used in production.
 */
/** True when running inside the Tauri desktop shell (bundled or navigated to the backend). */
export function isTauri(): boolean {
  return (
    typeof window !== 'undefined' &&
    ('__TAURI__' in window || '__TAURI_INTERNALS__' in window)
  );
}

/**
 * True when the page is served from the bundled Tauri asset origin
 * (`tauri://localhost` on macOS/Linux, `http://tauri.localhost` on Windows).
 * Relative `/api` URLs do not exist there, which caused the blank-screen login bug.
 */
export function isBundledTauriOrigin(): boolean {
  if (typeof window === 'undefined') return false;
  const { protocol, hostname } = window.location;
  return protocol === 'tauri:' || hostname === 'tauri.localhost';
}

/** Backend origin used by the desktop app. Must match the backend PUBLIC_URL so cookies stay first-party. */
export const DESKTOP_BACKEND_ORIGIN =
  (import.meta.env.VITE_API_BASE as string | undefined) ?? 'http://localhost:8082';

export function getApiBase(): string {
  return isBundledTauriOrigin() ? DESKTOP_BACKEND_ORIGIN : '';
}

export const API_BASE = getApiBase();

export interface MeResponse {
  id: string;
  username: string | null;
  avatar_url: string | null;
  has_keys: boolean;
}

export interface UserSearchResult {
  id: string;
  username: string;
  avatar_url: string | null;
  relationship: 'none' | 'friend' | 'pending_sent' | 'pending_received';
}

export interface PublicKeysResponse {
  user_id: string;
  ecdh_public: string;
  ecdsa_public: string;
  fingerprint: string;
}

export interface PendingRequestItem {
  id: string;
  sender_id: string;
  sender_username: string;
  sender_avatar_url?: string | null;
  created_at: string;
}

export interface FriendItem {
  id: string;
  username: string;
  avatar_url?: string | null;
}

export interface MessageWireItem {
  id: string;
  seq: number;
  sender_id: string;
  recipient_id: string;
  envelope: string;
  created_at: string;
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers || {});
  if (options.body && !(options.body instanceof FormData) && !(options.body instanceof Blob)) {
    if (!headers.has('Content-Type')) {
      headers.set('Content-Type', 'application/json');
    }
  }

  const url = API_BASE + path;
  const res = await fetch(url, {
    ...options,
    headers,
    credentials: 'include',
  });

  if (!res.ok) {
    let errorCode = 'request_failed';
    try {
      const errJson = await res.json();
      if (errJson && errJson.error) {
        errorCode = errJson.error;
      }
    } catch {
      // ignore
    }
    const err = new Error(errorCode);
    (err as any).status = res.status;
    throw err;
  }

  if (res.status === 204) {
    return null as T;
  }

  return res.json();
}

export const api = {
  auth: {
    me: () => request<MeResponse>('/api/auth/me'),
    providers: () => request<{ google: boolean; dev: boolean }>('/api/auth/providers'),
    logout: () => request<void>('/api/auth/logout', { method: 'POST' }),
    devLogin: (username?: string) =>
      request<{ success: boolean; user_id?: string }>('/api/auth/dev-login', {
        method: 'POST',
        body: JSON.stringify({ username }),
      }),
  },
  users: {
    setName: (name: string) =>
      request<{ id: string; username: string }>('/api/users/name', {
        method: 'POST',
        body: JSON.stringify({ name }),
      }),
    search: (q: string) =>
      request<UserSearchResult[]>(`/api/users/search?q=${encodeURIComponent(q)}`),
    uploadAvatar: (blob: Blob) =>
      request<{ avatar_url: string }>('/api/users/avatar', {
        method: 'POST',
        body: blob,
        headers: {
          'Content-Type': 'image/avif',
        },
      }),
  },
  keys: {
    register: (ecdhPublic: string, ecdsaPublic: string) =>
      request<PublicKeysResponse>('/api/keys', {
        method: 'POST',
        body: JSON.stringify({
          ecdh_public: ecdhPublic,
          ecdsa_public: ecdsaPublic,
        }),
      }),
    get: (userId: string) =>
      request<PublicKeysResponse>(`/api/keys/${encodeURIComponent(userId)}`),
  },
  friends: {
    sendRequest: (target: { username?: string; receiver_id?: string } | string) => {
      const body = typeof target === 'string' ? { username: target } : target;
      return request<{ id: string; status: string }>('/api/friends/requests', {
        method: 'POST',
        body: JSON.stringify(body),
      });
    },
    getRequests: () =>
      request<PendingRequestItem[]>('/api/friends/requests'),
    acceptRequest: (id: string) =>
      request<{ id: string; status: string }>(
        `/api/friends/requests/${encodeURIComponent(id)}/accept`,
        { method: 'POST' }
      ),
    getFriends: () =>
      request<FriendItem[]>('/api/friends'),
  },
  messages: {
    send: (payload: {
      message_id: string;
      recipient_id: string;
      recipient_envelope: string;
      sender_envelope: string;
    }) =>
      request<{ id: string; seq: number; created_at: string }>('/api/messages', {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
    get: (friendId: string, beforeSeq?: number, limit?: number) => {
      let q = `/api/messages/${encodeURIComponent(friendId)}?limit=${limit || 50}`;
      if (beforeSeq) {
        q += `&before_seq=${beforeSeq}`;
      }
      return request<MessageWireItem[]>(q);
    },
  },
  typing: {
    send: (friendId: string, isTyping: boolean) =>
      request<void>('/api/typing', {
        method: 'POST',
        body: JSON.stringify({
          friend_id: friendId,
          is_typing: isTyping,
        }),
      }),
  },
};
