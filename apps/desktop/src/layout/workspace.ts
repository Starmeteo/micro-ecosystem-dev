import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { errorText, native, viewId } from '../world';
import { applyEcologyBindings, bounds, DEFAULT_LAYOUT, LAYOUT_AREA, readLayout, shiftToPanelOrigin,
  type CompositionMode, type EcologyBindings, type PanelPlacement, type PieceKind, type WorkspaceLayout } from './layout';

const previewKey = 'micro-ecosystem-preview-layout-v1';
const changedEvent = 'workspace-layout-changed';

interface Rectangle { x: number; y: number; width: number; height: number }
export interface PanelBounds extends Rectangle {
  scaleFactor: number;
  workArea: Rectangle;
  monitorName: string | null;
  monitorX: number;
  monitorY: number;
}
type PanelBoundsRequest = Rectangle | { width: number; height: number; placement: PanelPlacement };
export interface PanelPointer { x: number; y: number; primaryDown: boolean }
export type TrayAction = { id: string } & ({ action: 'toggle'; kind: PieceKind } | { action: 'mode'; mode: CompositionMode } | { action: 'add-ecology' });

export function useWorkspaceLayout() {
  const [layout, setLayout] = useState<WorkspaceLayout>(DEFAULT_LAYOUT);
  const latest = useRef(layout);
  const [ready, setReady] = useState(false);
  const [loadError, setLoadError] = useState('');

  useEffect(() => {
    let disposed = false;
    let eventReceived = false;
    let unlisten: (() => void) | undefined;

    function apply(value: unknown) {
      if (disposed) return;
      try {
        const checked = readLayout(value);
        latest.current = checked;
        setLayout(checked);
        setLoadError('');
      } catch (error) {
        setLoadError(errorText(error));
      } finally {
        setReady(true);
      }
    }

    function storageChanged(event: StorageEvent) {
      if (event.key !== previewKey) return;
      try { apply(event.newValue ? JSON.parse(event.newValue) : null); }
      catch (error) { if (!disposed) setLoadError(errorText(error)); }
    }

    if (native) {
      void (async () => {
        try {
          const unsubscribe = await listen<unknown>(changedEvent, ({ payload }) => {
            eventReceived = true;
            apply(payload);
          }, { target: viewId });
          if (disposed) { unsubscribe(); return; }
          unlisten = unsubscribe;
          let value: unknown;
          for (let retry = 0; ; retry += 1) {
            try {
              value = await invoke<unknown>('get_workspace_layout');
              break;
            } catch (error) {
              if (disposed || eventReceived) return;
              if (retry >= 12 || !errorText(error).includes('state not managed')) throw error;
              await new Promise<void>((resolve) => window.setTimeout(resolve, 140));
            }
          }
          if (eventReceived || disposed) return;
          if (viewId === 'panel' && value && typeof value === 'object' && 'version' in value && value.version === 1) {
            const migrated = readLayout(value);
            try {
              const result = await invoke<unknown>('save_workspace_layout', { layout: migrated });
              if (!eventReceived) apply(result);
            } catch (error) {
              if (eventReceived || disposed) return;
              apply(migrated);
              setLoadError('旧布局已读取。无法更新存档：' + errorText(error));
            }
          } else {
            apply(value);
          }
        } catch (error) {
          if (!disposed && !eventReceived) {
            setLoadError(errorText(error));
            setReady(true);
          }
        }
      })();
    } else {
      window.addEventListener('storage', storageChanged);
      try {
        const value = localStorage.getItem(previewKey);
        apply(value ? JSON.parse(value) : null);
      } catch (error) {
        setLoadError(errorText(error));
        setReady(true);
      }
    }

    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener('storage', storageChanged);
    };
  }, []);

  async function save(next: WorkspaceLayout, trayThroughId?: string) {
    const checked = readLayout(next);
    const saved = native
      ? readLayout(await invoke<unknown>('save_workspace_layout', { layout: checked, trayThroughId }))
      : checked;
    if (!native) localStorage.setItem(previewKey, JSON.stringify(checked));
    latest.current = saved;
    setLayout(saved);
    setLoadError('');
    return saved;
  }

  async function saveIdle(idle: WorkspaceLayout['idle'], ecologyBindings: EcologyBindings) {
    if (!native) return save(applyEcologyBindings({ ...latest.current, idle }, ecologyBindings));
    const saved = readLayout(await invoke<unknown>('save_workspace_idle', { idle, layout: latest.current, ecologyBindings }));
    latest.current = saved;
    setLayout(saved);
    setLoadError('');
    return saved;
  }

  return { layout, latest, ready, loadError, save, saveIdle };
}

export const readPanelBounds = () => invoke<PanelBounds>('get_panel_bounds');
export const readPanelPointer = () => invoke<PanelPointer>('get_panel_pointer');
export const setPanelBounds = (bounds: PanelBoundsRequest) => invoke<PanelBounds>('set_panel_bounds', { bounds });
export const pendingTrayActions = () => invoke<TrayAction[]>('get_workspace_tray_actions');
export const trayActionEvent = 'workspace-tray-action';

export async function placeNativePanel(layout: WorkspaceLayout, shouldApply: () => boolean) {
  if (!native || !shouldApply()) return;
  const rect = bounds(layout.items);
  if (layout.panelPlacement) return setPanelBounds({ width: rect.width, height: rect.height, placement: layout.panelPlacement });
  const current = await readPanelBounds();
  if (!shouldApply()) return;
  const { scaleFactor, workArea } = current;
  const fit = Math.min(1, workArea.width / (rect.width * scaleFactor), workArea.height / (rect.height * scaleFactor));
  const width = Math.max(1, Math.floor(rect.width * fit));
  const height = Math.max(1, Math.floor(rect.height * fit));
  const horizontal = rect.x / Math.max(1, LAYOUT_AREA.width - rect.width);
  const vertical = rect.y / Math.max(1, LAYOUT_AREA.height - rect.height);
  const x = workArea.x + Math.round((workArea.width - width * scaleFactor) * horizontal);
  const y = workArea.y + Math.round((workArea.height - height * scaleFactor) * vertical);
  return setPanelBounds({ x, y, width, height });
}

export function resizeAroundPanel(layout: WorkspaceLayout, before: WorkspaceLayout, starting: PanelBounds) {
  const oldBox = bounds(before.items);
  const nextBox = bounds(layout.items);
  const scale = starting.scaleFactor;
  const fit = Math.min(1, starting.width / scale / oldBox.width, starting.height / scale / oldBox.height);
  const nextFit = Math.min(fit, starting.workArea.width / scale / nextBox.width, starting.workArea.height / scale / nextBox.height);
  return {
    x: starting.x + Math.round((nextBox.x - oldBox.x) * scale * fit),
    y: starting.y + Math.round((nextBox.y - oldBox.y) * scale * fit),
    width: Math.max(1, Math.floor(nextBox.width * nextFit + .001)),
    height: Math.max(1, Math.floor(nextBox.height * nextFit + .001)),
  };
}

export function layoutAtPanel(layout: WorkspaceLayout, current: PanelBounds) {
  const box = bounds(layout.items);
  const availableX = current.workArea.width - current.width;
  const availableY = current.workArea.height - current.height;
  const horizontal = availableX > 0 ? Math.max(0, Math.min(1, (current.x - current.workArea.x) / availableX))
    : layout.panelPlacement?.horizontal ?? box.x / Math.max(1, LAYOUT_AREA.width - box.width);
  const vertical = availableY > 0 ? Math.max(0, Math.min(1, (current.y - current.workArea.y) / availableY))
    : layout.panelPlacement?.vertical ?? box.y / Math.max(1, LAYOUT_AREA.height - box.height);
  const shifted = shiftToPanelOrigin(layout, horizontal * (LAYOUT_AREA.width - box.width), vertical * (LAYOUT_AREA.height - box.height));
  const { monitorName, monitorX, monitorY } = current;
  return { ...shifted, panelPlacement: { monitorName, monitorX, monitorY, horizontal, vertical } };
}
