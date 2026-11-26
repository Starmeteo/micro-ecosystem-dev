import { useEffect, useRef, useState } from 'react';
import {
  R5SampleScene, type R5SampleDensity, type R5SampleFocus, type R5SampleTheme, type R5SampleZoom,
} from '../../render/r5/R5SampleScene';
import {
  R5DensityControl, R5ObservationControls, R5PreviewHeader, R5ReferenceNote, useR5PreviewTheme, type R5PreviewProps,
} from './R5PreviewControls';
import './R5Preview.css';

type Tab = 'object' | 'environment' | 'biology';
type Task = Tab | 'scene' | 'content';
type SampleObject = 'stone' | 'log';
type Placement = { object: SampleObject; x: number; y: number };

const tabs = [
  { id: 'object', label: '属性', path: 'M4 5h16v14H4z M8 9h8 M8 15h5 M11 7v4 M14 13v4' },
  { id: 'environment', label: '环境', path: 'M3 17q3-3 6 0t6 0t6 0 M3 11q3-3 6 0t6 0t6 0 M3 5q3-3 6 0t6 0t6 0' },
  { id: 'biology', label: '生物', path: 'M20 4c-9 0-15 4-15 10a5 5 0 0 0 5 5c6 0 10-6 10-15Z M4 20l11-11' },
] as const;

const content = {
  forest: {
    stone: { name: '苔斑岩石', description: '观察石底、草叶与地面的接触' },
    log: { name: '朽木段', description: '观察树皮前缘遮挡与接地点' },
  },
  pond: {
    stone: { name: '水底主石', description: '观察虾足与岩面的接触' },
    log: { name: '沉木', description: '观察根须、停驻面与水下遮挡' },
  },
} as const;

const biology = {
  forest: [
    { name: '鼠妇', detail: '主体 12 mm，触须另计', state: '姿态样板' },
    { name: '弹尾虫', detail: '主体 1.5 mm，微点仅代表群体', state: '群体示意' },
    { name: '小型陆螺', detail: '壳宽 6 mm，待确定类群', state: '候选' },
  ],
  pond: [
    { name: '米虾', detail: '眼后缘至尾节 20 mm', state: '姿态样板' },
    { name: '水蚤', detail: '躯体 2 mm，微点仅代表群体', state: '群体示意' },
    { name: '小型淡水螺', detail: '壳宽 8 mm，待确定类群', state: '候选' },
  ],
} as const;

const objectKeys = ['stone', 'log'] as const;

export function R5Workbench(props: R5PreviewProps) {
  const { theme, selectTheme } = useR5PreviewTheme(props);
  const [tab, setTab] = useState<Tab>('object');
  const [task, setTask] = useState<Task>('scene');
  const [drawer, setDrawer] = useState(false);
  const [zoom, setZoom] = useState<R5SampleZoom>(1);
  const [focus, setFocus] = useState<R5SampleFocus>('center');
  const [density, setDensity] = useState<R5SampleDensity>(480);
  const [showScale, setShowScale] = useState(false);
  const [pose, setPose] = useState<0 | 1>(0);
  const [waterLevel, setWaterLevel] = useState(.65);
  const [selectedObject, setSelectedObject] = useState<SampleObject | null>(null);
  const [positions, setPositions] = useState<Partial<Record<SampleObject, Placement>>>({});
  const [resetKey, setResetKey] = useState(0);
  const [reduceTransparency, setReduceTransparency] = useState(false);
  const [reduceMotion, setReduceMotion] = useState(false);
  const scene = useRef<HTMLDivElement>(null);

  const selected = selectedObject && content[theme][selectedObject];
  const placement = selectedObject && positions[selectedObject];
  const canReset = Boolean(selectedObject || Object.keys(positions).length);
  const changeTheme = (next: R5SampleTheme) => {
    selectTheme(next);
    setPose(0);
    setZoom(1);
    setFocus('center');
  };

  useEffect(() => {
    // An externally selected theme also resets transient placement coordinates.
    setSelectedObject(null);
    setPositions({});
  }, [theme]);

  useEffect(() => {
    if (selectedObject && task === 'scene') scene.current?.querySelector('canvas')?.focus({ preventScroll: true });
  }, [selectedObject, task]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || !canReset) return;
      event.preventDefault();
      setSelectedObject(null);
      setPositions({});
      setResetKey((key) => key + 1);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [canReset]);

  const resetPlacement = () => {
    setSelectedObject(null);
    setPositions({});
    setResetKey((key) => key + 1);
  };

  const previewPlacement = (key: SampleObject) => {
    setSelectedObject(key);
    setTab('object');
    setTask('scene');
    setZoom(1);
    setFocus('center');
  };

  const toggleDrawer = () => {
    const compact = window.matchMedia('(max-width: 780px), (max-height: 500px)').matches;
    const open = compact ? task !== 'content' : !drawer;
    setDrawer(open);
    setTask(open ? 'content' : 'scene');
  };

  return (
    <main className="r5-frame r5-workbench" data-task={task} data-drawer={drawer}
      data-reduce-transparency={reduceTransparency} data-reduce-motion={reduceMotion}>
      <R5PreviewHeader title="生态编辑工作台" theme={theme} onThemeChange={changeTheme} onHide={props.onHide} />
      <div className="r5-workspace">
        <nav className="r5-tool-rail" aria-label="工作台工具">
          <button className="r5-tool r5-return-scene" type="button" aria-pressed={task === 'scene'}
            onClick={() => setTask('scene')}>
            场景
          </button>
          {tabs.map(({ id, label, path }) => (
            <button key={id} className="r5-tool" type="button"
              data-wide-selected={tab === id} data-task-selected={task === id}
              aria-controls="r5-workbench-inspector"
              onClick={() => { setTab(id); setTask(id); }}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
                <path d={path} strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              {label}
            </button>
          ))}
          <button className="r5-tool r5-tool--content" type="button" aria-controls="r5-workbench-content"
            data-wide-selected={drawer} data-task-selected={task === 'content'} onClick={toggleDrawer}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
              <path d="M4 7l8-4 8 4-8 4-8-4Z M4 7v10l8 4 8-4V7 M12 11v10" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            内容
          </button>
        </nav>

        <section className="r5-viewport" aria-label="样板场景">
          <div className="r5-scene-area" ref={scene}>
            <R5SampleScene
              className="r5-sample-view"
              theme={theme} density={density} zoom={zoom} focus={focus} pose={pose} waterLevel={waterLevel}
              showScale={showScale} editPreview={selectedObject !== null} selectedObject={selectedObject} resetKey={resetKey}
              onPlacement={(next) => setPositions((current) => ({ ...current, [next.object]: next }))}
            />
          </div>
          <div className="r5-viewport__controls">
            <R5ObservationControls {...{ zoom, focus, showScale }}
              onZoomChange={setZoom} onFocusChange={setFocus} onScaleChange={setShowScale} />
          </div>
          {selected && (
            <div className="r5-placement" role="status">
              <span>{selected.name} · {placement ? `样板位置 (${placement.x.toFixed(0)}, ${placement.y.toFixed(0)})` : '拖动或聚焦画面按方向键'}</span>
              <button className="r5-button" type="button" onClick={resetPlacement}>撤回试摆</button>
            </div>
          )}
        </section>

        <aside id="r5-workbench-inspector" className="r5-inspector" aria-label="工作台检查器">
          {tab === 'object' && (
            <section aria-labelledby="r5-object-heading">
              <h2 id="r5-object-heading">对象属性</h2>
              {selected ? (
                <>
                  <h3>{selected.name}</h3>
                  <p>{selected.description}。</p>
                  {placement && <dl className="r5-facts">
                    <div><dt>样板画面 X</dt><dd>{placement.x.toFixed(0)} px</dd></div>
                    <div><dt>样板画面 Y</dt><dd>{placement.y.toFixed(0)} px</dd></div>
                  </dl>}
                </>
              ) : <p>从「内容」选择石块或木段，然后在场景试摆。</p>}
              {selected && <button className="r5-button r5-secondary-action" type="button" onClick={resetPlacement}>恢复初始位置</button>}
              <p className="r5-detail-note">试摆只影响样板。按 Esc 恢复石木的位置。</p>
            </section>
          )}

          {tab === 'environment' && (
            <section aria-labelledby="r5-environment-heading">
              <h2 id="r5-environment-heading">场景环境</h2>
              <p>固定参考平面。环境和个体共用画面标尺。</p>
              <dl className="r5-facts">
                <div><dt>环境读数</dt><dd>样板未连接生态模型</dd></div>
              </dl>
              {theme === 'pond' && (
                <label className="r5-range">
                  <span>水线高度预览 · {Math.round(waterLevel * 100)}%</span>
                  <input aria-label="水线高度预览" type="range" min="0" max="100" step="1"
                    value={Math.round(waterLevel * 100)} onChange={(event) => setWaterLevel(event.target.valueAsNumber / 100)} />
                  <small>观察淹没与露出。百分比表示画面控件的位置。</small>
                </label>
              )}
              <R5DensityControl value={density} onChange={setDensity} />
              <div className="r5-preferences">
                <label><input type="checkbox" checked={reduceTransparency}
                  onChange={(event) => setReduceTransparency(event.target.checked)} />减少透明</label>
                <label><input type="checkbox" checked={reduceMotion}
                  onChange={(event) => setReduceMotion(event.target.checked)} />减少动态</label>
              </div>
            </section>
          )}

          {tab === 'biology' && (
            <section aria-labelledby="r5-biology-heading">
              <h2 id="r5-biology-heading">生物观察</h2>
              <ul className="r5-specimens" aria-label="生物观察口径">
                {biology[theme].map(({ name, detail, state }) => (
                  <li key={name}><strong>{name}</strong><span>{state}</span><small>{detail}</small></li>
                ))}
              </ul>
              <button className="r5-button r5-secondary-action" type="button" onClick={() => {
                setFocus('animal');
                setZoom(2);
                setTask('scene');
              }}>放大观察</button>
              <button className="r5-button r5-secondary-action" type="button" onClick={() => {
                setPose(pose === 0 ? 1 : 0);
                if (zoom > 1) setFocus('animal');
                setTask('scene');
              }}>{theme === 'pond' ? (pose === 0 ? '对照离底' : '对照贴底') : (pose === 0 ? '对照遮挡' : '对照贴地')}</button>
              <p className="r5-detail-note">姿态由样板切换。候选类群只用于尺度与形态评审。</p>
            </section>
          )}
        </aside>

        <section id="r5-workbench-content" className="r5-drawer" aria-labelledby="r5-content-heading">
          <header>
            <h2 id="r5-content-heading">可试摆物件</h2>
            <button className="r5-button" type="button" onClick={() => {
              setDrawer(false);
              setTask('scene');
            }}>收起内容</button>
          </header>
          <div className="r5-content-options">
            {objectKeys.map((key) => {
              const object = content[theme][key];
              return (
                <button className="r5-content-option" type="button" key={key} aria-pressed={selectedObject === key}
                  onClick={() => previewPlacement(key)}>
                  <strong>{object.name}</strong>
                  <span>{object.description}</span>
                </button>
              );
            })}
          </div>
        </section>
      </div>
      <footer className="r5-footer">
        <R5ReferenceNote theme={theme} density={density}><span>外观与试摆预览</span></R5ReferenceNote>
      </footer>
    </main>
  );
}
