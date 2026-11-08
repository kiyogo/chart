// End-to-End Encryption protocol implementation for chatw
// Web Crypto API (P-256 ECDH, P-256 ECDSA, HKDF-SHA-256, AES-256-GCM) + IndexedDB

const DB_NAME = 'chatw_e2ee';
const DB_VERSION = 1;
const PROTOCOL_VERSION = 1;

function toBufferSource(arr: Uint8Array): BufferSource {
  return arr as unknown as BufferSource;
}

export interface UserKeys {
  ecdhKeyPair: CryptoKeyPair;
  ecdsaKeyPair: CryptoKeyPair;
  ecdhRaw: Uint8Array;
  ecdsaRaw: Uint8Array;
  fingerprint: Uint8Array;
  ecdhRawB64: string;
  ecdsaRawB64: string;
  fingerprintB64: string;
}

export interface PeerKeys {
  ecdhRaw: Uint8Array;
  ecdsaRaw: Uint8Array;
  fingerprint: Uint8Array;
  ecdhKey: CryptoKey;
  ecdsaKey: CryptoKey;
}

// Base64URL Encoding & Decoding (no padding)
export function base64UrlEncode(buffer: ArrayBuffer | Uint8Array): string {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

export function base64UrlDecode(str: string): Uint8Array {
  let base64 = str.replace(/-/g, '+').replace(/_/g, '/');
  while (base64.length % 4 !== 0) {
    base64 += '=';
  }
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

// Convert standard UUID string (8-4-4-4-12) to 16 bytes
export function uuidToBytes(uuid: string): Uint8Array {
  const hex = uuid.replace(/-/g, '');
  if (hex.length !== 32) {
    throw new Error('Invalid UUID format');
  }
  const bytes = new Uint8Array(16);
  for (let i = 0; i < 16; i++) {
    bytes[i] = parseInt(hex.substring(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

// Convert 16 bytes back to standard UUID string
export function bytesToUuid(bytes: Uint8Array): string {
  let hex = '';
  for (let i = 0; i < 16; i++) {
    hex += bytes[i].toString(16).padStart(2, '0');
  }
  return [
    hex.substring(0, 8),
    hex.substring(8, 12),
    hex.substring(12, 16),
    hex.substring(16, 20),
    hex.substring(20, 32),
  ].join('-');
}

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains('keys')) {
        db.createObjectStore('keys');
      }
      if (!db.objectStoreNames.contains('peers')) {
        db.createObjectStore('peers');
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function getUserKeys(): Promise<UserKeys | null> {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keys', 'readonly');
    const store = tx.objectStore('keys');
    const req = store.get('current_keys');
    req.onsuccess = () => {
      const val = req.result;
      if (!val) {
        resolve(null);
        return;
      }
      resolve({
        ecdhKeyPair: val.ecdhKeyPair,
        ecdsaKeyPair: val.ecdsaKeyPair,
        ecdhRaw: val.ecdhRaw,
        ecdsaRaw: val.ecdsaRaw,
        fingerprint: val.fingerprint,
        ecdhRawB64: base64UrlEncode(val.ecdhRaw),
        ecdsaRawB64: base64UrlEncode(val.ecdsaRaw),
        fingerprintB64: base64UrlEncode(val.fingerprint),
      });
    };
    req.onerror = () => reject(req.error);
  });
}

export async function createAndStoreUserKeys(): Promise<UserKeys> {
  // 1. Generate P-256 ECDH KeyPair
  const ecdhKeyPair = await crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    false,
    ['deriveKey', 'deriveBits']
  );

  // 2. Generate P-256 ECDSA KeyPair
  const ecdsaKeyPair = await crypto.subtle.generateKey(
    { name: 'ECDSA', namedCurve: 'P-256' },
    false,
    ['sign', 'verify']
  );

  // 3. Export raw uncompressed public keys (65 bytes each: 0x04 || X || Y)
  const ecdhRaw = new Uint8Array(await crypto.subtle.exportKey('raw', ecdhKeyPair.publicKey));
  const ecdsaRaw = new Uint8Array(await crypto.subtle.exportKey('raw', ecdsaKeyPair.publicKey));

  // 4. Compute fingerprint = SHA-256(ecdhRaw) (32 bytes)
  const fpBuffer = await crypto.subtle.digest('SHA-256', ecdhRaw);
  const fingerprint = new Uint8Array(fpBuffer);

  const keysRecord = {
    ecdhKeyPair,
    ecdsaKeyPair,
    ecdhRaw,
    ecdsaRaw,
    fingerprint,
  };

  // 5. Save in IndexedDB
  const db = await openDatabase();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction('keys', 'readwrite');
    tx.objectStore('keys').put(keysRecord, 'current_keys');
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });

  return {
    ecdhKeyPair,
    ecdsaKeyPair,
    ecdhRaw,
    ecdsaRaw,
    fingerprint,
    ecdhRawB64: base64UrlEncode(ecdhRaw),
    ecdsaRawB64: base64UrlEncode(ecdsaRaw),
    fingerprintB64: base64UrlEncode(fingerprint),
  };
}

export async function getOrVerifyPeerKeys(
  peerId: string,
  ecdhRawB64: string,
  ecdsaRawB64: string,
  fingerprintB64: string
): Promise<PeerKeys> {
  const ecdhRaw = base64UrlDecode(ecdhRawB64);
  const ecdsaRaw = base64UrlDecode(ecdsaRawB64);
  const fingerprint = base64UrlDecode(fingerprintB64);

  // Validate fingerprint locally
  const computedFp = new Uint8Array(await crypto.subtle.digest('SHA-256', toBufferSource(ecdhRaw)));
  for (let i = 0; i < 32; i++) {
    if (computedFp[i] !== fingerprint[i]) {
      throw new Error('Invalid peer key fingerprint');
    }
  }

  const db = await openDatabase();
  const existing = await new Promise<any>((resolve, reject) => {
    const tx = db.transaction('peers', 'readonly');
    const req = tx.objectStore('peers').get(peerId);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });

  if (existing) {
    // Trust On First Use (TOFU) check:
    const storedFp = existing.fingerprint as Uint8Array;
    let match = true;
    for (let i = 0; i < 32; i++) {
      if (storedFp[i] !== fingerprint[i]) {
        match = false;
        break;
      }
    }
    if (!match) {
      throw new Error('Chìa khóa bảo mật của người này đã bị thay đổi bất thường!');
    }
  } else {
    // Save to IndexedDB (First use)
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction('peers', 'readwrite');
      tx.objectStore('peers').put({ ecdhRaw, ecdsaRaw, fingerprint }, peerId);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
  }

  const ecdhKey = await crypto.subtle.importKey(
    'raw',
    toBufferSource(ecdhRaw),
    { name: 'ECDH', namedCurve: 'P-256' },
    false,
    []
  );

  const ecdsaKey = await crypto.subtle.importKey(
    'raw',
    toBufferSource(ecdsaRaw),
    { name: 'ECDSA', namedCurve: 'P-256' },
    false,
    ['verify']
  );

  return {
    ecdhRaw,
    ecdsaRaw,
    fingerprint,
    ecdhKey,
    ecdsaKey,
  };
}

async function buildEnvelope(params: {
  messageId: string;
  senderId: string;
  recipientId: string;
  recipientFingerprint: Uint8Array;
  recipientEcdhKey: CryptoKey;
  senderEcdsaPrivateKey: CryptoKey;
  plaintextBytes: Uint8Array;
}): Promise<Uint8Array> {
  const {
    messageId,
    senderId,
    recipientId,
    recipientFingerprint,
    recipientEcdhKey,
    senderEcdsaPrivateKey,
    plaintextBytes,
  } = params;

  // 1. Generate ephemeral ECDH keypair
  const ephemeralKeyPair = await crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    true,
    ['deriveKey', 'deriveBits']
  );
  const ephemeralPublicRaw = new Uint8Array(
    await crypto.subtle.exportKey('raw', ephemeralKeyPair.publicKey)
  );

  // 2. Perform ECDH to get shared bits
  const sharedBits = await crypto.subtle.deriveBits(
    { name: 'ECDH', public: recipientEcdhKey },
    ephemeralKeyPair.privateKey,
    256
  );

  // 3. Derive AES-GCM key with HKDF-SHA-256
  const hkdfKey = await crypto.subtle.importKey(
    'raw',
    sharedBits,
    { name: 'HKDF' },
    false,
    ['deriveKey']
  );

  const salt = crypto.getRandomValues(new Uint8Array(16));
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const info = new TextEncoder().encode('chatw-v1-msg:' + messageId);

  const aesKey = await crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt,
      info,
    },
    hkdfKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );

  // 4. Construct AAD (81 bytes)
  // [version (1)] + [msg_id (16)] + [sender_id (16)] + [recipient_id (16)] + [fingerprint (32)]
  const aad = new Uint8Array(81);
  aad[0] = PROTOCOL_VERSION;
  aad.set(uuidToBytes(messageId), 1);
  aad.set(uuidToBytes(senderId), 17);
  aad.set(uuidToBytes(recipientId), 33);
  aad.set(recipientFingerprint, 49);

  // 5. Encrypt with AES-GCM
  const ciphertextBuffer = await crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: toBufferSource(nonce),
      additionalData: toBufferSource(aad),
      tagLength: 128,
    },
    aesKey,
    toBufferSource(plaintextBytes)
  );
  const ciphertext = new Uint8Array(ciphertextBuffer);

  // 6. Assemble envelope body before signature (174 + ciphertext.length)
  const headerLen = 174;
  const bodyLen = headerLen + ciphertext.length;
  const envelopeWithSig = new Uint8Array(bodyLen + 64);

  envelopeWithSig.set(aad, 0); // 0..81
  envelopeWithSig.set(ephemeralPublicRaw, 81); // 81..146 (65 bytes)
  envelopeWithSig.set(salt, 146); // 146..162 (16 bytes)
  envelopeWithSig.set(nonce, 162); // 162..174 (12 bytes)
  envelopeWithSig.set(ciphertext, 174); // 174..bodyLen

  // 7. Sign envelope body using sender's ECDSA private key
  const bodySlice = envelopeWithSig.subarray(0, bodyLen);
  const sigBuffer = await crypto.subtle.sign(
    { name: 'ECDSA', hash: 'SHA-256' },
    senderEcdsaPrivateKey,
    toBufferSource(bodySlice)
  );

  // 8. Append 64-byte signature
  envelopeWithSig.set(new Uint8Array(sigBuffer), bodyLen);

  return envelopeWithSig;
}

export async function encryptChatMessage(params: {
  messageId: string;
  senderId: string;
  recipientId: string;
  senderKeys: UserKeys;
  recipientKeys: PeerKeys;
  text: string;
}): Promise<{ recipientEnvelopeB64: string; senderEnvelopeB64: string }> {
  const plaintextBytes = new TextEncoder().encode(params.text);

  // Envelope for recipient
  const recipientEnvelope = await buildEnvelope({
    messageId: params.messageId,
    senderId: params.senderId,
    recipientId: params.recipientId,
    recipientFingerprint: params.recipientKeys.fingerprint,
    recipientEcdhKey: params.recipientKeys.ecdhKey,
    senderEcdsaPrivateKey: params.senderKeys.ecdsaKeyPair.privateKey,
    plaintextBytes,
  });

  // Envelope for sender (self-envelope using sender's own public key)
  const senderEnvelope = await buildEnvelope({
    messageId: params.messageId,
    senderId: params.senderId,
    recipientId: params.senderId, // recipient is sender
    recipientFingerprint: params.senderKeys.fingerprint,
    recipientEcdhKey: params.senderKeys.ecdhKeyPair.publicKey,
    senderEcdsaPrivateKey: params.senderKeys.ecdsaKeyPair.privateKey,
    plaintextBytes,
  });

  return {
    recipientEnvelopeB64: base64UrlEncode(recipientEnvelope),
    senderEnvelopeB64: base64UrlEncode(senderEnvelope),
  };
}

export async function decryptChatMessage(params: {
  envelopeB64: string;
  myUserId: string;
  myKeys: UserKeys;
  senderEcdsaKey: CryptoKey;
}): Promise<{ messageId: string; senderId: string; recipientId: string; text: string }> {
  const raw = base64UrlDecode(params.envelopeB64);
  if (raw.length < 255) {
    throw new Error('Envelope too short');
  }

  const version = raw[0];
  if (version !== PROTOCOL_VERSION) {
    throw new Error('Unsupported protocol version');
  }

  const messageId = bytesToUuid(raw.subarray(1, 17));
  const senderId = bytesToUuid(raw.subarray(1, 17 + 16));
  const recipientId = bytesToUuid(raw.subarray(33, 49));
  const fingerprint = raw.subarray(49, 81);
  const ephemeralPublicRaw = raw.subarray(81, 146);
  const salt = raw.subarray(146, 162);
  const nonce = raw.subarray(162, 174);
  const sigStart = raw.length - 64;
  const ciphertext = raw.subarray(174, sigStart);
  const signature = raw.subarray(sigStart);

  // 1. Verify ECDSA signature over body
  const bodySlice = raw.subarray(0, sigStart);
  const isValid = await crypto.subtle.verify(
    { name: 'ECDSA', hash: 'SHA-256' },
    params.senderEcdsaKey,
    toBufferSource(signature),
    toBufferSource(bodySlice)
  );

  if (!isValid) {
    throw new Error('Signature verification failed');
  }

  // 2. Import ephemeral key
  const ephemeralKey = await crypto.subtle.importKey(
    'raw',
    toBufferSource(ephemeralPublicRaw),
    { name: 'ECDH', namedCurve: 'P-256' },
    false,
    []
  );

  // 3. Derive shared secret
  const sharedBits = await crypto.subtle.deriveBits(
    { name: 'ECDH', public: ephemeralKey },
    params.myKeys.ecdhKeyPair.privateKey,
    256
  );

  // 4. Derive AES key via HKDF
  const hkdfKey = await crypto.subtle.importKey(
    'raw',
    sharedBits,
    { name: 'HKDF' },
    false,
    ['deriveKey']
  );

  const info = new TextEncoder().encode('chatw-v1-msg:' + messageId);
  const aesKey = await crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: toBufferSource(salt),
      info,
    },
    hkdfKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['decrypt']
  );

  // 5. Decrypt AES-GCM
  const aad = raw.subarray(0, 81);
  const plaintextBuffer = await crypto.subtle.decrypt(
    {
      name: 'AES-GCM',
      iv: toBufferSource(nonce),
      additionalData: toBufferSource(aad),
      tagLength: 128,
    },
    aesKey,
    toBufferSource(ciphertext)
  );

  const text = new TextDecoder().decode(plaintextBuffer);
  return {
    messageId,
    senderId,
    recipientId,
    text,
  };
}
