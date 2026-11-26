import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { MouseEvent } from 'react';
import { R5Observation } from './editor/r5/R5Observation';
import { R5ObservationControls } from './editor/r5/R5PreviewControls';
import { R5Workbench } from './editor/r5/R5Workbench';
import { WorldEditor } from './editor/WorldEditor';
import { WorkspaceSettings } from './layout/WorkspaceEditor';
import { addEcologyPiece, bounds, ecologyAnchor, effectiveMode, PIECES, pieceScaleLimits, pieceSize, pieceWorld,
  type EcologyBindings, type WorkspaceLayout, type WorkspacePiece } from './layout/layout';
import { usePanelInteraction } from './layout/panelInteraction';
import { useWorkspaceLayout } from './layout/workspace';
import { WorldScene } from './render/WorldScene';
import { R5SampleScene } from './render/r5/R5SampleScene';
import type { R5SampleFocus, R5SampleZoom } from './render/r5/geometry';
import type { R5ForestView } from './render/r5/ForestLiveScene';
import { r5Preview, useR5PreviewTheme, type R5PreviewTheme } from './r5Preview';
import { native, useWorld, viewId, worldName, type WorldSnapshot } from './world';

const hourMinute = new Intl.DateTimeFormat('zh-CN', {
  hour: '2-digit', minute: '2-digit', hourCycle: 'h23', timeZone: 'Asia/Shanghai',
});
const displayDate = new Intl.DateTimeFormat('zh-CN', {
  month: 'long', day: 'numeric', weekday: 'short', timeZone: 'Asia/Shanghai',
});
const idleDelayMs = 3_600;
const noCover = { mode: 'off', text: '' } as const;

function usePageVisible() {
  const [visible, setVisible] = useState(!document.hidden);
  useEffect(() => {
    const update = () => setVisible(!document.hidden);
    document.addEventListener('visibilitychange', update);
    return () => document.removeEventListener('visibilitychange', update);
  }, []);
  return visible;
}

function Clock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    let timeout: number;
    const update = () => {
      const time = new Date();
      setNow(time);
      window.clearTimeout(timeout);
      timeout = window.setTimeout(update, 60_000 - (time.getTime() % 60_000) + 25);
    };
    update();
    document.addEventListener('visibilitychange', update);
    return () => {
      window.clearTimeout(timeout);
      document.removeEventListener('visibilitychange', update);
    };
  }, []);

  return (
    <div className="beijing-clock" aria-label="北京时间">
      <time dateTime={now.toISOString()}>{hourMinute.format(now)}</time>
      <span>{displayDate.format(now)}</span>
    </div>
  );
}

type WorldView = ReturnType<typeof useWorld>;
const emptyWorldView: WorldView = { snapshot: null, connectionError: '', viewActive: false };
const feedKey = (id?: string) => id ? `world-${id}` : 'follow';

function PanelWorldFeed({ worldId, present }: { worldId?: string; present: (slot: string, view?: WorldView) => void }) {
  const slot = feedKey(worldId);
  const { snapshot, connectionError, viewActive } = useWorld(worldId, true, slot);
  useEffect(() => present(slot, { snapshot, connectionError, viewActive }), [slot, snapshot, connectionError, viewActive, present]);
  useEffect(() => () => present(slot), [slot, present]);
  return null;
}

type IdleCoverState = { enabled: boolean; covered: boolean };

function useIdleCover(idle: WorkspaceLayout['idle'], active: boolean, ready: boolean, blocked = false) {
  const [pointerInside, setPointerInside] = useState(false);
  const [focused, setFocused] = useState(false);
  const [afterDelay, setAfterDelay] = useState(false);
  const enabled = idle.mode === 'clock' || (idle.mode === 'text' && Boolean(idle.text.trim()));
  const waiting = enabled && active && ready && !blocked && !focused && !pointerInside;
  const covered = waiting && afterDelay;

  useEffect(() => {
    if (!waiting) { setAfterDelay(false); return; }
    const timer = window.setTimeout(() => setAfterDelay(true), idleDelayMs);
    return () => window.clearTimeout(timer);
  }, [waiting]);

  useEffect(() => {
    if (active) return;
    setPointerInside(false);
    setFocused(false);
    setAfterDelay(false);
  }, [active]);

  const hover = useCallback((value: boolean) => {
    setPointerInside(value);
    if (value) setAfterDelay(false);
  }, []);
  const focus = useCallback((value: boolean) => {
    setFocused(value);
    if (value) setAfterDelay(false);
  }, []);
  return { enabled, covered, hover, focus };
}

function Landscape({ snapshot, connectionError, active, idle, sharedCover, previewTheme, observation, desktopPanel }: {
  snapshot: WorldSnapshot | null;
  connectionError: string;
  active: boolean;
  idle: WorkspaceLayout['idle'];
  sharedCover?: IdleCoverState;
  previewTheme?: R5PreviewTheme;
  observation?: R5ForestView;
  desktopPanel?: boolean;
}) {
  const localCover = useIdleCover(idle, active && !sharedCover, Boolean(previewTheme || snapshot));
  const { enabled, covered } = sharedCover ?? localCover;
  const title = previewTheme ? (previewTheme === 'forest' ? '林下 R5 样板' : '浅水池塘 R5 样板') : worldName(snapshot);

  return (
    <section
      className={`landscape${covered ? ' landscape--covered' : ''}`}
      tabIndex={enabled && active ? 0 : undefined}
      aria-label={title + (enabled ? '，聚焦可恢复画面' : '')}
      onPointerEnter={sharedCover ? undefined : () => localCover.hover(true)}
      onPointerLeave={sharedCover ? undefined : () => localCover.hover(false)}
      onFocusCapture={sharedCover ? undefined : () => localCover.focus(true)}
      onBlurCapture={(event) => {
        if (!sharedCover && !event.currentTarget.contains(event.relatedTarget)) localCover.focus(false);
      }}
    >
      <div className="landscape__image" aria-hidden={covered}>
        {previewTheme ? <R5SampleScene theme={previewTheme} />
          : snapshot ? <WorldScene snapshot={snapshot} active={active && !covered} exactFit forestView={observation}
            pondFraming={desktopPanel ? 'underwater' : undefined} />
          : <p className="scene-message" role={connectionError ? 'alert' : 'status'}>{connectionError || '正在打开世界'}</p>}
      </div>
      {enabled && (
        <div className="landscape__cover" aria-hidden={!covered}>
          {idle.mode === 'clock' ? <Clock />
            : <span className={`landscape__message${idle.text.length > 12 ? ' landscape__message--long' : ''}`}>{idle.text}</span>}
          {!previewTheme && covered && (snapshot?.suspended || snapshot?.paused || snapshot?.lastSaveMessage?.includes('失败')) && (
            <span className="landscape__cover-status">{snapshot.suspended ? '系统暂停' : snapshot.lastSaveMessage?.includes('失败') ? '保存失败' : '世界已暂停'}</span>
          )}
        </div>
      )}
      {!previewTheme && snapshot && connectionError && <p className="landscape__error" role="alert">{connectionError}</p>}
    </section>
  );
}

function EnvironmentInfo({ snapshot }: { snapshot: WorldSnapshot | null }) {
  const status = snapshot?.suspended ? '系统暂停' : snapshot?.paused ? '已暂停' : snapshot ? '运行中' : '正在连接';
  const water = snapshot?.forest?.waterMl;
  const basin = snapshot?.pond?.basins[0];
  return (
    <section className="environment-info" aria-label={`${worldName(snapshot)}环境信息`}>
      <div><strong>{worldName(snapshot)}</strong><span>{status}</span></div>
      {basin && <div className="environment-info__measure">
        <span>{basin.wet ? '全池溶氧' : '已干涸'}</span>
        <strong>{basin.oxygenMgPerLiter === null ? '—' : basin.oxygenMgPerLiter.toFixed(2)}<small> mg/L</small></strong>
      </div>}
      {water ? <div className="environment-info__measure">
        <span>地表水量（mL）</span>
        <strong>左 {water[0].toFixed(1)}<i aria-hidden="true" />右 {water[1].toFixed(1)}</strong>
      </div> : snapshot?.moisture && <div className="environment-info__measure">
        <span>湿润度 · 试验读数</span>
        <strong>左 {Math.round(snapshot.moisture[0] * 100)}<small>%</small><i />右 {Math.round(snapshot.moisture[1] * 100)}<small>%</small></strong>
      </div>}
    </section>
  );
}

function inEcologyIdleGroup(target: EventTarget | null) {
  return target instanceof Element && Boolean(target.closest('[data-idle-group="true"]'));
}

function WorkspacePanel({ previewTheme }: { previewTheme?: R5PreviewTheme }) {
  const workspace = useWorkspaceLayout();
  const { layout, ready, loadError } = workspace;
  const interaction = usePanelInteraction(workspace);
  const [feeds, setFeeds] = useState<Record<string, WorldView>>({});
  const [adjust, setAdjust] = useState<{ id: string; kind: 'move' | 'resize' }>();
  const [menu, setMenu] = useState<{ pieceId: string; x: number; y: number } | null>(null);
  const panel = useRef<HTMLElement>(null);
  const visible = usePageVisible();
  const display = interaction.preview?.layout ?? layout;
  const box = bounds(display.items);
  const ecology = ecologyAnchor(display.items);
  const ecologyId = ecology?.id;
  const panelWorldId = ecology ? pieceWorld(ecology, layout.panelWorldId) : layout.panelWorldId;
  const world = feeds[feedKey(panelWorldId)] ?? emptyWorldView;
  const sources = new Set(layout.items.filter(({ kind }) => kind === 'ecology').map((piece) => pieceWorld(piece, layout.panelWorldId)));
  if (layout.items.some(({ kind }) => kind === 'environment')) sources.add(panelWorldId);
  const present = useCallback((slot: string, view?: WorldView) => setFeeds((before) => {
    if (view) return { ...before, [slot]: view };
    if (!Object.hasOwn(before, slot)) return before;
    const { [slot]: _closed, ...remaining } = before;
    return remaining;
  }), []);
  const mode = effectiveMode(display);
  const active = Boolean(previewTheme || Object.values(feeds).some(({ viewActive }) => viewActive)) && visible;
  const portal = interaction.preview?.portal;
  const contextPiece = menu && layout.items.find(({ id }) => id === menu.pieceId);
  const belongsToGroup = (piece: WorkspacePiece) => Boolean(ecologyId)
    && (piece.id === ecologyId || (mode !== 'free' && piece.docked === true));
  const arrangingGroup = display.items.some((piece) =>
    belongsToGroup(piece) && (piece.id === adjust?.id || piece.id === contextPiece?.id));
  const groupCover = useIdleCover(layout.idle, active, Boolean(ecologyId && (previewTheme || world.snapshot)),
    arrangingGroup || interaction.dragging !== null);
  const groupMembers = display.items.filter(belongsToGroup).map(({ id }) => id).sort().join(' ');

  function trackGroupBoundary(next: EventTarget | null, previous: EventTarget | null, signal: (value: boolean) => void) {
    // Only crossing the group edge changes idle state. Keeping cards as
    // siblings preserves pointer capture and the ordering during drag preview.
    const inside = inEcologyIdleGroup(next);
    if (inside !== inEcologyIdleGroup(previous)) signal(inside);
  }

  function openContext(event: MouseEvent<HTMLDivElement>, piece: WorkspacePiece) {
    event.preventDefault();
    if (interaction.dragging) return;
    const panel = event.currentTarget.closest('.desktop-panel');
    if (!panel) return;
    const rectangle = panel.getBoundingClientRect();
    setMenu({
      pieceId: piece.id,
      x: Math.max(5, Math.min(event.clientX - rectangle.left, rectangle.width - 126)),
      y: Math.max(5, Math.min(event.clientY - rectangle.top, rectangle.height - (layout.items.length > 1 ? 99 : 42))),
    });
  }

  useEffect(() => {
    if (!visible) { setMenu(null); setAdjust(undefined); }
  }, [visible]);

  useEffect(() => {
    if (!active || !ready || interaction.dragging) return;
    groupCover.hover(Boolean(panel.current?.querySelector('[data-idle-group="true"]:hover')));
    groupCover.focus(inEcologyIdleGroup(document.activeElement));
  }, [active, ready, interaction.dragging, groupMembers, groupCover.hover, groupCover.focus]);

  if (!ready) return <p className="scene-message panel-message" role="status">正在打开组件</p>;

  return (
    <main ref={panel} className="desktop-panel" aria-label="微缩生态组件面板" data-mode={mode} data-dragging={interaction.dragging ?? undefined}
      onPointerMove={interaction.browserMove} onPointerUp={interaction.browserEnd}
      onPointerOver={(event) => trackGroupBoundary(event.target, event.relatedTarget, groupCover.hover)}
      onPointerOut={(event) => trackGroupBoundary(event.relatedTarget, event.target, groupCover.hover)}
      onFocusCapture={(event) => trackGroupBoundary(event.target, event.relatedTarget, groupCover.focus)}
      onBlurCapture={(event) => trackGroupBoundary(event.relatedTarget, event.target, groupCover.focus)}
      onPointerCancel={() => { if (!native) interaction.cancel(); }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') { interaction.cancel(); setMenu(null); setAdjust(undefined); }
      }}>
      {!previewTheme && Array.from(sources, (id) => <PanelWorldFeed key={feedKey(id)} worldId={id} present={present} />)}
      {display.items.map((piece) => {
        const { width, height } = pieceSize(piece);
        const resizing = adjust?.id === piece.id && adjust.kind === 'resize';
        const individual = adjust?.id === piece.id && adjust.kind === 'move';
        const ecologyWorld = feeds[feedKey(pieceWorld(piece, layout.panelWorldId))] ?? emptyWorldView;
        const moving = interaction.preview?.pieceId === piece.id;
        const inGroup = belongsToGroup(piece);
        const maskedDock = inGroup && piece.docked && groupCover.covered;
        return (
          <div key={piece.id} className={`desktop-piece desktop-piece--${piece.kind}${piece.docked ? ' desktop-piece--docked' : ''}`}
            role="group" aria-label={PIECES[piece.kind].label}
            data-kind={piece.kind} data-piece-id={piece.id} data-adjust={adjust?.id === piece.id || moving || undefined}
            data-bound-world-id={piece.kind === 'ecology' ? ecologyWorld.snapshot?.worldId : undefined}
            data-scale-percent={piece.scalePercent ?? 100}
            data-idle-group={inGroup || undefined} data-idle-covered={(inGroup && groupCover.covered) || undefined}
            data-drop={piece.id === ecologyId && portal === 'attach' ? 'attach' : moving && portal === 'detach' ? 'detach' : undefined}
            onContextMenu={(event) => openContext(event, piece)}
            onPointerDown={(event) => {
              if (event.button !== 0) return;
              if (menu) { setMenu(null); return; }
              if (interaction.start(event, piece, event.shiftKey || individual ? 'piece' : 'panel')) {
                // A drag begins with a pointer, not keyboard focus. A clicked
                // canvas would otherwise keep its group awake after the pointer leaves.
                event.preventDefault();
                setAdjust(undefined);
              }
            }} style={{
            left: `${(piece.x - box.x) / box.width * 100}%`,
            top: `${(piece.y - box.y) / box.height * 100}%`,
            width: `${width / box.width * 100}%`,
            height: `${height / box.height * 100}%`,
          }}>
            <div className="desktop-piece__content" aria-hidden={maskedDock || undefined}>
              {piece.kind === 'ecology' && <Landscape snapshot={ecologyWorld.snapshot} connectionError={ecologyWorld.connectionError} previewTheme={previewTheme}
                active={visible && Boolean(previewTheme || ecologyWorld.viewActive)}
                idle={!inGroup && (interaction.dragging || resizing || contextPiece?.id === piece.id) ? noCover : layout.idle}
                sharedCover={inGroup ? groupCover : undefined} desktopPanel />}
              {piece.kind === 'clock' && <Clock />}
              {piece.kind === 'environment' && (previewTheme ? <section className="environment-info" aria-label="样板信息">
                <div><strong>{previewTheme === 'forest' ? '林下' : '浅水池塘'}</strong><span>样板</span></div>
                <div className="environment-info__measure"><span>参考画宽</span><strong>{previewTheme === 'forest' ? 450 : 600}<small>mm</small></strong></div>
              </section> : <EnvironmentInfo snapshot={world.snapshot} />)}
            </div>
            {resizing && <button type="button" className="desktop-piece__resize" role="slider" tabIndex={0} autoFocus
              aria-label="组件尺寸" aria-valuemin={pieceScaleLimits.minimum} aria-valuemax={pieceScaleLimits.maximum}
              aria-valuenow={piece.scalePercent ?? 100} aria-valuetext={`${piece.scalePercent ?? 100}%`} data-resize-piece={piece.id}
              title="拖动调整尺寸，方向键微调，Esc 退出"
              onPointerDown={(event) => {
                event.stopPropagation();
                if (interaction.start(event, piece, 'resize')) event.preventDefault();
              }} onKeyDown={(event) => {
                const step = event.shiftKey ? 10 : 2;
                const change = event.key === 'ArrowRight' || event.key === 'ArrowUp' ? step
                  : event.key === 'ArrowLeft' || event.key === 'ArrowDown' ? -step : 0;
                if (!change) return;
                event.preventDefault();
                interaction.resizeBy(piece.id, change);
              }} />}
          </div>
        );
      })}
      {(interaction.dragging === 'piece' || interaction.dragging === 'resize') && <div className="desktop-panel__rim" aria-hidden="true" />}
      {interaction.pulse > 0 && <div key={interaction.pulse} className="desktop-panel__pulse" aria-hidden="true" />}
      {portal && <div className="desktop-panel__drop-note" aria-live="polite">{portal === 'attach' ? '松手并入生态' : '松手分离组件'}</div>}
      {contextPiece && menu && <div className="desktop-panel__context" style={{ left: menu.x, top: menu.y }}>
        {layout.items.length > 1 && <button type="button" onClick={() => { setAdjust({ id: contextPiece.id, kind: 'move' }); setMenu(null); }}>移动单件</button>}
        <button autoFocus type="button" onClick={() => { setAdjust({ id: contextPiece.id, kind: 'resize' }); setMenu(null); }}>调整尺寸</button>
        {layout.items.length > 1 && <button type="button" onClick={() => {
          setMenu(null); setAdjust(undefined); interaction.remove(contextPiece.id);
        }}>移除此组件</button>}
      </div>}
      {(interaction.notice || loadError) && <p className="desktop-panel__notice" role="status">{interaction.notice || loadError}</p>}
    </main>
  );
}

function ObservationWindow({ world }: { world: WorldView }) {
  const visible = usePageVisible();
  const { layout } = useWorkspaceLayout();
  const [zoom, setZoom] = useState<R5SampleZoom>(1);
  const [focus, setFocus] = useState<R5SampleFocus>('center');
  const [showScale, setShowScale] = useState(false);
  const enhanced = Boolean(world.snapshot?.forestRocks || world.snapshot?.pond);
  return (
    <main className="observation-window" data-look-controls={enhanced || undefined}>
      <header className="observation-window__title" data-tauri-drag-region>
        <div data-tauri-drag-region><h1 data-tauri-drag-region>{worldName(world.snapshot)}</h1>
          {world.snapshot?.paused && <span data-tauri-drag-region>已暂停</span>}</div>
        <button type="button" className="observation-window__close" onClick={() => {
          if (native) void invoke('hide_window', { viewId: 'observe' });
          else window.location.assign('?view=panel');
        }}>关闭</button>
      </header>
      <Landscape snapshot={world.snapshot} connectionError={world.connectionError}
        active={world.viewActive && visible} idle={layout.idle} observation={enhanced ? { zoom, focus, showScale } : undefined} />
      {enhanced && <R5ObservationControls {...{ zoom, focus, showScale }} onZoomChange={setZoom}
        onFocusChange={setFocus} onScaleChange={setShowScale} />}
    </main>
  );
}

function ArrangementWindow() {
  const { ready, layout, latest, loadError, save, saveIdle } = useWorkspaceLayout();
  const [session, setSession] = useState(0);

  async function close() {
    if (native) {
      await invoke('hide_window', { viewId: 'layout' });
      setSession((value) => value + 1);
    } else {
      window.location.assign('?view=panel');
    }
  }

  async function saveAndClose(next: WorkspaceLayout['idle'], ecologyBindings: EcologyBindings) {
    await saveIdle(next, ecologyBindings);
    await close();
  }

  async function add() {
    if (native) await invoke('add_workspace_ecology');
    else await save(addEcologyPiece(latest.current, crypto.randomUUID()));
  }

  if (!ready) return <p className="scene-message" role="status">正在读取组件布局</p>;
  return (
    <div className="arrangement-window">
      {loadError && <p className="arrangement-window__error" role="alert">{loadError}。当前显示默认布置。</p>}
      <WorkspaceSettings key={session} saved={layout.idle} pieces={layout.items} panelWorldId={layout.panelWorldId}
        onSave={saveAndClose} onClose={close} onAdd={add} />
    </div>
  );
}

function WorldWindows() {
  const world = useWorld();
  if (viewId === 'observe') return <ObservationWindow world={world} />;
  return <WorldEditor {...world} />;
}

function R5PreviewWindows() {
  const { theme, changeTheme } = useR5PreviewTheme();
  if (viewId === 'panel') return <WorkspacePanel previewTheme={theme} />;
  function hidePreview() {
    if (native) void invoke('hide_window', { viewId }).catch((error: unknown) => console.error(error));
    else window.location.assign('?view=panel');
  }
  const props = { theme, onDesktopThemeChange: changeTheme, onHide: hidePreview };
  return viewId === 'observe' ? <R5Observation {...props} /> : <R5Workbench {...props} />;
}

export function App() {
  if (viewId === 'layout') return <ArrangementWindow />;
  if (r5Preview) return <R5PreviewWindows />;
  if (viewId === 'panel') return <WorkspacePanel />;
  return <WorldWindows />;
}
