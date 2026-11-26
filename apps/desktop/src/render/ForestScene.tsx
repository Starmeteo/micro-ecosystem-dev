import { useEffect, useRef, useState } from 'react';
import type { ForestControls } from './pixiForest';
import type { ForestReadings, SeasonReadings } from '../world';
import './forest.css';

interface Point {
  readonly x: number;
  readonly y: number;
}

export interface ForestSceneSnapshot {
  readonly rock: Point;
  readonly animals: readonly (Point & {
    readonly id: string;
    readonly heading: number;
    readonly activity: string;
  })[];
  readonly daylightPhase: number;
  readonly forest?: Pick<ForestReadings, 'life'>;
  readonly season?: Pick<SeasonReadings, 'dayOfYear'>;
}

export interface ForestSceneProps {
  snapshot: ForestSceneSnapshot;
  density?: 1 | 2;
  active?: boolean;
  onRockMove?: (x: number, y: number) => void;
  onRockSelect?: () => void;
  rockSelected?: boolean;
  exactFit?: boolean;
  onReady?: (backend: string) => void;
}

export function ForestScene({
  snapshot, density = 2, active = true, onRockMove, onRockSelect, rockSelected = false, exactFit = false, onReady,
}: ForestSceneProps) {
  const host = useRef<HTMLDivElement>(null);
  const controls = useRef<ForestControls | null>(null);
  const latest = useRef({ snapshot, active, onRockMove, onRockSelect, rockSelected, onReady });
  const [error, setError] = useState('');
  const [ready, setReady] = useState(false);

  useEffect(() => {
    latest.current = { snapshot, active, onRockMove, onRockSelect, rockSelected, onReady };
    controls.current?.display(snapshot);
    controls.current?.setActive(active);
    controls.current?.setInteraction(Boolean(onRockMove), Boolean(onRockSelect), rockSelected);
  }, [snapshot, active, onRockMove, onRockSelect, rockSelected, onReady]);

  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const lifetime = new AbortController();
    let mounted: ForestControls | undefined;
    setError('');
    setReady(false);

    void import('./pixiForest')
      .then(({ mountForest }) => {
        if (lifetime.signal.aborted) return;
        return mountForest(element, density, lifetime.signal, (x, y) => latest.current.onRockMove?.(x, y),
          () => latest.current.onRockSelect?.(), exactFit);
      })
      .then((view) => {
        if (!view) return;
        if (lifetime.signal.aborted) {
          view.destroy();
          return;
        }
        mounted = view;
        controls.current = view;
        const current = latest.current;
        view.setInteraction(Boolean(current.onRockMove), Boolean(current.onRockSelect), current.rockSelected);
        view.display(current.snapshot);
        view.setActive(current.active);
        setReady(true);
        current.onReady?.(view.backend);
      })
      .catch((cause: unknown) => {
        if (lifetime.signal.aborted) return;
        setError(cause instanceof Error ? cause.message : String(cause));
        latest.current.onReady?.('unavailable');
      });

    return () => {
      lifetime.abort();
      if (controls.current === mounted) controls.current = null;
      mounted?.destroy();
    };
  }, [density, exactFit]);

  return (
    <div ref={host} className="forest-view">
      {!ready && !error && <p role="status" className="forest-view__error">正在加载林下画面</p>}
      {error && <p role="status" className="forest-view__error">林下画面加载失败 · {error}</p>}
    </div>
  );
}
