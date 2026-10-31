// Typed HTTP API Client for chatw

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

  const res = await fetch(path, {
    ...options,
    headers,
    credentials: 'same-origin',
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
    logout: () => request<void>('/api/auth/logout', { method: 'POST' }),
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
