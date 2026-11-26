// Runtime API & Media CDN origin configuration (empty = same-origin for local dev/tests).
// Production deployments override this file with dedicated API and CDN origins.
(function (root) {
  if (typeof root.CHATW_API_URL !== 'string') root.CHATW_API_URL = '';
  if (typeof root.CHATW_CDN_URL !== 'string') root.CHATW_CDN_URL = '';

  function isMediaPath(path) {
    return (
      path.startsWith('/uploads/') ||
      path.startsWith('/api/attachments/') ||
      path.startsWith('/api/media/') ||
      path.startsWith('/api/storage/uploads/') ||
      path.startsWith('/api/storage/msg-chunks/') ||
      path.startsWith('/stickers/') ||
      path.startsWith('/wallpapers/')
    );
  }

  root.CHATW_RESOLVE_URL = function (path) {
    if (
      typeof path !== 'string' ||
      !path ||
      path.startsWith('http://') ||
      path.startsWith('https://') ||
      path.startsWith('data:') ||
      path.startsWith('blob:')
    ) {
      return path;
    }
    var base = isMediaPath(path)
      ? (root.CHATW_CDN_URL || root.CHATW_API_URL || '')
      : (root.CHATW_API_URL || '');
    base = base.replace(/\/+$/, '');
    if (!base) return path;
    return path.charAt(0) === '/' ? base + path : base + '/' + path;
  };
})(typeof globalThis !== 'undefined' ? globalThis : self);
