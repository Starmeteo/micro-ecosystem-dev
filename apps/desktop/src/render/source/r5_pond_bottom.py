"""Export P01's opt-in mineral bed plus separate pebble and shadow."""

from json import loads
from pathlib import Path
from random import Random

from PIL import Image, ImageDraw


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5'


def bed_grit(size):
    image = Image.new('RGBA', size)
    pen = ImageDraw.Draw(image)
    rng = Random(604)
    colors = ('#82836a', '#938365', '#a29474', '#726f5d')
    # Color flecks and thin chips keep the same ground material without large rock silhouettes.
    for index in range(95):
        x = round(rng.uniform(18, 463))
        y = round(rng.uniform(140, 267))
        if 175 < x < 316 and 148 < y < 190 and index % 3:
            continue
        span = round(rng.uniform(1, 4) * (.66 + (y - 140) / 185))
        pen.polygon([(x - span, y), (x - 1, y - 1), (x + span - 1, y - 1),
                     (x + span, y), (x + 1, y + 1)], fill=colors[index % len(colors)])
    return image


def pebble():
    image = Image.new('RGBA', (28, 21))
    pen = ImageDraw.Draw(image)
    pen.polygon([(4, 14), (5, 10), (9, 7), (13, 6), (17, 6), (20, 8),
                 (23, 11), (24, 14), (20, 17), (12, 18), (7, 17)], fill='#3b4942')
    pen.polygon([(5, 13), (7, 10), (11, 8), (15, 7), (19, 8), (22, 11),
                 (19, 14), (14, 14), (10, 15), (6, 15)], fill='#64776a')
    pen.polygon([(8, 11), (11, 8), (15, 7), (18, 8), (19, 9), (14, 10),
                 (11, 10), (9, 12)], fill='#88927d')
    pen.line([(11, 9), (14, 8), (16, 8)], fill='#a2a18a')
    pen.polygon([(23, 11), (23, 14), (20, 16), (15, 16), (18, 14)], fill='#516458')
    shadow = Image.new('RGBA', image.size)
    ink = ImageDraw.Draw(shadow)
    ink.ellipse((2, 12, 26, 20), fill=(24, 38, 31, 55))
    ink.ellipse((5, 13, 23, 19), fill=(25, 39, 30, 104))
    ink.ellipse((7, 14, 20, 18), fill=(22, 34, 28, 139))
    return image, shadow


def main():
    manifest = loads((OUT / 'manifest.json').read_text(encoding='utf-8'))
    canvas = manifest['canvas']
    spec = manifest['pond']['bottomStones']
    ground = bed_grit((canvas['width'], canvas['height']))
    assert ground.getchannel('A').getextrema() == (0, 255)
    stone, shadow = pebble()
    rock = spec['pebble']
    x0, y0, x1, _ = stone.getbbox(alpha_only=True)
    assert (x1 - x0 - 1, rock['anchor'][1] - y0) == (2 * rock['halfSpanPx'], rock['heightPx'])
    assert shadow.getchannel('A').getextrema()[0] == 0
    for image, key in ((ground, spec['bedGrit']), (stone, rock['rock']), (shadow, rock['shadow'])):
        image.save(OUT / key, optimize=True)
    print('Exported independent opt-in grit, 25 mm pebble, and its contact shadow.')


if __name__ == '__main__':
    main()
