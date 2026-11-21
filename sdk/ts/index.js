export function getChatw() {
  if (!globalThis.chatw) {
    throw new Error('ChatW Plugin Runtime is not available in this context');
  }
  return globalThis.chatw;
}

export function definePlugin(setup) {
  const sdk = getChatw();
  sdk.onReady(() => {
    Promise.resolve(setup(sdk)).catch((err) => {
      try {
        sdk.ui.notify(`Plugin init error: ${err?.message || err}`, 'error');
      } catch (_) {}
    });
  });
}

export function createEmbed(embed) {
  return {
    title: embed.title || undefined,
    description: embed.description || undefined,
    url: embed.url || undefined,
    color: typeof embed.color === 'number' ? embed.color : 0x2563eb,
    timestamp: embed.timestamp || new Date().toISOString(),
    author: embed.author || undefined,
    footer: embed.footer || undefined,
    thumbnail: embed.thumbnail || undefined,
    image: embed.image || undefined,
    fields: Array.isArray(embed.fields) ? embed.fields.slice(0, 25) : [],
  };
}
