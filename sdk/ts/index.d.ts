export type PluginRuntimeType = 'js' | 'wasm' | 'hybrid' | 'declarative';

export interface PluginPermission {
  name: string;
  reason?: string;
}

export interface PluginCommandSpec {
  trigger: string;
  description: string;
  usage?: string;
}

export interface PluginUiMountSpec {
  slot: 'page' | 'sidebar' | 'panel' | 'dialog' | 'composer_popover';
  title: string;
  icon?: string;
}

export interface PluginMessageActionSpec {
  id: string;
  label: string;
  icon?: string;
}

export interface PluginComposerButtonSpec {
  id: string;
  label: string;
  content?: string;
}

export interface PluginManifestV2 {
  plugin_key: string;
  name: string;
  version: string;
  publisher?: string;
  entry_point?: string;
  runtime: PluginRuntimeType;
  sdk_version?: string;
  platforms?: string[];
  permissions: string[];
  dependencies?: string[];
  ui_contributions?: Record<string, unknown>;
  commands?: PluginCommandSpec[];
  event_subscriptions?: string[];
  settings_schema?: Array<Record<string, unknown>>;
  resource_limits?: {
    max_memory_mb?: number;
    max_exec_ms?: number;
    max_storage_kb?: number;
    max_requests_per_min?: number;
  };
  allowed_domains?: string[];
  allow_custom_logo?: boolean;
}

export interface DiscordEmbedField {
  name: string;
  value: string;
  inline?: boolean;
}

export interface DiscordEmbed {
  title?: string;
  description?: string;
  url?: string;
  color?: number;
  timestamp?: string;
  author?: { name: string; url?: string; icon_url?: string };
  footer?: { text: string; icon_url?: string };
  thumbnail?: { url: string };
  image?: { url: string };
  fields?: DiscordEmbedField[];
}

export interface ThemeTokens {
  theme: 'light' | 'dark' | string;
  font: string;
  bg: string;
  surface: string;
  surfaceHover: string;
  input: string;
  text: string;
  textSecondary: string;
  accent: string;
  danger: string;
  radius: string;
}

export interface ChatwSdk {
  app: {
    info(): Promise<{ platform: string; sdkVersion: string }>;
    themeTokens(): Promise<ThemeTokens>;
  };
  account: {
    current(): Promise<{ id: string; username: string; displayName: string }>;
  };
  users: {
    current(): Promise<{ id: string; username: string; displayName: string }>;
    listByServer(serverId: string): Promise<unknown>;
  };
  messages: {
    readDecrypted(conversationId: string): Promise<Array<{ id: string; senderName: string; text: string; createdAt: string }>>;
    insertToComposer(text: string): Promise<{ ok: boolean }>;
    sendToChannel(channelId: string, content: string): Promise<unknown>;
    sendToGroup(groupId: string, content: string): Promise<unknown>;
  };
  conversations: {
    active(): Promise<unknown>;
    readDecrypted(conversationId: string): Promise<Array<{ id: string; senderName: string; text: string; createdAt: string }>>;
  };
  chats: { active(): Promise<unknown> };
  groups: { list(): Promise<unknown> };
  servers: { list(): Promise<unknown> };
  channels: { list(serverId: string): Promise<unknown> };
  members: { list(serverId: string): Promise<unknown> };
  calls: { status(): Promise<unknown> };
  voice: { status(): Promise<unknown> };
  video: { status(): Promise<unknown> };
  events: {
    on(eventName: string, handler: (payload: Record<string, unknown>, meta: Record<string, unknown>) => void | Promise<void>): Promise<unknown>;
    off(eventName: string, handler: (...args: unknown[]) => void): void;
  };
  ui: {
    notify(message: string, level?: 'info' | 'success' | 'warn' | 'error'): Promise<{ ok: boolean }>;
    openDialog(opts: { title: string; html: string }): Promise<{ ok: boolean }>;
    closeDialog(): Promise<{ ok: boolean }>;
    registerMessageAction(action: PluginMessageActionSpec): Promise<{ ok: boolean }>;
    registerComposerButton(btn: PluginComposerButtonSpec): Promise<{ ok: boolean }>;
    registerPage(page: { id?: string; title: string }): Promise<{ ok: boolean }>;
  };
  modals: {
    open(opts: { title: string; html: string }): Promise<{ ok: boolean }>;
    close(): Promise<{ ok: boolean }>;
  };
  menus: {
    registerMessageAction(action: PluginMessageActionSpec): Promise<{ ok: boolean }>;
  };
  sidebar: {
    registerPage(page: { id?: string; title: string }): Promise<{ ok: boolean }>;
  };
  toolbar: {
    registerComposerButton(btn: PluginComposerButtonSpec): Promise<{ ok: boolean }>;
  };
  commands: {
    register(
      trigger: string,
      description: string,
      handler: (ctx: { trigger: string; args: string; channelId?: string; groupId?: string }) => unknown | Promise<unknown>
    ): Promise<{ ok: boolean }>;
  };
  notifications: {
    send(title: string, body: string): Promise<{ ok: boolean }>;
  };
  storage: {
    get<T = unknown>(key: string): Promise<T | null>;
    set(key: string, value: unknown, opts?: { syncable?: boolean; is_secret?: boolean }): Promise<{ ok: boolean }>;
    delete(key: string): Promise<{ ok: boolean }>;
    list(): Promise<Record<string, unknown>>;
    setSecret(key: string, value: unknown): Promise<{ ok: boolean }>;
  };
  network: {
    fetch(
      url: string,
      opts?: {
        method?: string;
        headers?: Record<string, string>;
        body?: string;
        inject_secret_key?: string;
        inject_secret_header?: string;
        inject_secret_prefix?: string;
      }
    ): Promise<{ status: number; headers: Record<string, string>; body: string }>;
  };
  webhooks: {
    list(scopeType: string, scopeId: string): Promise<unknown>;
    create(payload: Record<string, unknown>): Promise<unknown>;
  };
  bots: {
    list(): Promise<unknown>;
  };
  settings: {
    get<T = Record<string, unknown>>(): Promise<T>;
    set(patch: Record<string, unknown>): Promise<Record<string, unknown>>;
  };
  permissions: {
    list(): Promise<Array<{ permission: string; scope_type: string; scope_id?: string; device_only: boolean }>>;
    has(permission: string, scopeId?: string): Promise<boolean>;
  };
  theme: {
    getTokens(): Promise<ThemeTokens>;
  };
  lifecycle: {
    onEnable(fn: () => void | Promise<void>): Promise<unknown>;
    onDisable(fn: () => void | Promise<void>): Promise<unknown>;
  };
  onCommand(trigger: string, handler: (ctx: { trigger: string; args: string }) => unknown): Promise<unknown>;
  onMessageAction(actionId: string, handler: (message: Record<string, unknown>) => unknown): void;
  onReady(cb: (context: Record<string, unknown>) => void): void;
}

declare global {
  const chatw: ChatwSdk;
}

export function getChatw(): ChatwSdk;
export function definePlugin(setup: (sdk: ChatwSdk) => void | Promise<void>): void;
export function createEmbed(embed: DiscordEmbed): DiscordEmbed;
