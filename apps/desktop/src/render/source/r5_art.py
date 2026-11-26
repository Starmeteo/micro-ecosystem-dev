"""Rebuild R5 sample PNG layers with Python 3 and Pillow."""

from argparse import ArgumentParser
from json import loads
from math import dist
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw

from r5_forest import render_forest
from r5_pond import render_pond


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5'
NEAREST = Image.Resampling.NEAREST


def transparent_rgb_cleared(image):
    clean = Image.new('RGBA', image.size)
    clean.alpha_composite(image)
    return clean


def verify(theme, contract, images, canvas):
    layers = contract['layers']
    assigned = set()
    for key, path in layers.items():
        assert images[path].size == canvas, path
        if key == 'base':
            assert images[path].getchannel('A').getextrema() == (255, 255), path
        else:
            assert images[path].getchannel('A').getextrema()[0] == 0, path
        assigned.add(path)
    for name, spec in contract['objects'].items():
        size = spec['width'], spec['height']
        axes = spec['body']
        length = spec['lengthMm'] * canvas[0] / contract['referenceWidthMm']
        assert abs(dist(*axes) - length) <= 1, (theme, name, axes, length)
        anchors = [spec['anchor'], *axes, *spec['contact']]
        anchors.extend(spec[key] for key in ('perch', 'cover') if key in spec)
        assert all(0 <= x < size[0] and 0 <= y < size[1] for x, y in anchors), (theme, name, anchors)
        for path in spec['sprites'].values():
            sprite = images[path]
            assert sprite.size == size and sprite.mode == 'RGBA', path
            alpha = sprite.getchannel('A')
            assert alpha.getextrema()[0] == 0 and alpha.getbbox(), path
            assigned.add(path)
    assert assigned == {path for path in images if path.startswith(f'{theme}/')}, theme

    log = contract['objects']['log']
    back = images[log['sprites']['back']].getchannel('A')
    front = images[log['sprites']['front']].getchannel('A')
    assert ImageChops.multiply(back, front).getbbox() is None, theme
    assert back.getpixel(tuple(log['perch'])) == 255, theme
    if 'cover' in log:
        animal = contract['objects']['woodlouse']
        hidden = images[animal['sprites']['hidden']].getchannel('A')
        x, y = log['cover']
        ax, ay = animal['anchor']
        front_at_cover = front.crop((x - ax, y - ay, x - ax + hidden.width, y - ay + hidden.height))
        opaque = hidden.histogram()[255]
        covered = ImageChops.multiply(hidden, front_at_cover).histogram()[255]
        assert 0 < covered < opaque, (theme, covered, opaque)


def proof_for_theme(theme, contract, images):
    result = images[contract['layers']['base']].copy()
    objects = contract['objects']

    def draw_object(name, sprite, position=None):
        data = objects[name]
        x, y = position or data['position']
        ax, ay = data['anchor']
        result.alpha_composite(images[data['sprites'][sprite]], (x - ax, y - ay))

    def on_log(anchor):
        log = objects['log']
        x, y = log['position']
        ax, ay = log['anchor']
        ox, oy = log[anchor]
        return x - ax + ox, y - ay + oy

    draw_object('rock', 'shadow')
    draw_object('log', 'shadow')
    draw_object('log', 'back')
    if theme == 'forest':
        draw_object('woodlouse', 'hidden', on_log('cover'))
    draw_object('log', 'front')
    draw_object('log', 'growth')
    draw_object('rock', 'default')
    if theme == 'forest':
        draw_object('moss', 'default')
        draw_object('leaf', 'default')
        draw_object('leafLarge', 'default')
        draw_object('woodlouse', 'default', on_log('perch'))
        draw_object('woodlouse', 'walk')
    else:
        draw_object('plant', 'default')
        draw_object('shrimp', 'default', on_log('perch'))
        draw_object('shrimp', 'swim')
        result.alpha_composite(images[contract['layers']['surface']])
    result.alpha_composite(images[contract['layers']['foreground']])
    result.alpha_composite(images[contract['layers']['light']])
    return result


def density_proof(scene):
    """Same screen-size strips, with new half-pixel marks only in the 960 proof."""
    crop = scene.crop((70, 110, 198, 200))
    width, height = crop.size
    lower = crop.resize((width // 2, height // 2), NEAREST).resize((width * 2, height * 2), NEAREST)
    standard = crop.resize((width * 2, height * 2), NEAREST)
    higher = standard.copy()
    pencil = ImageDraw.Draw(higher)
    # Shared framing makes these lower-contrast fissures and moss ticks comparable.
    bark = ((18, 128, 12), (49, 117, 9), (56, 119, 15), (90, 111, 18), (117, 104, 10),
            (132, 98, 12), (151, 90, 15), (178, 84, 15), (201, 64, 14), (214, 56, 14))
    for x, y, span in bark:
        pencil.line(((x, y), (x + span // 2, y - 1), (x + span, y - 4)), '#a17b54')
    for x, y in ((36, 80), (44, 74), (56, 72), (62, 79), (69, 65), (72, 67), (83, 76), (102, 78)):
        pencil.line(((x, y), (x + 2, y - 3), (x + 3, y - 4)), '#a3a453')

    gutter = 10
    panel = Image.new('RGB', (width * 6 + gutter * 4, height * 2 + 36), '#202c28')
    labels = ('240 nearest', '480 native', '960 local repaint')
    ink = ImageDraw.Draw(panel)
    for index, sample in enumerate((lower, standard, higher)):
        x = gutter + index * (width * 2 + gutter)
        panel.paste(sample, (x, 25))
        ink.text((x, 7), labels[index], fill='#e8e4cf')
    return panel


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('--proof', type=Path, help='also write comparison images to this directory')
    args = parser.parse_args()
    manifest = loads((OUT / 'manifest.json').read_text(encoding='utf-8'))
    canvas = manifest['canvas']['width'], manifest['canvas']['height']
    images = {f'{theme}/{name}.png': transparent_rgb_cleared(cel.image)
              for theme, cels in (('forest', render_forest()), ('pond', render_pond()))
              for name, cel in cels.items()}
    for theme in ('forest', 'pond'):
        verify(theme, manifest[theme], images, canvas)
    for name, image in images.items():
        out = OUT / name
        out.parent.mkdir(parents=True, exist_ok=True)
        image.save(out, optimize=True)
    if args.proof:
        args.proof.mkdir(parents=True, exist_ok=True)
        for theme in ('forest', 'pond'):
            proof = proof_for_theme(theme, manifest[theme], images)
            proof.resize((canvas[0] * 2, canvas[1] * 2), NEAREST).save(args.proof / f'{theme}.png')
            if theme == 'forest':
                density_proof(proof).save(args.proof / 'density.png')
    print(f'Verified and exported {len(images)} transparent-capable R5 PNG layers.')


if __name__ == '__main__':
    main()
