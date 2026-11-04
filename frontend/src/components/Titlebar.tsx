import { Show, createSignal, onCleanup, onMount } from 'solid-js';
import { isTauri } from '../api/client';
import { LogoIcon } from './Icons';

export function Titlebar() {
  const [maximized, setMaximized] = createSignal(false);
  const inTauri = isTauri();

  let appWindow: any = null;

  onMount(async () => {
    if (inTauri) {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        appWindow = getCurrentWindow();
        setMaximized(await appWindow.isMaximized());
        const unlisten = await appWindow.onResized(async () => {
          setMaximized(await appWindow.isMaximized());
        });
        onCleanup(() => {
          unlisten();
        });
      } catch (err) {
        console.warn('Tauri window API not available:', err);
      }
    }
  });

  const handleMinimize = () => {
    appWindow?.minimize();
  };

  const handleToggleMaximize = async () => {
    if (appWindow) {
      await appWindow.toggleMaximize();
      setMaximized(await appWindow.isMaximized());
    }
  };

  const handleClose = () => {
    appWindow?.close();
  };

  return (
    <header data-tauri-drag-region class="custom-titlebar">
      <div data-tauri-drag-region class="titlebar-left">
        <div class="titlebar-icon">
          <LogoIcon size={16} />
        </div>
        <span data-tauri-drag-region class="titlebar-text">
          chatw – Secure Messaging
        </span>
      </div>

      <div
        data-tauri-drag-region
        class="titlebar-spacer"
        onDblClick={handleToggleMaximize}
      />

      <Show when={inTauri}>
        <div class="titlebar-controls">
          <button
            type="button"
            class="titlebar-btn titlebar-btn-minimize"
            onClick={handleMinimize}
            title="Thu nhỏ"
            aria-label="Thu nhỏ"
          >
            <svg width="10" height="1" viewBox="0 0 10 1">
              <path fill="currentColor" d="M0 0h10v1H0z" />
            </svg>
          </button>

          <button
            type="button"
            class="titlebar-btn titlebar-btn-maximize"
            onClick={handleToggleMaximize}
            title={maximized() ? 'Khôi phục' : 'Phóng to'}
            aria-label={maximized() ? 'Khôi phục' : 'Phóng to'}
          >
            <Show
              when={maximized()}
              fallback={
                <svg width="10" height="10" viewBox="0 0 10 10">
                  <path
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1"
                    d="M0.5 0.5h9v9h-9z"
                  />
                </svg>
              }
            >
              <svg width="10" height="10" viewBox="0 0 10 10">
                <path
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1"
                  d="M2.5 0.5h7v7h-7z"
                />
                <path
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1"
                  d="M0.5 2.5h7v7h-7z"
                />
              </svg>
            </Show>
          </button>

          <button
            type="button"
            class="titlebar-btn titlebar-btn-close"
            onClick={handleClose}
            title="Đóng"
            aria-label="Đóng"
          >
            <svg width="10" height="10" viewBox="0 0 10 10">
              <path
                fill="currentColor"
                d="M1.07 0L0 1.07l3.93 3.93L0 8.93 1.07 10l3.93-3.93L8.93 10 10 8.93 6.07 5 10 1.07 8.93 0 5 3.93z"
              />
            </svg>
          </button>
        </div>
      </Show>
    </header>
  );
}
