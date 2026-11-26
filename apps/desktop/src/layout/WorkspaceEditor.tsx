import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import { errorText, native } from '../world';
import type { Library, WorldEntry } from '../editor/WorldLibrary';
import { ecologyAnchor, MAX_PIECES, pieceWorld, type EcologyBindings, type WorkspaceLayout, type WorkspacePiece } from './layout';
import './workspace-editor.css';

type IdleSetting = WorkspaceLayout['idle'];

interface WorkspaceSettingsProps {
  saved: IdleSetting;
  pieces: WorkspacePiece[];
  panelWorldId?: string;
  onSave: (idle: IdleSetting, ecologyBindings: EcologyBindings) => Promise<void>;
  onClose: () => Promise<void>;
  onAdd: () => Promise<void>;
}

const idleOptions: { mode: IdleSetting['mode']; label: string }[] = [
  { mode: 'off', label: '保持画面' },
  { mode: 'clock', label: '时钟' },
  { mode: 'text', label: '自定义文字' },
];

export function WorkspaceSettings({ saved, pieces, panelWorldId, onSave, onClose, onAdd }: WorkspaceSettingsProps) {
  const [draft, setDraft] = useState(saved);
  const [bindings, setBindings] = useState<EcologyBindings>({});
  const [worlds, setWorlds] = useState<WorldEntry[]>([]);
  const [worldError, setWorldError] = useState('');
  const [pending, setPending] = useState<'save' | 'close' | 'add' | null>(null);
  const [error, setError] = useState('');
  const activeAction = useRef(false);
  const hasText = draft.mode !== 'text' || Boolean(draft.text.trim());
  const primary = ecologyAnchor(pieces)?.id;
  const ecology = pieces.filter(({ kind }) => kind === 'ecology')
    .sort((a, b) => Number(b.id === primary) - Number(a.id === primary) || a.id.localeCompare(b.id));

  useEffect(() => {
    if (!native) return;
    let closed = false;
    function update() {
      void invoke<Library>('list_worlds').then((value) => {
        if (closed) return;
        setWorlds(value.worlds.filter((world) => !world.archived && (world.state === 'ready' || world.state === 'recovered')));
        setWorldError('');
      }).catch((reason) => {
        if (!closed) setWorldError('无法读取世界列表。' + errorText(reason));
      });
    }
    update();
    window.addEventListener('focus', update);
    return () => {
      closed = true;
      window.removeEventListener('focus', update);
    };
  }, []);

  async function finish(action: 'save' | 'close' | 'add') {
    if (activeAction.current || (action === 'save' && !hasText)) return;
    activeAction.current = true;
    setPending(action);
    setError('');
    try {
      if (action === 'save') await onSave({ ...draft, text: draft.text.trim() }, bindings);
      else if (action === 'close') await onClose();
      else await onAdd();
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      activeAction.current = false;
      setPending(null);
    }
  }

  return (
    <main className="workspace-settings" onKeyDown={(event) => {
      if (event.key !== 'Escape' || event.nativeEvent.isComposing) return;
      event.preventDefault();
      void finish('close');
    }}>
      <header className="workspace-settings__heading" data-tauri-drag-region>
        <h1 data-tauri-drag-region>组件设置</h1>
        <button type="button" className="settings-button settings-button--close" onClick={() => void finish('close')} disabled={pending !== null}
          title="关闭设置，舍弃未保存的修改">
          <span aria-hidden="true">×</span>关闭
        </button>
      </header>

      <form className="workspace-settings__form" onSubmit={(event) => {
        event.preventDefault();
        void finish('save');
      }}>
        <section className="workspace-settings__sources" aria-label="生态组件的世界">
          {native && ecology.map((piece, index) => {
            const fixedWorld = (Object.hasOwn(bindings, piece.id) ? bindings[piece.id] : pieceWorld(piece, panelWorldId)) ?? '';
            return <label key={piece.id} className="workspace-settings__text workspace-settings__world">
              <span>{index === 0 ? '主生态组件' : `生态组件 ${index + 1}`}</span>
              <select value={fixedWorld} data-ecology-binding={piece.id} onChange={(event) => {
                const value = event.target.value || null;
                setBindings((before) => ({ ...before, [piece.id]: value }));
              }} disabled={pending !== null}>
                <option value="">跟随编辑器当前世界</option>
                {fixedWorld && !worlds.some((world) => world.worldId === fixedWorld) && <option value={fixedWorld}>已绑定的世界暂不可用</option>}
                {worlds.map(({ worldId, name }) => <option key={worldId} value={worldId}>{name}</option>)}
              </select>
            </label>;
          })}
          {worldError && <small role="status">{worldError}</small>}
          <button type="button" className="settings-button" data-add-ecology disabled={pending !== null || pieces.length >= MAX_PIECES}
            onClick={() => void finish('add')}>添加生态组件</button>
        </section>
        <fieldset className="workspace-settings__idle" disabled={pending !== null}>
          <legend>离开生态组件后显示</legend>
          <div className="workspace-settings__choices">
            {idleOptions.map(({ mode, label }) => (
              <label className="settings-choice" key={mode}>
                <input type="radio" name="idle-display" value={mode} checked={draft.mode === mode}
                  onChange={() => setDraft((current) => ({ ...current, mode }))} />
                <span>{label}</span>
              </label>
            ))}
          </div>
          {draft.mode === 'text' && (
            <label className="workspace-settings__text">
              <span>显示文字</span>
              <input type="text" maxLength={30} required value={draft.text} aria-describedby="workspace-text-limit"
                onChange={(event) => setDraft({ ...draft, text: event.target.value })} />
              <small id="workspace-text-limit">最多 30 字</small>
            </label>
          )}
        </fieldset>

        <section className="workspace-settings__guide" aria-labelledby="workspace-gesture-heading">
          <h2 id="workspace-gesture-heading">在桌面上移动</h2>
          <dl>
            <div><dt>拖动组件</dt><dd>移动整个面板</dd></div>
            <div><dt><kbd>Shift</kbd> + 拖动</dt><dd>移动单个组件</dd></div>
            <div><dt>右键组件</dt><dd>从菜单开启单次位置调整</dd></div>
            <div><dt>调整尺寸</dt><dd>右键选择后拖动右下角，方向键可微调</dd></div>
          </dl>
        </section>

        <div className="workspace-settings__actions">
          {error && <p role="alert">{error}</p>}
          <button type="submit" className="settings-button settings-button--primary" disabled={pending !== null || !hasText}>
            {pending === 'save' ? '正在保存' : '保存设置'}
          </button>
        </div>
      </form>
    </main>
  );
}
