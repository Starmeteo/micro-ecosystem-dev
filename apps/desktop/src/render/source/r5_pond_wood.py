"""Paint P02 bed wood cels with separate footprint shadows and ornament."""

from json import loads
from pathlib import Path
from random import Random

from PIL import Image, ImageChops, ImageDraw

from forest_art import Cel, cluster


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5'


def wood_cels(spec, is_root):
    size = spec['width'], spec['height']
    ax, ay = spec['anchor']
    span, rise = spec['halfSpanPx'], spec['heightPx']
    landing = (spec['surfaceHalfSpanPx'] + 8) / span
    layers = {name: Image.new('RGBA', size) for name in ('shadow', 'back', 'front', 'growth', 'preview')}

    def at(x, y):
        return round(ax + x * span), round(ay + y)

    def polygon(layer, color, points):
        ImageDraw.Draw(layer).polygon([at(x, y) for x, y in points], fill=color)

    def streak(layer, color, points):
        ImageDraw.Draw(layer).line([at(x, y) for x, y in points], fill=color)

    polygon(layers['shadow'], (21, 32, 25, 73), (
        (-1.02, -5), (-.87, -10), (-.35, -7), (.53, -7), (1.02, -4), (1.02, 2), (.84, 6), (-.72, 7), (-1, 4),
    ))
    polygon(layers['shadow'], (19, 32, 26, 121), (
        (-.97, -3), (-.68, -5), (.25, -4), (.94, -3), (1, 0), (.73, 4), (-.73, 4),
    ))

    if is_root:
        outline = ((-1, -1), (-.94, -6), (-.78, -10), (-landing, -rise), (landing, -rise),
                   (.75, -11), (.82, -8), (1, -8),
                   (.94, -4), (.65, -3), (.38, -1), (.12, 1), (-.06, 0), (-.27, -3),
                   (-.50, 1), (-.76, 3), (-.89, 1))
        face = ((-1, -1), (-.83, -5), (-.61, -6), (-.54, -9), (-.48, -8), (-.23, -6),
                (.01, -5), (.33, -7), (.74, -8), (1, -8), (.94, -4), (.57, -3),
                (.28, 0), (.12, 1), (-.13, -2), (-.28, -4), (-.54, 1), (-.76, 3), (-.89, 1))
    else:
        outline = ((-1, -9), (-.98, -15), (-.91, -19), (-.74, -21), (-landing, -rise), (landing, -rise),
                   (.75, -15), (.81, -14), (.97, -17), (1, -16), (.96, -11), (.85, -10),
                   (.72, -7), (.59, -4), (.40, -5), (.16, -1), (-.11, -3), (-.33, 0),
                   (-.61, 2), (-.89, 2), (-.99, -3))
        face = ((-1, -9), (-.89, -10), (-.74, -9), (-.56, -11), (-.43, -8), (-.25, -9),
                (-.07, -8), (.12, -10), (.29, -10), (.44, -9), (.60, -11), (.83, -14),
                (.99, -16), (.96, -11), (.80, -9), (.56, -5), (.42, -5), (.17, -1),
                (-.13, -3), (-.36, 0), (-.66, 2), (-.90, 2), (-.99, -3))
    back = layers['back']
    polygon(back, '#4c3d2c', outline)
    polygon(back, '#795e42', [
        (-.96, -rise * .48), (-.86, -rise * .81), (-.59, -rise + 2), (-.32, -rise + 4),
        (-.10, -rise + 5), (.18, -rise + 4), (.30, -rise + 3), (.59, -rise + 7),
        (.79, -rise + 8), (.92, -rise + 7), (.82, -rise * .38), (.40, -rise * .31),
        (.16, -rise * .49), (-.13, -rise * .42), (-.44, -rise * .51), (-.76, -rise * .55),
    ])
    silhouette = Image.new('RGBA', size)
    polygon(silhouette, '#ffffff', outline)
    rng = Random(401 if is_root else 1007)
    for index in range(17 if is_root else 28):
        x = rng.uniform(-.91, .75)
        y = rng.uniform(-rise * .89, -rise * .37)
        length = rng.uniform(.035, .19)
        tone = ('#5d4832', '#927350', '#9d8056', '#694f36')[index % 4]
        streak(back, tone, [(x, y), (x + length * .6, y - .6), (x + length, y)])
    back.putalpha(silhouette.getchannel('A'))
    # The flat contact rim spans even the outer pixels of a resting shrimp.
    for x in range(round(ax - landing * span), round(ax + landing * span) + 1):
        assert back.getpixel((x, ay - rise))[-1] == 255
        if (x // 3) % 4 != 1:
            ImageDraw.Draw(back).point((x, ay - rise), fill='#7e6749')

    front = layers['front']
    polygon(front, '#3d3225', face)
    front_alpha = front.getchannel('A')
    streak(front, '#987349', face[:11])
    for index in range(10 if is_root else 17):
        x = rng.uniform(-.86, .73)
        y = rng.uniform(-rise * .46, -.8)
        level = ('#674e35', '#74593d', '#53402c')[index % 3]
        streak(front, level, [(x, y), (x + .11, y), (x + .16, y - 1)])
    if not is_root:
        for radius, tone in ((7, '#ae8b5b'), (5, '#c19d67'), (3, '#6d573c')):
            ImageDraw.Draw(front).ellipse((ax-span+2, ay-radius-5, ax-span+radius, ay+radius-5), outline=tone)
    front.putalpha(ImageChops.multiply(front_alpha, silhouette.getchannel('A')))

    growth = layers['growth']
    ornament = ImageDraw.Draw(growth)
    for index in range(31 if is_root else 48):
        bx = -.34 if is_root else -.43
        x = bx + rng.uniform(-.18, .20)
        y = -rise * .73 + rng.uniform(-3, 3)
        px, py = at(x, y)
        color = ('#465b43', '#59684a', '#748061', '#82916b')[index % 4]
        ornament.line([(px, py + 1), (px + rng.choice((-1, 0, 1)), py - 1)], fill=color)
    growth.putalpha(ImageChops.multiply(growth.getchannel('A'), silhouette.getchannel('A')))
    for name in ('shadow', 'back', 'growth', 'front'):
        layers['preview'].alpha_composite(layers[name])
    x0, y0, x1, _ = back.getbbox(alpha_only=True)
    assert (x1 - x0 - 1, ay - y0) == (span * 2, rise)
    assert growth.getbbox() and front.getbbox()
    return layers


def fixed_bed_film():
    """Keep the inherited right-bed details without patches drawn on the old branch."""
    cel = Cel(1)
    for x, y, w, h in ((341, 153, 16, 6), (365, 153, 26, 8), (384, 160, 14, 5), (399, 171, 13, 5)):
        cluster(cel, x, y, w, h, ('#415b42', '#657f4e', '#a1aa62'), x * y, 18)
    return cel.image


def main():
    manifest = loads((OUT / 'manifest.json').read_text(encoding='utf-8'))
    wood = manifest['pond']['bottomWoods']
    fixed_bed_film().save(OUT / wood['bedBiofilm'], optimize=True)
    for name in ('branch', 'root'):
        spec = wood[name]
        for part, image in wood_cels(spec, name == 'root').items():
            image.save(OUT / spec['sprites'][part], optimize=True)
        print(name, 'fixed footprint, five separately rendered sprite files')


if __name__ == '__main__':
    main()
