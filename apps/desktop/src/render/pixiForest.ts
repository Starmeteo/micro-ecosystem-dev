import { Application, Container, Graphics, Rectangle, Sprite, Texture } from 'pixi.js';
import 'pixi.js/unsafe-eval';
import type { ForestSceneSnapshot } from './ForestScene';
import { seasonColor } from './seasonColor';

const urls = import.meta.glob<string>('./assets/*/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
});

const backdropLayers = ['sky', 'trees', 'soil', 'wood'] as const;
const layers = [...backdropLayers, 'moss', 'foliage', 'ferns', 'litter', 'front', 'light', 'contact', 'rock',
  'woodlouse-0', 'woodlouse-1', 'woodlouse-2', 'woodlouse-shelter'] as const;

type Layer = typeof layers[number];

export interface ForestControls {
  backend: string;
  display(snapshot: ForestSceneSnapshot): void;
  setActive(value: boolean): void;
  setInteraction(editable: boolean, selectable: boolean, selected: boolean): void;
  destroy(): void;
}

type PointerPreview = {
  pointerId: number;
  start: { x: number; y: number };
  offset: { x: number; y: number };
  position: { x: number; y: number };
};

function bounded(value: number) {
  return Math.min(1, Math.max(0, value));
}

function blendColor(night: readonly number[], day: readonly number[], amount: number) {
  return night.reduce((color, channel, index) =>
    (color << 8) | Math.round(channel + (day[index] - channel) * amount), 0);
}

function stressedTint(tint: number, stress: number) {
  const fade = bounded(stress);
  const red = Math.round((tint >> 16) * (1 - fade * .08));
  const green = Math.round((tint >> 8 & 255) * (1 - fade * .36));
  const blue = Math.round((tint & 255) * (1 - fade * .28));
  return (red << 16) | (green << 8) | blue;
}

export async function mountForest(
  host: HTMLElement,
  density: 1 | 2,
  signal: AbortSignal,
  moveRock: (x: number, y: number) => void,
  selectRock: () => void,
  exactFit: boolean,
): Promise<ForestControls | undefined> {
  // Decode the local PNGs directly. Pixi's Assets worker handshake can stay
  // pending when the packaged WebView's CSP refuses blob workers.
  const decoded = await Promise.all(layers.map(async (name) => {
    const url = urls['./assets/d' + density + '/' + name + '.png'];
    if (!url) throw new Error('Missing forest texture: ' + name);
    const image = new Image();
    image.src = url;
    await image.decode();
    return [name, image] as const;
  }));
  if (signal.aborted) return;

  const width = 240 * density;
  const height = 135 * density;
  const app = new Application();
  await app.init({
    width,
    height,
    preference: 'webgl',
    autoStart: false,
    sharedTicker: false,
    background: 0x121d17,
    antialias: false,
    resolution: 1,
  });
  if (signal.aborted) {
    app.destroy({ removeView: true }, { children: true });
    return;
  }

  const textures = Object.fromEntries(decoded.map(([name, image]) => {
    const texture = Texture.from(image, true);
    texture.source.scaleMode = 'nearest';
    return [name, texture];
  })) as Record<Layer, Texture>;
  const canvas = app.canvas as HTMLCanvasElement;
  const stage = new Container();
  const scenery = new Container();
  const objects = new Container();
  objects.sortableChildren = true;
  const front = new Sprite(textures.front);
  const light = new Sprite(textures.light);
  const shadow = new Sprite(textures.contact);
  const rock = new Sprite(textures.rock);
  const mark = new Graphics();
  mark.moveTo(-23, -16).lineTo(-23, -24).lineTo(-15, -24)
    .moveTo(15, -24).lineTo(23, -24).lineTo(23, -16)
    .moveTo(23, -1).lineTo(23, 7).lineTo(15, 7)
    .moveTo(-15, 7).lineTo(-23, 7).lineTo(-23, -1)
    .stroke({ color: 0xe3efb9, width: 1.2, alpha: .95 });
  mark.scale.set(density);
  mark.visible = false;
  shadow.anchor.set(0.5, 46 / 56);
  rock.anchor.copyFrom(shadow.anchor);

  const backdrop = backdropLayers.map((name) => new Sprite(textures[name]));
  const legacyMoss = new Sprite(textures.moss);
  const legacyFoliage = new Sprite(textures.foliage);
  const litter = new Sprite(textures.litter);
  const patchTextures: Texture[] = [];

  function splitPatches(name: 'ferns' | 'moss') {
    return ([0, 1] as const).map((side) => {
      const x = side * width / 2;
      const texture = new Texture({
        source: textures[name].source,
        frame: new Rectangle(x, 0, width / 2, height),
      });
      patchTextures.push(texture);
      const sprite = new Sprite(texture);
      sprite.x = x;
      sprite.visible = false;
      return sprite;
    });
  }

  const mossPatches = splitPatches('moss');
  const fernPatches = splitPatches('ferns');
  const tinted = [...backdrop, front, rock, legacyMoss, legacyFoliage, litter];
  litter.visible = false;
  scenery.addChild(...backdrop, legacyMoss, ...mossPatches, legacyFoliage, ...fernPatches, litter);
  objects.addChild(rock);
  stage.addChild(scenery, shadow, objects, front, light, mark);
  app.stage.addChild(stage);

  const sprites = new Map<string, Sprite>();
  const gait = [textures['woodlouse-0'], textures['woodlouse-1'], textures['woodlouse-2']];
  const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
  let snapshot: ForestSceneSnapshot | undefined;
  let active = false;
  let editable = false;
  let selectable = false;
  let drag: PointerPreview | undefined;
  let destroyed = false;
  let lastFrame = -1;

  function locateRock() {
    const position = drag?.position ?? snapshot?.rock;
    if (!position) return;
    rock.position.set(Math.round(position.x * width), Math.round(position.y * height));
    shadow.position.copyFrom(rock.position);
    mark.position.copyFrom(rock.position);
    rock.zIndex = rock.y;
  }

  function display(next: ForestSceneSnapshot) {
    snapshot = next;
    locateRock();
    const phase = next.daylightPhase;
    const elevation = Math.cos((phase - .5) * 2 * Math.PI);
    const daylight = bounded((elevation + .17) / .65);
    const warmth = Math.max(0, 1 - Math.abs(elevation - .24) / .36) * .38;
    const [red, green, blue] = seasonColor(next.season?.dayOfYear, 'light');
    const tint = blendColor(
      [111, 135, 178],
      [red, green - warmth * 31, blue - warmth * 89],
      daylight,
    );
    for (const sprite of tinted) sprite.tint = tint;
    const life = next.forest?.life;
    legacyMoss.visible = !life;
    legacyFoliage.visible = !life;
    litter.visible = Boolean(life);
    const [leafRed, leafGreen, leafBlue] = seasonColor(next.season?.dayOfYear, 'foliage');
    const leafTint = blendColor([111, 135, 178], [leafRed, leafGreen - warmth * 23, leafBlue - warmth * 44], daylight);
    for (const side of [0, 1] as const) {
      const fern = fernPatches[side];
      const moss = mossPatches[side];
      fern.visible = Boolean(life);
      moss.visible = Boolean(life);
      if (life) {
        // These are cover cues for the two aggregate model regions. They
        // never change physical fern/moss placement or habitat availability.
        fern.alpha = Math.sqrt(bounded(life.fernMgC[side] / 90));
        moss.alpha = Math.sqrt(bounded(life.mossMgC[side] / 192));
        const foliageTint = next.season ? leafTint : tint;
        fern.tint = stressedTint(foliageTint, life.fernStressFraction[side]);
        moss.tint = stressedTint(foliageTint, life.mossStressFraction[side]);
      }
    }
    light.alpha = daylight * (.61 + warmth);
    shadow.alpha = .7 + daylight * .3;

    const juveniles = life && new Set(life.woodlice.filter(({ stage }) => stage === 'juvenile').map(({ id }) => id));
    const newIds = new Set<string>();
    for (const animal of next.animals) {
      newIds.add(animal.id);
      let sprite = sprites.get(animal.id);
      if (!sprite) {
        sprite = new Sprite(gait[0]);
        sprite.anchor.set(.5, 17 / 26);
        objects.addChild(sprite);
        sprites.set(animal.id, sprite);
      }
      sprite.position.set(Math.round(animal.x * width), Math.round(animal.y * height));
      sprite.zIndex = sprite.y;
      const facing = Math.cos(animal.heading) >= 0 ? 1 : -1;
      const size = juveniles?.has(animal.id) ? .66 : 1;
      sprite.scale.set(facing * size, size);
      sprite.rotation = Math.round(Math.sin(animal.heading) * facing * 2) * Math.PI / 12;
      sprite.tint = tint;
    }

    for (const [id, sprite] of sprites) {
      if (newIds.has(id)) continue;
      sprites.delete(id);
      sprite.destroy();
    }
    animate();
  }

  function animate() {
    if (!snapshot) return;
    const frame = motion.matches ? 0 : Math.floor(performance.now() / 115) % gait.length;
    for (const [index, animal] of snapshot.animals.entries()) {
      const sprite = sprites.get(animal.id);
      if (!sprite) continue;
      const activity = animal.activity.toLowerCase();
      if (activity === 'sheltering' || activity === 'shelter' || activity === 'hidden') {
        sprite.texture = textures['woodlouse-shelter'];
      } else {
        const moving = activity === 'moving' || activity === 'move'
          || activity === 'foraging' || activity === 'feeding';
        sprite.texture = moving ? gait[(frame + index) % gait.length] : gait[0];
      }
    }
  }

  // The artwork is composed in an authored pixel grid. Render directly at
  // physical-pixel size so nearest-neighbor sampling also applies at 125%/150%.
  function fit() {
    const pixelRatio = window.devicePixelRatio || 1;
    const fitScale = Math.min(host.clientWidth * pixelRatio / width, host.clientHeight * pixelRatio / height);
    if (fitScale <= 0) return;
    const whole = Math.floor(fitScale);
    const scale = !exactFit && whole >= 1 && whole / fitScale >= .82 ? whole : fitScale;
    const fittedWidth = Math.max(1, Math.floor(width * scale));
    const fittedHeight = Math.max(1, Math.floor(height * scale));
    canvas.style.width = (fittedWidth / pixelRatio) + 'px';
    canvas.style.height = (fittedHeight / pixelRatio) + 'px';
    app.renderer.resize(fittedWidth, fittedHeight);
    stage.scale.set(scale);
    if (snapshot) app.render();
  }

  const observer = new ResizeObserver(fit);
  host.appendChild(canvas);
  observer.observe(host);
  let dpiQuery: MediaQueryList | undefined;
  function observeDpi() {
    dpiQuery?.removeEventListener('change', observeDpi);
    dpiQuery = window.matchMedia('(resolution: ' + window.devicePixelRatio + 'dppx)');
    dpiQuery.addEventListener('change', observeDpi);
    fit();
  }
  observeDpi();

  app.ticker.maxFPS = 36;
  app.ticker.add(() => {
    const frame = Math.floor(performance.now() / 115);
    if (frame !== lastFrame) {
      lastFrame = frame;
      animate();
    }
  });

  function pointerPosition(event: PointerEvent) {
    const rectangle = canvas.getBoundingClientRect();
    return {
      x: (event.clientX - rectangle.left) / rectangle.width,
      y: (event.clientY - rectangle.top) / rectangle.height,
    };
  }

  function isOverRock(position: { x: number; y: number }) {
    if (!snapshot) return false;
    const dx = (position.x - snapshot.rock.x) * 480;
    const dy = (position.y - snapshot.rock.y) * 270;
    return dx >= -37 && dx <= 37 && dy >= -40 && dy <= 10;
  }

  function clearPreview() {
    const pointerId = drag?.pointerId;
    drag = undefined;
    if (pointerId !== undefined && canvas.hasPointerCapture(pointerId)) {
      canvas.releasePointerCapture(pointerId);
    }
    canvas.style.cursor = '';
    locateRock();
  }

  canvas.addEventListener('pointerdown', (event) => {
    if ((!editable && !selectable) || !snapshot || event.button !== 0 || drag) return;
    const start = pointerPosition(event);
    if (!isOverRock(start)) return;
    event.preventDefault();
    canvas.focus({ preventScroll: true });
    selectRock();
    if (!editable) return;
    canvas.setPointerCapture(event.pointerId);
    drag = {
      pointerId: event.pointerId,
      start,
      offset: { x: snapshot.rock.x - start.x, y: snapshot.rock.y - start.y },
      position: snapshot.rock,
    };
    canvas.style.cursor = 'grabbing';
  });

  canvas.addEventListener('pointermove', (event) => {
    if (!editable && !selectable) return;
    const position = pointerPosition(event);
    if (!drag) {
      canvas.style.cursor = isOverRock(position) ? (editable ? 'grab' : 'pointer') : '';
      return;
    }
    if (event.pointerId !== drag.pointerId) return;
    drag.position = {
      x: bounded(position.x + drag.offset.x),
      y: bounded(position.y + drag.offset.y),
    };
    locateRock();
  });

  canvas.addEventListener('pointerup', (event) => {
    if (event.pointerId !== drag?.pointerId) return;
    const position = pointerPosition(event);
    const moved = Math.abs(position.x - drag.start.x) * width > 1
      || Math.abs(position.y - drag.start.y) * height > 1;
    const destination = {
      x: bounded(position.x + drag.offset.x),
      y: bounded(position.y + drag.offset.y),
    };
    clearPreview();
    if (moved) moveRock(destination.x, destination.y);
  });

  for (const type of ['pointercancel', 'lostpointercapture'] as const) {
    canvas.addEventListener(type, (event) => {
      if (event.pointerId === drag?.pointerId) clearPreview();
    });
  }

  canvas.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && drag) {
      event.preventDefault();
      clearPreview();
      return;
    }
    if (selectable && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault();
      selectRock();
      return;
    }
    if (!editable || drag || !snapshot) return;
    const direction = {
      ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0],
    }[event.key];
    if (!direction) return;
    event.preventDefault();
    moveRock(bounded(snapshot.rock.x + direction[0] * .015), bounded(snapshot.rock.y + direction[1] * .015));
  });

  return {
    backend: app.renderer.name.toLowerCase(),
    display,
    setActive(value) {
      if (active === value) return;
      active = value;
      if (active) {
        app.render();
        app.start();
        return;
      }
      clearPreview();
      app.stop();
    },
    setInteraction(canEdit, canSelect, selected) {
      const changed = mark.visible !== selected;
      mark.visible = selected;
      if (editable !== canEdit || selectable !== canSelect || !canvas.hasAttribute('role')) {
        editable = canEdit;
        selectable = canSelect;
        canvas.setAttribute('role', editable || selectable ? 'group' : 'img');
        canvas.setAttribute('aria-label', editable ? '林下场景，拖动石头或按方向键移动'
          : selectable ? '林下场景，按回车选择石头' : '林下像素场景');
        canvas.tabIndex = editable || selectable ? 0 : -1;
        canvas.style.touchAction = editable ? 'none' : '';
        if (!editable) clearPreview();
      }
      if (changed && !active) app.render();
    },
    destroy() {
      if (destroyed) return;
      destroyed = true;
      observer.disconnect();
      dpiQuery?.removeEventListener('change', observeDpi);
      app.destroy({ removeView: true }, { children: true });
      for (const texture of patchTextures) texture.destroy();
      for (const texture of Object.values(textures)) texture.destroy(true);
    },
  };
}
