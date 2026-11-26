"""Build the F02 flattish stone sprites and their contact shadows on the R5 grid."""

from json import loads
from pathlib import Path

from PIL import Image, ImageDraw


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5'

PALETTES = (
    ('#242924', '#414641', '#626863', '#7c8177', '#858980', '#565c55', '#979b8d'),
    ('#252823', '#41483f', '#616a61', '#717d73', '#858d80', '#515a50', '#909d88'),
    ('#272925', '#454b45', '#646a64', '#818677', '#8c8f82', '#555b56', '#9c9c8f'),
    ('#2a2b25', '#494b42', '#6b6d62', '#74796b', '#8d8d7b', '#5b5f54', '#9c9b85'),
)


def rock(draw, center_x, ground_y, width, height, variant):
    edge, foot, top, face, shoulder, fissure, glint = PALETTES[variant % len(PALETTES)]
    slope = (-.11, .08, -.18, .20)[variant % 4]

    def at(x, y):
        return (round(center_x + x * width), round(ground_y + (y + slope * x) * height))

    def face_at(color, points):
        draw.polygon([at(x, y) for x, y in points], fill=color)

    face_at(edge, [(-.50, -.34), (-.37, -.69), (-.14, -.88), (.19, -.92),
                   (.34, -.73), (.50, -.30), (.44, -.07), (.24, .02), (-.26, .03), (-.44, -.09)])
    face_at(foot, [(-.46, -.32), (-.24, -.45), (.14, -.49), (.47, -.31),
                   (.38, -.15), (.21, -.05), (-.23, -.04), (-.40, -.14)])
    face_at(top, [(-.46, -.33), (-.35, -.65), (-.13, -.82), (.17, -.85),
                  (.32, -.67), (.46, -.32), (.18, -.46), (-.18, -.40)])
    face_at(face, [(-.39, -.47), (-.29, -.70), (-.12, -.79), (.15, -.80),
                   (.17, -.66), (.02, -.54), (-.19, -.55)])
    face_at(shoulder, [(-.35, -.62), (-.23, -.74), (-.04, -.79), (.09, -.73), (-.19, -.63)])
    face_at(fissure, [(.15, -.64), (.25, -.67), (.35, -.52), (.31, -.44), (.20, -.51), (.00, -.51)])
    if width >= 14:
        draw.line([at(-.28, -.66), at(-.12, -.70), at(-.02, -.65)], fill=glint)
    if width > 32:
        draw.line([at(-.36, -.21), at(-.17, -.26), at(-.02, -.24), at(.20, -.30)], fill=top)
        draw.line([at(-.06, -.56), at(.02, -.52), at(.17, -.53)], fill=fissure)


def footprint_shadow(draw, center_x, ground_y, width):
    for shift, inset, opacity in ((2, 0, 32), (1, 2, 70), (0, 3, 122)):
        left = round(center_x - width * .47 + inset)
        right = round(center_x + width * .54 - inset)
        draw.ellipse((left, ground_y - 2, right, ground_y + 2 + shift), fill=(19, 22, 18, opacity))


def alpha_runs(sprite):
    alpha = sprite.getchannel('A')
    columns = [x for x in range(sprite.width) if alpha.crop((x, 0, x + 1, sprite.height)).getbbox()]
    return 1 + sum(right > left + 1 for left, right in zip(columns, columns[1:]))


def main():
    manifest = loads((OUT / 'manifest.json').read_text(encoding='utf-8'))
    forest = manifest['forest']
    pixels_per_mm = manifest['canvas']['width'] / forest['referenceWidthMm']
    for index, (name, spec) in enumerate(forest['stonework'].items()):
        size = spec['width'], spec['height']
        anchor_x, anchor_y = spec['anchor']
        sprite, shadow = Image.new('RGBA', size), Image.new('RGBA', size)
        sprite_pen, shadow_pen = ImageDraw.Draw(sprite), ImageDraw.Draw(shadow)
        for member, part in enumerate(spec['parts']):
            x = anchor_x + part['offsetMm'] * pixels_per_mm
            width = part['lengthMm'] * pixels_per_mm
            footprint_shadow(shadow_pen, x, anchor_y, width)
            rock(sprite_pen, x, anchor_y, width, part['heightPx'], index + member)
        alpha_box = sprite.getbbox(alpha_only=True)
        if alpha_box is None:
            raise ValueError(f'{name} is empty')
        span = alpha_box[2] - alpha_box[0]
        assert abs(span - spec['lengthMm'] * pixels_per_mm) <= 2, (name, span)
        assert alpha_runs(sprite) == alpha_runs(shadow) == len(spec['parts']), name
        sprite.save(OUT / spec['sprites']['default'], optimize=True)
        shadow.save(OUT / spec['sprites']['shadow'], optimize=True)
        print(f'{name}: {span} logical px, {alpha_runs(sprite)} disconnected silhouettes, {size[0]}×{size[1]} cel')


if __name__ == '__main__':
    main()
