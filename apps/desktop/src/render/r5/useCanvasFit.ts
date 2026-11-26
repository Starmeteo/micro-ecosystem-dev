import { useLayoutEffect, useState } from 'react';
import { fitSample } from './geometry';

export function useCanvasFit(host: { current: HTMLDivElement | null }) {
  const [fit, setFit] = useState(() => fitSample(0, 0, 1));
  useLayoutEffect(() => {
    const target = host.current;
    if (!target) return;

    let resolution: MediaQueryList | undefined;
    function fitTarget() {
      const box = target!.getBoundingClientRect();
      const next = fitSample(box.width, box.height, window.devicePixelRatio || 1);
      setFit((previous) => previous.cssWidth === next.cssWidth && previous.cssHeight === next.cssHeight
        && previous.devicePixelRatio === next.devicePixelRatio ? previous : next);
    }
    function watchResolution() {
      resolution?.removeEventListener('change', watchResolution);
      resolution = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
      resolution.addEventListener('change', watchResolution);
      fitTarget();
    }
    const resize = new ResizeObserver(fitTarget);
    resize.observe(target);
    watchResolution();
    return () => {
      resize.disconnect();
      resolution?.removeEventListener('change', watchResolution);
    };
  }, [host]);
  return fit;
}
