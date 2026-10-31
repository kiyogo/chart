import {
  For,
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
} from 'solid-js';
import {
  FriendItem,
  PendingRequestItem,
  UserSearchResult,
  api,
} from '../api/client';
import { EmojiPicker } from '../components/EmojiPicker';
import { Avatar } from '../components/Avatar';
import {
  BackIcon,
  CloseIcon,
  EmojiIcon,
  FriendsIcon,
  LogoIcon,
  LogoutIcon,
  MenuIcon,
  SendIcon,
  SettingsIcon,
} from '../components/Icons';
import {
  PeerKeys,
  UserKeys,
  base64UrlEncode,
  decryptChatMessage,
  encryptChatMessage,
  getOrVerifyPeerKeys,
  getUserKeys,
} from '../crypto/e2ee';
import { processAvatarImage } from '../utils/image';
import { wsClient } from '../websocket/ws';

export interface DisplayMessage {
  id: string;
  seq: number;
  senderId: string;
  recipientId: string;
  text: string;
  createdAt: string;
  isSelf: boolean;
  status: 'sent' | 'pending' | 'error';
}

export function Chat(props: {
  currentUserId: string;
  currentUsername: string;
  currentUserAvatarUrl?: string | null;
  onLogout: () => void;
  onAvatarUpdated?: (url: string) => void;
}) {
  // State
  const [userKeys, setUserKeys] = createSignal<UserKeys | null>(null);
  const [friends, setFriends] = createSignal<FriendItem[]>([]);
  const [activeFriend, setActiveFriend] = createSignal<FriendItem | null>(null);
  const [messages, setMessages] = createSignal<DisplayMessage[]>([]);
  const [inputText, setInputText] = createSignal('');
  const [isEmojiOpen, setIsEmojiOpen] = createSignal(false);
  const [isFriendTyping, setIsFriendTyping] = createSignal(false);
  const [isSending, setIsSending] = createSignal(false);
  const [isSettingsOpen, setIsSettingsOpen] = createSignal(false);

  // Avatar state
  const [myAvatarUrl, setMyAvatarUrl] = createSignal<string | null | undefined>(props.currentUserAvatarUrl);
  createEffect(() => {
    setMyAvatarUrl(props.currentUserAvatarUrl);
  });
  const [isUploadingAvatar, setIsUploadingAvatar] = createSignal(false);
  const [avatarUploadError, setAvatarUploadError] = createSignal<string | null>(null);

  // Partner Info Sidebar state (responsive auto-open on >= 1024px)
  const [isPartnerInfoOpen, setIsPartnerInfoOpen] = createSignal(
    typeof window !== 'undefined' ? window.innerWidth >= 1024 : false
  );
  const [partnerFingerprint, setPartnerFingerprint] = createSignal<string | null>(null);

  // Sidebar Tabs: 'chats' | 'friends'
  const [sidebarTab, setSidebarTab] = createSignal<'chats' | 'friends'>('chats');
  const [friendInput, setFriendInput] = createSignal('');
  const [searchResults, setSearchResults] = createSignal<UserSearchResult[]>([]);
  const [addFriendStatus, setAddFriendStatus] = createSignal<{
    type: 'error' | 'success';
    message: string;
  } | null>(null);
  const [isAddingFriend, setIsAddingFriend] = createSignal(false);
  const [pendingRequests, setPendingRequests] = createSignal<PendingRequestItem[]>([]);

  // Persist previews in localStorage to survive page reload
  const previewStorageKey = `chatw_previews_${props.currentUserId}`;
  const savedPreviews: Record<string, string> = (() => {
    try { return JSON.parse(localStorage.getItem(previewStorageKey) || '{}'); } catch { return {}; }
  })();
  const [lastPreviews, setLastPreviews] = createSignal<Record<string, string>>(savedPreviews);
  const [unreadCounts, setUnreadCounts] = createSignal<Record<string, number>>({});

  // Wrap setLastPreviews to also persist to localStorage
  const updatePreview = (partnerId: string, text: string) => {
    setLastPreviews((prev) => {
      const next = { ...prev, [partnerId]: text };
      try { localStorage.setItem(previewStorageKey, JSON.stringify(next)); } catch { /* ignore */ }
      return next;
    });
  };

  const totalUnreadCount = () =>
    Object.values(unreadCounts()).reduce((sum, c) => sum + c, 0);

  // Unlock AudioContext on first user interaction so chime works without autoplay error
  let audioCtx: AudioContext | null = null;
  const unlockAudio = () => {
    try {
      const AC = window.AudioContext || (window as any).webkitAudioContext;
      if (AC && !audioCtx) {
        audioCtx = new AC();
        // Resume so subsequent calls work immediately
        if (audioCtx.state === 'suspended') audioCtx.resume();
      }
    } catch {
      // ignore
    }
  };

  const playNotificationChime = () => {
    try {
      const AC = window.AudioContext || (window as any).webkitAudioContext;
      if (!AC) return;
      const ctx = audioCtx || new AC();
      if (!audioCtx) audioCtx = ctx;
      if (ctx.state === 'suspended') {
        ctx.resume().then(() => _chime(ctx));
      } else {
        _chime(ctx);
      }
    } catch {
      // ignore
    }
  };

  const _chime = (ctx: AudioContext) => {
    try {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(880, ctx.currentTime);
      osc.frequency.exponentialRampToValueAtTime(660, ctx.currentTime + 0.15);
      gain.gain.setValueAtTime(0.18, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.3);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.3);
    } catch {
      // ignore
    }
  };

  createEffect(() => {
    const total = totalUnreadCount();
    const reqs = pendingRequests().length;
    const combined = total + reqs;
    if (typeof document !== 'undefined') {
      document.title = combined > 0 ? `(${combined}) chatw` : 'chatw';
    }
  });

  // Peer keys cache in memory
  const peerKeysCache = new Map<string, PeerKeys>();

  let messagesContainerRef: HTMLDivElement | undefined;
  let typingTimeout: any = null;
  let isCurrentlyTyping = false;
  let inputRef: HTMLInputElement | undefined;

  const handleWindowResize = () => {
    if (typeof window !== 'undefined' && window.innerWidth >= 1024 && !isPartnerInfoOpen()) {
      setIsPartnerInfoOpen(true);
    }
  };

  // Load user keys & initial friends list
  onMount(async () => {
    if (typeof window !== 'undefined') {
      window.addEventListener('resize', handleWindowResize);
      // Unlock AudioContext on first interaction to bypass autoplay policy
      document.addEventListener('click', unlockAudio, { once: true });
      document.addEventListener('keydown', unlockAudio, { once: true });
    }

    const keys = await getUserKeys();
    setUserKeys(keys);

    await reloadFriends();
    await reloadRequests();

    // Connect WebSocket
    wsClient.connect();

    // Subscribe to real-time events
    const unsubscribe = wsClient.subscribe(async (event) => {
      if (event.type === 'message.new') {
        const msg = event.data;
        const keys = userKeys();
        if (!keys) return;

        const isFromMe = msg.sender_id === props.currentUserId;
        const partnerId = isFromMe ? msg.recipient_id : msg.sender_id;

        // Ensure partner is in friends list
        if (!friends().some((f) => f.id === partnerId)) {
          await reloadFriends();
        }

        // Bump partner to top of conversation list
        setFriends((prev) => {
          const idx = prev.findIndex((f) => f.id === partnerId);
          if (idx > 0) {
            const f = prev[idx];
            return [f, ...prev.slice(0, idx), ...prev.slice(idx + 1)];
          }
          return prev;
        });

        // Determine if user is currently looking at this active conversation
        const active = activeFriend();
        // Message belongs to active conversation if activeFriend matches partner,
        // regardless of which sidebar tab is open (messages are always appended)
        const isForActiveFriend = active && active.id === partnerId;

        try {
          // Fetch or get sender peer keys for signature verification
          let senderPeerKeys = peerKeysCache.get(msg.sender_id);
          if (!senderPeerKeys) {
            const pkResp = await api.keys.get(msg.sender_id);
            senderPeerKeys = await getOrVerifyPeerKeys(
              pkResp.user_id,
              pkResp.ecdh_public,
              pkResp.ecdsa_public,
              pkResp.fingerprint
            );
            peerKeysCache.set(msg.sender_id, senderPeerKeys);
          }

          const decrypted = await decryptChatMessage({
            envelopeB64: msg.envelope,
            myUserId: props.currentUserId,
            myKeys: keys,
            senderEcdsaKey: senderPeerKeys.ecdsaKey,
          });

          // Always update preview for this partner
          updatePreview(partnerId, decrypted.text);

          if (isForActiveFriend) {
            // Append message to active conversation
            setMessages((prev) => {
              if (prev.some((m) => m.id === msg.id)) return prev;
              return [
                ...prev,
                {
                  id: msg.id,
                  seq: msg.seq,
                  senderId: msg.sender_id,
                  recipientId: msg.recipient_id,
                  text: decrypted.text,
                  createdAt: msg.created_at,
                  isSelf: isFromMe,
                  status: 'sent',
                },
              ];
            });
            scrollToBottom();
            // Only count as unread if not actively viewing chat window
            if (!isFromMe && (sidebarTab() !== 'chats' || isSettingsOpen())) {
              setUnreadCounts((prev) => ({
                ...prev,
                [partnerId]: (prev[partnerId] || 0) + 1,
              }));
              playNotificationChime();
            }
          } else {
            // Incoming from a different partner: mark unread and chime
            if (!isFromMe) {
              setUnreadCounts((prev) => ({
                ...prev,
                [partnerId]: (prev[partnerId] || 0) + 1,
              }));
              playNotificationChime();
            }
          }
        } catch {
          // Decryption failed or signature invalid
          if (isForActiveFriend) {
            setMessages((prev) => [
              ...prev,
              {
                id: msg.id,
                seq: msg.seq,
                senderId: msg.sender_id,
                recipientId: msg.recipient_id,
                text: 'Không thể giải mã tin nhắn',
                createdAt: msg.created_at,
                isSelf: false,
                status: 'error',
              },
            ]);
          }
        }
      } else if (event.type === 'friend.request') {
        await reloadRequests();
        playNotificationChime();
      } else if (event.type === 'friend.accepted') {
        await reloadFriends();
        await reloadRequests();
      } else if (event.type === 'typing.start') {
        const active = activeFriend();
        if (active && event.data.user_id === active.id) {
          setIsFriendTyping(true);
        }
      } else if (event.type === 'typing.stop') {
        const active = activeFriend();
        if (active && event.data.user_id === active.id) {
          setIsFriendTyping(false);
        }
      }
    });

    onCleanup(() => {
      if (typeof window !== 'undefined') {
        window.removeEventListener('resize', handleWindowResize);
        document.removeEventListener('click', unlockAudio);
        document.removeEventListener('keydown', unlockAudio);
      }
      unsubscribe();
    });
  });

  const handleAvatarFileSelect = async (e: Event) => {
    const target = e.target as HTMLInputElement;
    if (!target.files || target.files.length === 0) return;
    const file = target.files[0];
    setIsUploadingAvatar(true);
    setAvatarUploadError(null);
    try {
      const avifBlob = await processAvatarImage(file);
      const res = await api.users.uploadAvatar(avifBlob);
      setMyAvatarUrl(res.avatar_url);
      if (props.onAvatarUpdated) {
        props.onAvatarUpdated(res.avatar_url);
      }
    } catch (err: any) {
      console.error('Lỗi cập nhật ảnh đại diện:', err);
      setAvatarUploadError('Không thể cập nhật ảnh đại diện');
    } finally {
      setIsUploadingAvatar(false);
      target.value = '';
    }
  };

  // Scroll to bottom of chat (column-reverse: scrollTop = 0 is the bottom)
  const scrollToBottom = (behavior: ScrollBehavior = 'smooth') => {
    if (messagesContainerRef) {
      messagesContainerRef.scrollTo({ top: 0, behavior });
    }
  };

  const reloadFriends = async () => {
    try {
      const list = await api.friends.getFriends();
      setFriends(list);
    } catch {
      // ignore
    }
  };

  const reloadRequests = async () => {
    try {
      const list = await api.friends.getRequests();
      setPendingRequests(list);
    } catch {
      // ignore
    }
  };

  // Open a conversation with a friend
  const selectFriend = async (friend: FriendItem) => {
    setIsSettingsOpen(false);
    setActiveFriend(friend);
    setIsFriendTyping(false);
    setMessages([]);
    setPartnerFingerprint(null);

    // Clear unread count for this friend
    setUnreadCounts((prev) => {
      if (!prev[friend.id]) return prev;
      const next = { ...prev };
      delete next[friend.id];
      return next;
    });

    // Auto-open on large screen if closed
    if (typeof window !== 'undefined' && window.innerWidth >= 1024) {
      setIsPartnerInfoOpen(true);
    }

    const keys = userKeys();
    if (!keys) return;

    try {
      // 1. Fetch friend public keys
      let peerKeys = peerKeysCache.get(friend.id);
      if (!peerKeys) {
        const pkResp = await api.keys.get(friend.id);
        setPartnerFingerprint(pkResp.fingerprint);
        peerKeys = await getOrVerifyPeerKeys(
          pkResp.user_id,
          pkResp.ecdh_public,
          pkResp.ecdsa_public,
          pkResp.fingerprint
        );
        peerKeysCache.set(friend.id, peerKeys);
      } else {
        setPartnerFingerprint(base64UrlEncode(peerKeys.fingerprint));
      }

      // 2. Fetch history from backend
      const wireList = await api.messages.get(friend.id);
      // Sort oldest to newest
      const sorted = wireList.sort((a, b) => a.seq - b.seq);

      const decryptedList: DisplayMessage[] = [];
      for (const wire of sorted) {
        try {
          const senderEcdsa =
            wire.sender_id === props.currentUserId
              ? keys.ecdsaKeyPair.publicKey
              : peerKeys.ecdsaKey;

          const dec = await decryptChatMessage({
            envelopeB64: wire.envelope,
            myUserId: props.currentUserId,
            myKeys: keys,
            senderEcdsaKey: senderEcdsa,
          });

          decryptedList.push({
            id: wire.id,
            seq: wire.seq,
            senderId: wire.sender_id,
            recipientId: wire.recipient_id,
            text: dec.text,
            createdAt: wire.created_at,
            isSelf: wire.sender_id === props.currentUserId,
            status: 'sent',
          });
        } catch {
          decryptedList.push({
            id: wire.id,
            seq: wire.seq,
            senderId: wire.sender_id,
            recipientId: wire.recipient_id,
            text: 'Không thể giải mã tin nhắn',
            createdAt: wire.created_at,
            isSelf: wire.sender_id === props.currentUserId,
            status: 'error',
          });
        }
      }

      setMessages(decryptedList);
      if (decryptedList.length > 0) {
        const last = decryptedList[decryptedList.length - 1];
        updatePreview(friend.id, last.text);
      }
      // Instant jump to bottom on first load (no animation)
      scrollToBottom('instant');
    } catch (err: any) {
      // Failed to load
    }
  };

  // Search users live as user types
  createEffect(async () => {
    const q = friendInput().trim();
    if (q.length === 0) {
      setSearchResults([]);
      return;
    }
    try {
      const res = await api.users.search(q);
      setSearchResults(res);
    } catch {
      setSearchResults([]);
    }
  });

  const handleSendFriendRequest = async (receiverId: string) => {
    try {
      await api.friends.sendRequest({ receiver_id: receiverId });
      setSearchResults((prev) =>
        prev.map((u) => (u.id === receiverId ? { ...u, relationship: 'pending_sent' } : u))
      );
      await reloadFriends();
      await reloadRequests();
    } catch {
      // ignore
    }
  };

  const handleAddFriendSubmit = async (e?: Event) => {
    if (e) e.preventDefault();
    const target = friendInput().trim();
    if (!target || isAddingFriend()) return;

    if (target.toLowerCase() === props.currentUsername.toLowerCase()) {
      setAddFriendStatus({ type: 'error', message: 'Không thể tự kết bạn' });
      return;
    }

    setIsAddingFriend(true);
    setAddFriendStatus(null);
    try {
      await api.friends.sendRequest(target);
      setFriendInput('');
      setAddFriendStatus({ type: 'success', message: 'Đã gửi lời mời kết bạn' });
      await reloadFriends();
      await reloadRequests();
    } catch (err: any) {
      if (err.message === 'receiver_not_found') {
        setAddFriendStatus({ type: 'error', message: 'Không tìm thấy người dùng' });
      } else if (err.message === 'already_friends') {
        setAddFriendStatus({ type: 'error', message: 'Hai người đã là bạn bè' });
      } else if (err.message === 'request_already_pending') {
        setAddFriendStatus({ type: 'error', message: 'Đã gửi lời mời trước đó' });
      } else {
        setAddFriendStatus({ type: 'error', message: 'Không thể gửi lời mời' });
      }
    } finally {
      setIsAddingFriend(false);
    }
  };

  const handleAcceptFriendRequest = async (reqId: string) => {
    try {
      await api.friends.acceptRequest(reqId);
      await reloadFriends();
      await reloadRequests();
    } catch {
      // ignore
    }
  };

  // Typing event handling
  const handleInputChange = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    const val = e.currentTarget.value;
    setInputText(val);

    const active = activeFriend();
    if (!active) return;

    if (!isCurrentlyTyping && val.trim().length > 0) {
      isCurrentlyTyping = true;
      api.typing.send(active.id, true).catch(() => {});
    }

    if (typingTimeout) clearTimeout(typingTimeout);
    typingTimeout = setTimeout(() => {
      isCurrentlyTyping = false;
      api.typing.send(active.id, false).catch(() => {});
    }, 2500);
  };

  // Send message
  const handleSendMessage = async (e?: Event) => {
    if (e) e.preventDefault();
    const active = activeFriend();
    const text = inputText().trim();
    const keys = userKeys();

    if (!active || !text || !keys || isSending()) return;

    if (typingTimeout) {
      clearTimeout(typingTimeout);
      isCurrentlyTyping = false;
      api.typing.send(active.id, false).catch(() => {});
    }

    setIsSending(true);
    setInputText('');
    setIsEmojiOpen(false);

    const messageId = crypto.randomUUID();

    try {
      // Ensure recipient keys are ready
      let peerKeys = peerKeysCache.get(active.id);
      if (!peerKeys) {
        const pkResp = await api.keys.get(active.id);
        peerKeys = await getOrVerifyPeerKeys(
          pkResp.user_id,
          pkResp.ecdh_public,
          pkResp.ecdsa_public,
          pkResp.fingerprint
        );
        peerKeysCache.set(active.id, peerKeys);
      }

      // 1. E2EE Encrypt on device
      const { recipientEnvelopeB64, senderEnvelopeB64 } = await encryptChatMessage({
        messageId,
        senderId: props.currentUserId,
        recipientId: active.id,
        senderKeys: keys,
        recipientKeys: peerKeys,
        text,
      });

      // 2. Send via HTTP API
      const resp = await api.messages.send({
        message_id: messageId,
        recipient_id: active.id,
        recipient_envelope: recipientEnvelopeB64,
        sender_envelope: senderEnvelopeB64,
      });

      // 3. Add to UI
      setMessages((prev) => [
        ...prev,
        {
          id: messageId,
          seq: resp.seq,
          senderId: props.currentUserId,
          recipientId: active.id,
          text,
          createdAt: resp.created_at,
          isSelf: true,
          status: 'sent',
        },
      ]);

      updatePreview(active.id, text);
      scrollToBottom();
    } catch (err: any) {
      // On failure, display error message
      setMessages((prev) => [
        ...prev,
        {
          id: messageId,
          seq: 0,
          senderId: props.currentUserId,
          recipientId: active.id,
          text: 'Không gửi được',
          createdAt: new Date().toISOString(),
          isSelf: true,
          status: 'error',
        },
      ]);
    } finally {
      setIsSending(false);
      inputRef?.focus();
    }
  };

  const handleEmojiSelect = (emoji: string) => {
    if (!inputRef) {
      setInputText((prev) => prev + emoji);
      return;
    }
    const start = inputRef.selectionStart || inputText().length;
    const end = inputRef.selectionEnd || inputText().length;
    const current = inputText();
    const updated = current.substring(0, start) + emoji + current.substring(end);
    setInputText(updated);

    setTimeout(() => {
      if (inputRef) {
        inputRef.selectionStart = inputRef.selectionEnd = start + emoji.length;
        inputRef.focus();
      }
    }, 0);
  };

  const formatTime = (isoString: string) => {
    try {
      const d = new Date(isoString);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch {
      return '';
    }
  };

  return (
    <div class="app-layout">
      {/* Sidebar */}
      <aside class={`sidebar ${activeFriend() ? 'hidden-mobile' : ''}`}>
        <div class="sidebar-header">
          <div class="app-brand">
            <Avatar name={props.currentUsername} avatarUrl={myAvatarUrl()} size={28} />
            <span>{props.currentUsername}</span>
          </div>
          <div class="sidebar-actions">
            <button
              type="button"
              class={`icon-btn ${sidebarTab() === 'chats' && !isSettingsOpen() ? 'active' : ''}`}
              onClick={() => {
                setSidebarTab('chats');
                setIsSettingsOpen(false);
                const active = activeFriend();
                if (active) {
                  setUnreadCounts((prev) => {
                    if (!prev[active.id]) return prev;
                    const next = { ...prev };
                    delete next[active.id];
                    return next;
                  });
                }
              }}
              aria-label="Danh sách chat"
            >
              <LogoIcon size={18} />
              <Show when={totalUnreadCount() > 0}>
                <span class="tab-badge">{totalUnreadCount() > 99 ? '99+' : totalUnreadCount()}</span>
              </Show>
            </button>
            <button
              type="button"
              class={`icon-btn ${sidebarTab() === 'friends' && !isSettingsOpen() ? 'active' : ''}`}
              onClick={() => {
                setSidebarTab('friends');
                setIsSettingsOpen(false);
              }}
              aria-label="Kết bạn"
            >
              <FriendsIcon />
              <Show when={pendingRequests().length > 0}>
                <span class="tab-badge">{pendingRequests().length > 99 ? '99+' : pendingRequests().length}</span>
              </Show>
            </button>
            <button
              type="button"
              class={`icon-btn ${isSettingsOpen() ? 'active' : ''}`}
              onClick={() => {
                const nextOpen = !isSettingsOpen();
                setIsSettingsOpen(nextOpen);
                if (nextOpen) {
                  setActiveFriend(null);
                }
              }}
              aria-label="Cài đặt"
            >
              <SettingsIcon />
            </button>
            <button
              type="button"
              class="icon-btn btn-danger"
              onClick={props.onLogout}
              aria-label="Đăng xuất"
            >
              <LogoutIcon />
            </button>
          </div>
        </div>

        {/* Tab: Friends / Direct Add & Requests */}
        <Show when={sidebarTab() === 'friends'}>
          <div class="add-friend-container">
            <form onSubmit={handleAddFriendSubmit} class="add-friend-form">
              <input
                type="text"
                placeholder="Tên tài khoản"
                value={friendInput()}
                onInput={(e) => {
                  setFriendInput(e.currentTarget.value);
                  setAddFriendStatus(null);
                }}
                aria-label="Tên tài khoản kết bạn"
              />
              <button
                type="submit"
                class="btn-small-primary"
                disabled={!friendInput().trim() || isAddingFriend()}
              >
                Kết bạn
              </button>
            </form>
            <Show when={addFriendStatus()}>
              <div class={`status-badge ${addFriendStatus()!.type}`}>
                {addFriendStatus()!.message}
              </div>
            </Show>
          </div>
          <div class="sidebar-scrollable">
            <Show
              when={friendInput().trim().length > 0}
              fallback={
                <For
                  each={pendingRequests()}
                  fallback={
                    <Show when={!addFriendStatus()}>
                      <div class="empty-chat">Không có lời mời nào</div>
                    </Show>
                  }
                >
                  {(req) => (
                    <div class="request-card">
                      <div class="request-user">
                        <Avatar name={req.sender_username} avatarUrl={req.sender_avatar_url} size={32} />
                        <span class="chat-item-name">{req.sender_username}</span>
                      </div>
                      <button
                        type="button"
                        class="btn-small-primary"
                        onClick={() => handleAcceptFriendRequest(req.id)}
                      >
                        Chấp nhận
                      </button>
                    </div>
                  )}
                </For>
              }
            >
              <For
                each={searchResults()}
                fallback={<div class="empty-chat">Không tìm thấy người dùng</div>}
              >
                {(user) => (
                  <div class="request-card">
                    <div class="request-user">
                      <Avatar name={user.username} avatarUrl={user.avatar_url} size={32} />
                      <span class="chat-item-name">{user.username}</span>
                    </div>
                    <Show when={user.relationship === 'friend'}>
                      <span class="btn-small-disabled">Bạn bè</span>
                    </Show>
                    <Show when={user.relationship === 'pending_sent'}>
                      <span class="btn-small-disabled">Đã gửi</span>
                    </Show>
                    <Show when={user.relationship === 'pending_received'}>
                      <button
                        type="button"
                        class="btn-small-primary"
                        onClick={() => {
                          const req = pendingRequests().find((r) => r.sender_id === user.id);
                          if (req) handleAcceptFriendRequest(req.id);
                        }}
                      >
                        Chấp nhận
                      </button>
                    </Show>
                    <Show when={user.relationship === 'none'}>
                      <button
                        type="button"
                        class="btn-small-primary"
                        onClick={() => handleSendFriendRequest(user.id)}
                      >
                        Kết bạn
                      </button>
                    </Show>
                  </div>
                )}
              </For>
            </Show>
          </div>
        </Show>

        {/* Tab: Chats (Default) */}
        <Show when={sidebarTab() === 'chats'}>
          <div class="sidebar-scrollable">
            <For
              each={friends()}
              fallback={<div class="empty-chat">Chưa có bạn bè nào</div>}
            >
              {(friend) => {
                const unread = () => unreadCounts()[friend.id] || 0;
                return (
                  <button
                    type="button"
                    class={`chat-item ${activeFriend()?.id === friend.id ? 'selected' : ''}`}
                    onClick={() => selectFriend(friend)}
                  >
                    <Avatar name={friend.username} avatarUrl={friend.avatar_url} size={38} />
                    <div class="chat-item-info">
                      <div class="chat-item-header">
                        <span class={`chat-item-name ${unread() > 0 ? 'unread' : ''}`}>
                          {friend.username}
                        </span>
                        <Show when={unread() > 0}>
                          <span class="chat-item-badge">
                            {unread() > 99 ? '99+' : unread()}
                          </span>
                        </Show>
                      </div>
                      <div class={`chat-item-preview ${unread() > 0 ? 'unread' : ''}`}>
                        {lastPreviews()[friend.id] || 'Bắt đầu trò chuyện'}
                      </div>
                    </div>
                  </button>
                );
              }}
            </For>
          </div>
        </Show>
      </aside>

      {/* Main Chat View */}
      <main class={`chat-view ${!activeFriend() && !isSettingsOpen() ? 'hidden-mobile' : ''}`}>
        <Show
          when={activeFriend()}
          fallback={
            <Show
              when={isSettingsOpen()}
              fallback={
                <div class="empty-chat">
                  <span>Chọn một cuộc trò chuyện để bắt đầu</span>
                </div>
              }
            >
              {/* Settings View */}
              <header class="chat-header">
                <button
                  type="button"
                  class="icon-btn"
                  onClick={() => setIsSettingsOpen(false)}
                  aria-label="Đóng cài đặt"
                >
                  <BackIcon />
                </button>
                <h2 class="chat-header-title">Cài đặt</h2>
              </header>
              <div class="settings-content">
                <div class="settings-profile-card">
                  <div class="settings-avatar-wrapper">
                    <Avatar name={props.currentUsername} avatarUrl={myAvatarUrl()} size={56} />
                  </div>
                  <div class="settings-profile-info">
                    <div class="settings-profile-name">{props.currentUsername}</div>
                    <div class="settings-profile-sub">Tài khoản cá nhân</div>
                  </div>
                  <div class="settings-profile-action">
                    <label class="btn-avatar-upload">
                      <input
                        type="file"
                        accept="image/*"
                        onChange={handleAvatarFileSelect}
                        disabled={isUploadingAvatar()}
                        style="display: none;"
                      />
                      {isUploadingAvatar() ? 'Đang tải' : 'Đổi ảnh đại diện'}
                    </label>
                    <Show when={avatarUploadError()}>
                      <span class="error-text">{avatarUploadError()}</span>
                    </Show>
                  </div>
                </div>

                <div class="settings-group">
                  <div class="settings-group-title">Tài khoản</div>
                  <div class="settings-row">
                    <span class="settings-label">Tên người dùng</span>
                    <span class="settings-value">{props.currentUsername}</span>
                  </div>
                  <div class="settings-row column-layout">
                    <span class="settings-label">Mã định danh</span>
                    <span class="settings-value mono">{props.currentUserId}</span>
                  </div>
                </div>

                <div class="settings-group">
                  <div class="settings-group-title">Mã hóa đầu cuối</div>
                  <div class="settings-row">
                    <span class="settings-label">Giao thức</span>
                    <span class="settings-value nowrap">ECDH P-256 / AES-256-GCM</span>
                  </div>
                  <div class="settings-row">
                    <span class="settings-label">Chữ ký số</span>
                    <span class="settings-value nowrap">ECDSA P-256 / SHA-256</span>
                  </div>
                  <div class="settings-row">
                    <span class="settings-label">Trạng thái khóa</span>
                    <span class="settings-value tag-active">
                      {userKeys() ? 'Đã kích hoạt trên thiết bị' : 'Đang tải'}
                    </span>
                  </div>
                  <Show when={userKeys()?.fingerprintB64}>
                    <div class="settings-row column-layout">
                      <span class="settings-label">Vân tay khóa</span>
                      <span class="settings-value mono">{userKeys()!.fingerprintB64}</span>
                    </div>
                  </Show>
                </div>

                <div class="settings-group">
                  <div class="settings-group-title">Phiên đăng nhập</div>
                  <button
                    type="button"
                    class="btn-danger-outline"
                    onClick={props.onLogout}
                  >
                    Đăng xuất
                  </button>
                </div>
              </div>
            </Show>
          }
        >
          <div class="chat-main-container">
            <div class="chat-column">
              {/* Header */}
              <header class="chat-header">
                <button
                  type="button"
                  class="icon-btn back-btn"
                  onClick={() => setActiveFriend(null)}
                  aria-label="Quay lại"
                >
                  <BackIcon />
                </button>
                <Avatar name={activeFriend()?.username || ''} avatarUrl={activeFriend()?.avatar_url} size={34} />
                <h2 class="chat-header-title">{activeFriend()?.username}</h2>
                <div class="chat-header-spacer" />
                <button
                  type="button"
                  class={`icon-btn ${isPartnerInfoOpen() ? 'active' : ''}`}
                  onClick={() => setIsPartnerInfoOpen(!isPartnerInfoOpen())}
                  aria-label="Thông tin tài khoản"
                  title="Thông tin tài khoản"
                >
                  <MenuIcon />
                </button>
              </header>

              {/* Messages — column-reverse: newest at bottom, scroll up for older */}
              <div class="chat-messages" ref={messagesContainerRef}>
                <For
                  each={[...messages()].reverse()}
                  fallback={<div class="empty-chat-inner">Chưa có tin nhắn</div>}
                >
                  {(msg) => (
                    <div class={`message-row ${msg.isSelf ? 'sent' : 'received'}`}>
                      <div class="message-content-wrapper">
                        <Show when={!msg.isSelf}>
                          <Avatar name={activeFriend()?.username || ''} avatarUrl={activeFriend()?.avatar_url} size={28} />
                        </Show>
                        <div class="message-bubble">{msg.text}</div>
                      </div>
                      <Show when={msg.createdAt}>
                        <span class="message-time">{formatTime(msg.createdAt)}</span>
                      </Show>
                    </div>
                  )}
                </For>
              </div>

              {/* Typing Indicator */}
              <Show when={isFriendTyping()}>
                <div class="typing-indicator">Đang nhập</div>
              </Show>

              {/* Input Bar */}
              <form class="chat-input-bar" onSubmit={handleSendMessage}>
                <Show when={isEmojiOpen()}>
                  <EmojiPicker
                    onSelect={handleEmojiSelect}
                    onClose={() => setIsEmojiOpen(false)}
                  />
                </Show>
                <button
                  type="button"
                  class="icon-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    setIsEmojiOpen(!isEmojiOpen());
                  }}
                  aria-label="Chọn emoji"
                >
                  <EmojiIcon />
                </button>
                <input
                  ref={inputRef}
                  type="text"
                  class="chat-input"
                  placeholder="Nhập tin nhắn"
                  value={inputText()}
                  onInput={handleInputChange}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey) {
                      e.preventDefault();
                      handleSendMessage();
                    } else if (e.key === 'Escape') {
                      setIsEmojiOpen(false);
                    }
                  }}
                  aria-label="Nội dung tin nhắn"
                />
                <button
                  type="submit"
                  class="send-btn"
                  disabled={isSending() || !inputText().trim()}
                  aria-label="Gửi"
                >
                  <SendIcon />
                </button>
              </form>
            </div>

            {/* Partner Info Panel */}
            <Show when={isPartnerInfoOpen()}>
              <aside class="partner-info-sidebar">
                <header class="partner-info-header">
                  <h3 class="partner-info-title">Thông tin</h3>
                  <button
                    type="button"
                    class="icon-btn"
                    onClick={() => setIsPartnerInfoOpen(false)}
                    aria-label="Đóng"
                  >
                    <CloseIcon />
                  </button>
                </header>

                <div class="partner-info-content">
                  <div class="partner-info-card">
                    <Avatar name={activeFriend()?.username || ''} avatarUrl={activeFriend()?.avatar_url} size={64} />
                    <div class="partner-info-name">{activeFriend()?.username}</div>
                    <div class="partner-info-sub">Tài khoản bạn bè</div>
                  </div>

                  <div class="settings-group">
                    <div class="settings-group-title">Tài khoản</div>
                    <div class="settings-row">
                      <span class="settings-label">Tên người dùng</span>
                      <span class="settings-value">{activeFriend()?.username}</span>
                    </div>
                    <div class="settings-row column-layout">
                      <span class="settings-label">Mã định danh</span>
                      <span class="settings-value mono">{activeFriend()?.id}</span>
                    </div>
                  </div>

                  <div class="settings-group">
                    <div class="settings-group-title">Mã hóa đầu cuối</div>
                    <div class="settings-row">
                      <span class="settings-label">Giao thức</span>
                      <span class="settings-value nowrap">ECDH P-256 / AES-256-GCM</span>
                    </div>
                    <div class="settings-row">
                      <span class="settings-label">Chữ ký số</span>
                      <span class="settings-value nowrap">ECDSA P-256 / SHA-256</span>
                    </div>
                    <div class="settings-row">
                      <span class="settings-label">Trạng thái khóa</span>
                      <span class="settings-value tag-active">
                        {partnerFingerprint() ? 'Đã kích hoạt trên thiết bị' : 'Đang tải'}
                      </span>
                    </div>
                    <Show when={partnerFingerprint()}>
                      <div class="settings-row column-layout">
                        <span class="settings-label">Vân tay khóa</span>
                        <span class="settings-value mono">{partnerFingerprint()}</span>
                      </div>
                    </Show>
                  </div>
                </div>
              </aside>
            </Show>
          </div>
        </Show>
      </main>
    </div>
  );
}
