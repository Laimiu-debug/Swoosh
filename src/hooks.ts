import { useCallback, useEffect, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { Run, Snapshot } from './types';

export const native = isTauri();

const POLL_INTERVAL = 600;
const NOTICE_DURATION = 2500;

/** Polls the native core for state, and exposes a `run` helper that refreshes after each command. */
export function useCore() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  const refresh = useCallback(async () => {
    if (!native) return;
    try { setSnapshot(await invoke<Snapshot>('snapshot')); } catch (error) { setError(String(error)); }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), POLL_INTERVAL);
    return () => clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(''), NOTICE_DURATION);
    return () => clearTimeout(timer);
  }, [notice]);

  const run: Run = useCallback(async <T>(command: string, args?: Record<string, unknown>, success?: string) => {
    setError('');
    try {
      const value = await invoke<T>(command, args);
      await refresh();
      if (success) setNotice(success);
      return value;
    } catch (error) {
      setError(String(error));
      return null;
    }
  }, [refresh]);

  return { snapshot, run, error, setError, notice, setNotice };
}

type Theme = 'system' | 'light' | 'dark';
const THEME_KEY = 'swoosh-theme';
const nextTheme: Record<Theme, Theme> = { system: 'light', light: 'dark', dark: 'system' };

function storedTheme(): Theme {
  try {
    const value = localStorage.getItem(THEME_KEY);
    return value === 'light' || value === 'dark' ? value : 'system';
  } catch { return 'system'; }
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(storedTheme);
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'system') delete root.dataset.theme; else root.dataset.theme = theme;
    try { localStorage.setItem(THEME_KEY, theme); } catch { /* Preference is best-effort. */ }
  }, [theme]);
  return { theme, cycleTheme: () => setTheme(nextTheme[theme]) };
}

/** Tracks native drag-and-drop over the window and reports dropped paths. */
export function useFileDrop(onDrop: (paths: string[]) => void) {
  const [dragging, setDragging] = useState(false);
  useEffect(() => {
    if (!native) return;
    let disposed = false, unlisten: (() => void) | undefined;
    void getCurrentWebviewWindow().onDragDropEvent(({ payload }) => {
      setDragging(payload.type === 'enter' || payload.type === 'over');
      if (payload.type === 'drop') onDrop(payload.paths);
    }).then(stop => { if (disposed) stop(); else unlisten = stop; });
    return () => { disposed = true; unlisten?.(); };
  }, [onDrop]);
  return dragging;
}
