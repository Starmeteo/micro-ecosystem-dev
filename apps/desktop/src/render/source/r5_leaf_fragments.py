"""Assemble separate, authored F06 torn-leaf sprites from the approved R5 leaf art."""

from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageEnhance


COMPOSED = Path(__file__).resolve().parents[1] / 'assets' / 'r5' / 'forest-composed'
SCALE = 6
SIZE = (36 * SCALE, 24 * SCALE)

# The source boxes sample different parts of the approved leaves. The masks
# select torn margins and midrib sections with distinct silhouettes.
PARTS = (
    ('leaf.png', (.25, .30, .49, .61), (
        (.03, .30), (.21, .21), (.38, .02), (.48, .19), (.73, .11), (.82, .32), (.98, .38),
        (.83, .58), (.87, .81), (.62, .70), (.48, .96), (.30, .67), (.09, .66),
    ), 25, 23, .79),
    ('gold-leaf.png', (.49, .22, .70, .48), (
        (.05, .36), (.29, .08), (.48, .24), (.72, .05), (.73, .21), (.96, .27), (.86, .45),
        (.92, .65), (.65, .82), (.45, .70), (.23, .92), (.20, .58), (.04, .60),
    ), 15, -39, .72),
    ('leaf.png', (.33, .49, .51, .73), (
        (.12, .14), (.46, .10), (.56, .27), (.81, .15), (.97, .48), (.79, .60), (.71, .88),
        (.41, .74), (.21, .90), (.04, .58),
    ), 8, 52, .71),
    ('gold-leaf.png', (.35, .46, .53, .67), (
        (.04, .48), (.18, .16), (.47, .08), (.56, .28), (.81, .18), (.96, .54), (.71, .69),
        (.68, .91), (.43, .82), (.25, .95), (.19, .75),
    ), 8, -17, .64),
)


def cut_part(spec):
    file, bounds, outline, length_mm, angle, shade = spec
    source = Image.open(COMPOSED / file).convert('RGBA')
    left, top, right, bottom = bounds
    width, height = source.size
    cut = source.crop((round(left * width), round(top * height), round(right * width), round(bottom * height)))
    mask = Image.new('L', cut.size)
    ImageDraw.Draw(mask).polygon([(round(x * cut.width), round(y * cut.height)) for x, y in outline], fill=255)
    cut.putalpha(ImageChops.multiply(cut.getchannel('A'), mask))
    cut = cut.crop(cut.getbbox())
    cut = ImageEnhance.Color(cut).enhance(.72)
    cut = ImageEnhance.Brightness(cut).enhance(shade)
    width = length_mm * SCALE
    cut = cut.resize((width, round(width * cut.height / cut.width * .71)), Image.Resampling.LANCZOS)
    return cut.rotate(angle, resample=Image.Resampling.BICUBIC, expand=True)


def cluster(parts, locations):
    result = Image.new('RGBA', SIZE)
    for index, x, y in locations:
        part = parts[index]
        result.alpha_composite(part, (round(x * SCALE - part.width / 2), round(y * SCALE - part.height / 2)))
    return result


def main():
    parts = [cut_part(spec) for spec in PARTS]
    frames = {
        'default': cluster(parts, [(1, 25, 10), (0, 14, 14), (2, 31, 17), (3, 8, 19)]),
        'worn': cluster(parts, [(1, 19, 14), (2, 28, 17), (3, 11, 18)]),
        'trace': cluster(parts, [(2, 16, 15), (3, 24, 17)]),
    }
    for frame, image in frames.items():
        suffix = '' if frame == 'default' else '-' + frame
        image.save(COMPOSED / f'litter-fragments{suffix}.png', optimize=True)


if __name__ == '__main__':
    main()
