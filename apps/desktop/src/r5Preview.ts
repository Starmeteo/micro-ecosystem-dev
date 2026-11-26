import { emit, listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { native } from './world';

export const r5Preview = import.meta.env.DEV
  && (import.meta.env.MODE === 'r5-preview' || import.meta.env.VITE_R5_SAMPLE === '1');
export type R5PreviewTheme = 'forest' | 'pond';

const themeKey = 'micro-ecosystem.r5-preview.theme';
const themeChanged = 'r5-preview-theme-changed';

function savedTheme(): R5PreviewTheme {
  try {
    return localStorage.getItem(themeKey) === 'pond' ? 'pond' : 'forest';
  } catch {
    return 'forest';
  }
}

export function useR5PreviewTheme() {
  const [theme, setTheme] = useState(savedTheme);
  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    const fromStorage = (event: StorageEvent) => {
      if (event.key === themeKey) setTheme(savedTheme());
    };
    window.addEventListener('storage', fromStorage);
    if (native) {
      void listen<R5PreviewTheme>(themeChanged, ({ payload }) => {
        if (payload === 'forest' || payload === 'pond') setTheme(payload);
      }).then((remove) => {
        if (disposed) remove();
        else unsubscribe = remove;
      }).catch(() => undefined);
    }
    return () => {
      disposed = true;
      unsubscribe?.();
      window.removeEventListener('storage', fromStorage);
    };
  }, []);

  function changeTheme(next: R5PreviewTheme) {
    setTheme(next);
    try { localStorage.setItem(themeKey, next); } catch { /* Other native windows can still receive the event. */ }
    if (native) void emit(themeChanged, next).catch(() => undefined);
  }

  return { theme, changeTheme };
}
