"""Author independent P07 surface mat and parted frond cels."""

from json import loads
from pathlib import Path

from PIL import Image, ImageDraw


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5'

MAT_STEMS = (
    (-12, 2), (-9, 5), (-6, 7), (-4, 5), (-2, 8), (0, 6),
    (3, 7), (5, 5), (8, 6), (10, 4), (12, 2),
)
FROND_STEMS = (
    (-20, 10), (-17, 15), (-13, 12), (-11, 19), (-7, 15), (-5, 21), (-1, 16),
    (3, 21), (6, 14), (8, 18), (11, 14), (15, 15), (20, 9),
)
PALETTE = ('#233c33', '#31544a', '#56745c', '#7c9265')
EXPOSED = ('#4e4937', '#696044', '#8e7952', '#a89162')


def make(spec, frond, frame):
    image = Image.new('RGBA', (spec['width'], spec['height']))
    draw = ImageDraw.Draw(image)
    cx, cy = spec['anchor']
    dry = frame == 'exposed'
    shades = EXPOSED if dry else PALETTE
    stems = FROND_STEMS if frond else MAT_STEMS

    def origin(x, y):
        return cx + x, cy + y

    def polygon(points, color):
        draw.polygon([origin(x, y) for x, y in points], fill=color)

    def line(points, color):
        draw.line([origin(x, y) for x, y in points], fill=color)

    spread = 12 if frond else 9
    if frame != 'full':
        spread -= 4
    polygon([(-spread, 0), (-spread+3, -2), (-3, -2), (0, -4), (6, -3),
             (spread, -1), (spread, 1), (0, 2), (-spread+3, 1)], shades[0])
    polygon([(-spread+2, 0), (-spread+5, -2), (1, -2), (4, -3),
             (spread-1, -1), (4, 0), (-2, 1)], shades[1])

    for index, (tip, height) in enumerate(stems):
        if frame == 'sparse' and index % 3 == 1:
            continue
        if dry and index % 3 == 0:
            continue
        span_factor = .8 if frame == 'sparse' else .72 if dry else 1
        height_factor = .66 if frame == 'sparse' else .39 if dry else 1
        tip = round(tip * span_factor)
        height = max(2, round(height * height_factor))
        start = round(tip * (.23 if frond else .48))
        middle = round(start + (tip - start) * .38)
        bend = round(start + (tip - start) * .78)
        line([(start, 0), (middle, -round(height * .42)), (bend, -round(height * .73)), (tip, -height)], shades[1])
        for step in (.37, .60, .79, .91):
            if frond and step < .4 and index % 2 == 0:
                continue
            x = round(start + (tip - start) * step)
            y = -round(height * step)
            side = -1 if index % 2 else 1
            length = 2 if frond and .48 < step < .80 else 1
            line([(x, y), (x + side, y - 1), (x + side * length, y - length)], shades[2])
            if frame == 'full' and index % 3 != 1:
                line([(x + side, y - 1), (x + side * length, y - length)], shades[3])
        if not dry or index % 2 == 0:
            draw.point(origin(tip, -height), fill=shades[3])
    return image


def main():
    manifest = loads((OUT / 'manifest.json').read_text(encoding='utf-8'))
    for shape, spec in manifest['pond']['attachedMoss'].items():
        counts = {}
        for frame, path in spec['sprites'].items():
            image = make(spec, shape == 'frond', frame)
            ax, ay = spec['anchor']
            alpha = image.getchannel('A')
            assert alpha.getpixel((ax, ay)) == 255
            counts[frame] = alpha.histogram()[255]
            image.save(OUT / path, optimize=True)
        assert counts['full'] > counts['sparse'] > counts['exposed'] > 0
        print(shape, 'independent brush cels with fixed alpha contact', counts)


if __name__ == '__main__':
    main()
