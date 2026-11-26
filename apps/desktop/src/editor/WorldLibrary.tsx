import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { errorText } from '../world';

type Theme = 'forest' | 'pond';
type Shelf = 'worlds' | 'presets';

export interface WorldEntry {
  worldId: string;
  name: string;
  archived: boolean;
  state: 'ready' | 'recovered' | 'unreadable' | 'unsupported';
  theme?: Theme | null;
  modelVersion?: number | null;
  definition?: string | null;
  message?: string | null;
  paused?: boolean | null;
  simulationTick?: string | null;
}

export interface PresetEntry {
  presetId: string;
  name: string;
  theme: Theme;
  source: 'builtIn' | 'blank' | 'custom';
  modelVersion: number;
}

export interface Library {
  activeWorldId: string;
  worlds: WorldEntry[];
  presets: PresetEntry[];
  presetWarnings?: string[];
}

export type LibraryAction =
  | { command: 'switch_world'; payload: { worldId: string } }
  | { command: 'create_world'; payload: { presetId: string; name: string } }
  | { command: 'copy_world' | 'rename_world'; payload: { worldId: string; name: string } }
  | { command: 'archive_world'; payload: { worldId: string; archived: boolean } }
  | { command: 'save_world_preset'; payload: { name: string } };

type NameDraft = { kind: 'copy' | 'rename'; worldId: string; name: string }
  | { kind: 'savePreset'; name: string };

const themeLabels = { forest: '林下', pond: '浅水池塘' };
const sourceLabels = { builtIn: '内置', blank: '空白', custom: '个人' };

function worldStatus(world: WorldEntry) {
  if (world.archived) return '已归档';
  if (world.state === 'unreadable') return '检查点无法读取';
  if (world.state === 'unsupported') return '存档版本暂不支持';
  if (world.state === 'recovered') return '可从上一有效检查点恢复';
  if (world.paused === true) return '已暂停';
  return world.paused === false ? '运行中' : '已保存';
}

function worldTheme(world: WorldEntry) {
  if (world.theme) return themeLabels[world.theme];
  if (world.definition?.startsWith('pond-')) return themeLabels.pond;
  if (world.definition?.startsWith('forest-')) return themeLabels.forest;
  return '';
}

function presetDescription(preset: PresetEntry) {
  if (preset.presetId === 'forest-r5-composed') return '从分层林景开始，当地形移动时附着装饰随之移动。短拱树皮、石块和可分解落叶仍参与生态。';
  if (preset.presetId === 'blank-forest-r5-composed') return '从林地底景开始，可独立布置地形、落叶资源和画面造景。';
  if (preset.presetId === 'pond-r5-composed') return '以水下造景开始，可重排场景摆件。斜沉木提供停靠面，支持附着斑块。';
  if (preset.presetId === 'blank-pond-r5-composed') return '从空砂床开始，可摆放石木与场景摆件，并在环境页补水。';
  if (preset.presetId === 'pond-r5-moss') return '可在主石和沉木上附着苔藓小垫或分枝绒簇，查看水线与碳氮磷库存，并按量修剪。';
  if (preset.presetId === 'pond-r5-wood') return '沿池底布置石材、主枝和短根沉木。选中沉木可查看停靠小虾及附着装饰。';
  if (preset.presetId === 'pond-r5-stones') return '沿池底摆放主石和卵石，观察小虾绕行。水量与水质仍可在环境页查看。';
  if (preset.presetId === 'forest-r5-litter') return '可把落叶簇或落叶堆放入土格。选中后可查看投放量及剩余碳、氮、磷。';
  if (preset.presetId === 'forest-r5-bark') return '可放置短拱树皮或裂口半筒。鼠妇可从前后入口通过，选中洞口可查看洞内鼠妇数量。';
  if (preset.presetId === 'forest-r5-stonework') return '可布置扁石、两石组、三石组、主石和贴地木段。石组按整件选择、移动和撤销。';
  if (preset.presetId === 'forest-r5-terrain') return '编辑时可放置、移动、翻转和移除苔斑岩石及贴地朽木段。木段会占用一段地面。';
  if (preset.presetId === 'forest-r5') return '使用 R5 林下画面。编辑时可放置、移动、翻转和移除独立石块。';
  if (preset.source === 'blank') return preset.theme === 'pond'
    ? '从干床开始，可以补水、调整面积，并放入初始生物。'
    : '从林下基底开始，可以移动石块并放入初始生物。';
  if (preset.source === 'custom') return '使用已保存的环境布置和地点，从第 0 步开始。';
  return preset.theme === 'pond'
    ? '含一片浅水水域、水草、附着生物膜、米虾和水蚤。'
    : '含林下土壤、遮蔽石块、鼠妇、弹尾虫和植物斑块。';
}

export function WorldLibrary({ busy, canAct, dismiss, onAction, operationError }: {
  busy: boolean;
  canAct: boolean;
  dismiss: () => void;
  onAction: (action: LibraryAction) => Promise<boolean>;
  operationError?: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [library, setLibrary] = useState<Library | null>(null);
  const [listError, setListError] = useState('');
  const [refresh, setRefresh] = useState(0);
  const [shelf, setShelf] = useState<Shelf>('worlds');
  const [draft, setDraft] = useState<NameDraft | null>(null);
  const [archiveTarget, setArchiveTarget] = useState<WorldEntry | null>(null);
  const [presetId, setPresetId] = useState('');
  const [newName, setNewName] = useState('');
  const [selectedWorldId, setSelectedWorldId] = useState<string | null>(null);

  const preset = library?.presets.find((item) => item.presetId === presetId) ?? library?.presets[0];
  const current = library?.worlds.find((item) => item.worldId === library.activeWorldId);
  const selectedWorld = library?.worlds.find((item) => item.worldId === selectedWorldId) ?? current;
  const available = library?.worlds.filter((item) => !item.archived) ?? [];
  const archived = library?.worlds.filter((item) => item.archived) ?? [];

  useEffect(() => {
    const surface = dialog.current;
    if (surface && !surface.open) surface.showModal();
  }, []);

  useEffect(() => {
    let active = true;
    void invoke<Library>('list_worlds').then((result) => {
      if (!active) return;
      setLibrary(result);
      setListError('');
    }, (error: unknown) => {
      if (!active) return;
      setListError(errorText(error));
    });
    return () => { active = false; };
  }, [refresh]);

  async function run(action: LibraryAction) {
    if (!canAct || !await onAction(action)) return;
    if (action.command === 'switch_world' || action.command === 'create_world' || action.command === 'copy_world') return;
    setDraft(null);
    setArchiveTarget(null);
    setRefresh((value) => value + 1);
  }

  function submitName(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!draft || !draft.name.trim()) return;
    const name = draft.name.trim();
    if (draft.kind === 'savePreset') void run({ command: 'save_world_preset', payload: { name } });
    else void run({ command: draft.kind === 'rename' ? 'rename_world' : 'copy_world', payload: { worldId: draft.worldId, name } });
  }

  function switchShelf(next: Shelf) {
    setDraft(null);
    setArchiveTarget(null);
    setShelf(next);
  }

  function worldRow(world: WorldEntry) {
    const active = world.worldId === library?.activeWorldId;
    const status = [worldTheme(world), worldStatus(world)].filter(Boolean).join(' · ');
    return (
      <li key={world.worldId} className="editor-library__world" data-library-world={world.worldId}
        data-active={active || undefined} data-selected={selectedWorld?.worldId === world.worldId || undefined}>
        <button type="button" className="editor-library__identity" disabled={busy}
          onClick={() => {
            setSelectedWorldId(world.worldId);
            setDraft(null);
            setArchiveTarget(null);
          }} aria-pressed={selectedWorld?.worldId === world.worldId}>
          <strong>{world.name}</strong>
          <span>{active ? `当前世界 · ${status}` : status}</span>
        </button>
      </li>
    );
  }

  function worldPreview(world: WorldEntry) {
    const active = world.worldId === library?.activeWorldId;
    const readable = world.state === 'ready' || world.state === 'recovered';
    return (
      <section className="editor-library__preview" aria-label="所选世界预览">
        <div className="editor-library__preview-info">
          <strong>{world.name}</strong>
          <span>{[active && '当前世界', worldTheme(world), worldStatus(world)].filter(Boolean).join(' · ')}</span>
        </div>
        {world.simulationTick && <p>已保存进度 · 第 {world.simulationTick} 步</p>}
        {world.message && <p>{world.message}</p>}
        <div className="editor-library__row-actions">
          {world.archived ? <button type="button" className="editor-tool" disabled={!canAct} onClick={() => void run({
            command: 'archive_world', payload: { worldId: world.worldId, archived: false },
          })}>恢复到世界库</button> : <>
            {!active && <button type="button" className="editor-button editor-button--primary" disabled={!canAct || !readable} onClick={() => void run({
              command: 'switch_world', payload: { worldId: world.worldId },
            })}>打开世界</button>}
            <button type="button" className="editor-tool" disabled={!canAct} onClick={() => {
              setArchiveTarget(null);
              setDraft({ kind: 'rename', worldId: world.worldId, name: world.name });
            }}>重命名</button>
            <button type="button" className="editor-tool" disabled={!canAct || !readable} onClick={() => {
              setArchiveTarget(null);
              setDraft({ kind: 'copy', worldId: world.worldId, name: `${world.name.slice(0, 74)} 副本` });
            }}>复制现状</button>
            {!active && <button type="button" className="editor-tool" disabled={!canAct} onClick={() => {
              setDraft(null);
              setArchiveTarget(world);
            }}>归档</button>}
          </>}
        </div>
      </section>
    );
  }

  return (
    <dialog className="editor-library" ref={dialog} aria-labelledby="world-library-title" onCancel={(event) => {
      event.preventDefault();
      if (!busy) dismiss();
    }}>
      <header className="editor-library__heading">
        <h2 id="world-library-title">世界库</h2>
        <button type="button" className="editor-button editor-button--close" disabled={busy} onClick={dismiss}>关闭</button>
      </header>
      <div className="editor-library__controls" role="group" aria-label="世界库视图">
        <button type="button" className="editor-tool" disabled={busy} aria-pressed={shelf === 'worlds'} onClick={() => switchShelf('worlds')}>我的世界</button>
        <button type="button" className="editor-tool" disabled={busy} aria-pressed={shelf === 'presets'} onClick={() => switchShelf('presets')}>从预设新建</button>
      </div>
      <div className="editor-library__body">
        {!library && !listError && <p role="status">正在读取世界库</p>}
        {listError && <div role="alert" className="editor-library__form">
          <p>{listError}</p><button type="button" className="editor-tool" onClick={() => setRefresh((value) => value + 1)}>重新读取</button>
        </div>}
        {library && shelf === 'worlds' && <>
          {current && <div className="editor-library__controls">
            <button type="button" className="editor-tool" disabled={!canAct || (current.state !== 'ready' && current.state !== 'recovered')}
              onClick={() => { setArchiveTarget(null); setDraft({ kind: 'savePreset', name: `${current.name.slice(0, 74)} 布置` }); }}>
              将当前布置存为预设
            </button>
          </div>}
          {draft && <form className="editor-library__form" onSubmit={submitName}>
            <label>{draft.kind === 'savePreset' ? '预设名称' : '世界名称'}
              <input key={draft.kind} autoFocus required maxLength={80} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
            </label>
            {draft.kind === 'copy' && <p>复制当前进度、资源和生物，另建一个暂停的世界。</p>}
            {draft.kind === 'savePreset' && <p>保留支持的环境布置和日照地点。从个人预设新建时，以初始生态配置和第 0 步开始。</p>}
            <div className="editor-library__controls">
              <button type="submit" className="editor-button editor-button--primary" disabled={!canAct || !draft.name.trim()}>{draft.kind === 'copy' ? '复制并打开' : '保存名称'}</button>
              <button type="button" className="editor-button" disabled={busy} onClick={() => setDraft(null)}>取消</button>
            </div>
          </form>}
          {archiveTarget && <div className="editor-library__form">
            <p>归档「{archiveTarget.name}」会保存世界并停止其后台推进。可随时从归档恢复。</p>
            <div className="editor-library__controls">
              <button type="button" className="editor-button" disabled={!canAct} onClick={() => void run({
                command: 'archive_world', payload: { worldId: archiveTarget.worldId, archived: true },
              })}>确认归档</button>
              <button type="button" className="editor-button" disabled={busy} onClick={() => setArchiveTarget(null)}>取消</button>
            </div>
          </div>}
          <div className="editor-library__catalog">
            <div>
              {available.length > 0 ? <ul className="editor-library__list">{available.map(worldRow)}</ul> : <p>还没有已保存的世界。</p>}
              {archived.length > 0 && <details className="editor-library__archived">
                <summary>查看归档（{archived.length}）</summary>
                <ul className="editor-library__list">{archived.map(worldRow)}</ul>
              </details>}
            </div>
            {selectedWorld && worldPreview(selectedWorld)}
          </div>
        </>}
        {library && shelf === 'presets' && preset && <form className="editor-library__form" onSubmit={(event) => {
          event.preventDefault();
          const name = newName.trim();
          if (name) void run({ command: 'create_world', payload: { presetId: preset.presetId, name } });
        }}>
          <label>主题与预设<select value={preset.presetId} onChange={(event) => setPresetId(event.target.value)}>
            {(['forest', 'pond'] as const).map((theme) => <optgroup key={theme} label={themeLabels[theme]}>
              {library.presets.filter((item) => item.theme === theme).map((item) => <option key={item.presetId} value={item.presetId}>
                {item.name} · {sourceLabels[item.source]}
              </option>)}
            </optgroup>)}
          </select></label>
          <p>{presetDescription(preset)}</p>
          <label>世界名称<input required maxLength={80} placeholder={preset.name} value={newName} onChange={(event) => setNewName(event.target.value)} /></label>
          <button type="submit" className="editor-button editor-button--primary" disabled={!canAct || !newName.trim()}>新建并打开</button>
        </form>}
        {shelf === 'presets' && Boolean(library?.presetWarnings?.length) && <details className="editor-library__archived">
          <summary>有 {library?.presetWarnings?.length} 个预设暂不可用</summary>
          {library?.presetWarnings?.map((message, index) => <p key={index}>{message}</p>)}
        </details>}
        {operationError && <p role="alert" className="editor-library__error">{operationError}</p>}
      </div>
    </dialog>
  );
}
