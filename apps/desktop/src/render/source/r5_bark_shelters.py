"""Rebuild the F04 ground shadows, bark profiles, and toolbar previews."""

from json import loads
from pathlib import Path

from PIL import Image, ImageDraw


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5'


def build_shelter(spec, pixels_per_mm, split):
    size = spec['width'], spec['height']
    ax, ay = spec['anchor']
    outer = spec['lengthMm'] * pixels_per_mm / 2
    mouth = spec['openingWidthMm'] * pixels_per_mm / 2
    room = spec['openingHeightMm'] * pixels_per_mm
    rise = room + (15 if split else 12)
    layers = {name: Image.new('RGBA', size) for name in ('shadow', 'interior', 'back', 'front', 'preview')}

    def polygon(image, color, points):
        ImageDraw.Draw(image).polygon([(round(ax + x), round(ay + y)) for x, y in points], fill=color)

    def streak(image, color, points, width=1):
        ImageDraw.Draw(image).line([(round(ax + x), round(ay + y)) for x, y in points], fill=color, width=width)

    polygon(layers['shadow'], (19, 21, 16, 52), [(-outer+2, -10), (-outer*.64, -26), (outer*.65, -27),
                                               (outer-2, -9), (outer+3, 4), (outer*.73, 10),
                                               (-outer*.44, 12), (-outer-3, 6)])
    for sign in (-1, 1):
        foot = sign * spec['footOffsetMm'] * pixels_per_mm
        polygon(layers['shadow'], (16, 18, 13, 148), [(foot-8, -3), (foot+9, -4), (foot+11, 2),
                                                    (foot+5, 7), (foot-9, 6), (foot-13, 2)])

    polygon(layers['interior'], (22, 20, 15, 130), [(-mouth, 9), (-mouth-2, -8), (-mouth*.69, -room*.93),
                                                   (-mouth*.47, -rise-8), (mouth*.58, -rise-9),
                                                   (mouth*.86, -room*.62), (mouth+1, -7), (mouth, 9)])
    polygon(layers['interior'], (26, 21, 15, 195), [(-mouth+4, -1), (-mouth*.63, -room*.94),
                                                   (-mouth*.33, -rise+2), (mouth*.51, -rise+3),
                                                   (mouth*.84, -room*.75), (mouth-2, -4),
                                                   (mouth*.67, 2), (-mouth*.61, 3)])

    back = layers['back']
    polygon(back, '#282119', [(-outer+6, -12), (-outer*.78, -rise*.88), (-outer*.45, -rise-13),
                              (outer*.06, -rise-17), (outer*.47, -rise-15), (outer*.73, -rise-7),
                              (outer-4, -rise*.59), (outer-3, -13), (outer*.76, -8),
                              (outer*.52, -rise*.96), (-outer*.61, -rise*.96), (-outer*.77, -9)])
    polygon(back, '#54402c', [(-outer*.87, -rise*.71), (-outer*.5, -rise-11), (-outer*.08, -rise-15),
                              (outer*.47, -rise-13), (outer*.7, -rise-5), (outer*.82, -rise*.61),
                              (outer*.51, -rise*.94), (-outer*.57, -rise*.88)])
    streak(back, '#806044', [(-outer*.49, -rise-10), (-outer*.05, -rise-13),
                             (outer*.25, -rise-12), (outer*.45, -rise-11)])
    streak(back, '#3e2e20', [(-outer*.69, -rise*.68), (-outer*.46, -rise-5),
                             (-outer*.16, -rise-8), (outer*.23, -rise-8), (outer*.58, -rise-5)], 2)

    front = layers['front']
    contour = [(-outer, 4), (-outer+1, -5), (-outer*.85, -rise*.53), (-outer*.70, -rise*.79),
               (-outer*.38, -rise+3), (-outer*.03, -rise), (outer*.32, -rise+1),
               (outer*.62, -rise*.89), (outer*.83, -rise*.58), (outer, -rise*.22),
               (outer, 3), (outer*.81, 6), (outer*.52, 4), (-outer*.59, 5), (-outer*.83, 6)]
    polygon(front, '#30241b', contour)
    polygon(front, '#745137', [(-outer+2, 1), (-outer*.8, -rise*.56), (-outer*.68, -rise*.76),
                               (-outer*.32, -rise+5), (-outer*.06, -rise+2), (outer*.31, -rise+3),
                               (outer*.65, -rise*.81), (outer*.85, -rise*.52), (outer-2, -rise*.2),
                               (outer-3, 1), (outer*.72, -1), (outer*.60, -rise*.58),
                               (outer*.23, -rise*.71), (-outer*.27, -rise*.73),
                               (-outer*.67, -rise*.43), (-outer*.75, 2)])
    polygon(front, '#523926', [(-outer*.86, 0), (-outer*.71, -rise*.48), (-outer*.35, -rise*.67),
                               (-outer*.08, -rise*.72), (outer*.26, -rise*.68), (outer*.57, -rise*.6),
                               (outer*.82, -rise*.28), (outer*.84, 3), (outer*.52, 3),
                               (-outer*.54, 3), (-outer*.73, 4)])
    streak(front, '#ac8053', [(-outer*.8, -rise*.48), (-outer*.65, -rise*.72),
                              (-outer*.36, -rise*.82), (-outer*.14, -rise*.9), (outer*.04, -rise*.91)])
    streak(front, '#916945', [(outer*.12, -rise*.88), (outer*.32, -rise*.83),
                              (outer*.51, -rise*.78), (outer*.68, -rise*.62)])
    streak(front, '#bd8d5f', [(-outer*.82, -rise*.34), (-outer*.67, -rise*.52),
                              (-outer*.42, -rise*.61), (-outer*.21, -rise*.72)])
    streak(front, '#4a3526', [(outer*.50, -rise*.70), (outer*.68, -rise*.51),
                              (outer*.81, -rise*.36), (outer*.89, -rise*.13)], 2)
    hole = [(-mouth, 8), (-mouth, -3), (-mouth*.82, -room*.58), (-mouth*.52, -room*.83),
            (-mouth*.18, -room), (mouth*.24, -room+.5), (mouth*.60, -room*.75),
            (mouth*.85, -room*.46), (mouth, -2), (mouth, 8)]
    streak(front, '#916745', hole[1:-1], 3)
    polygon(front, (0, 0, 0, 0), hole)
    if split:
        crack = [(outer*.22, -rise-2), (outer*.42, -rise-2), (outer*.33, -rise+12),
                 (outer*.16, -rise+16), (outer*.22, -rise+8), (outer*.12, -rise+5)]
        polygon(front, (0, 0, 0, 0), crack)
        streak(front, '#ac8053', [(outer*.36, -rise+3), (outer*.32, -rise+12), (outer*.19, -rise+16)])

    for role in ('shadow', 'interior', 'back', 'front'):
        layers['preview'].alpha_composite(layers[role])
    return layers


def main():
    manifest = loads((OUT / 'manifest.json').read_text(encoding='utf-8'))
    forest = manifest['forest']
    pixels_per_mm = manifest['canvas']['width'] / forest['referenceWidthMm']
    for name, spec in forest['barkShelters'].items():
        layers = build_shelter(spec, pixels_per_mm, name == 'split')
        ax, ay = spec['anchor']
        front_alpha = layers['front'].getchannel('A')
        back_alpha = layers['back'].getchannel('A')
        assert front_alpha.getpixel((ax, ay-3)) == back_alpha.getpixel((ax, ay-3)) == 0
        for x, _ in spec['contact']:
            assert front_alpha.getpixel((round(x), ay-3)) == 255, (name, x)
        assert abs(front_alpha.getbbox()[2] - front_alpha.getbbox()[0] - spec['lengthMm'] * pixels_per_mm) <= 2
        for role, image in layers.items():
            image.save(OUT / spec['sprites'][role], optimize=True)
        print(f'{name}: front hole and both feet verified, {len(layers)} layers')


if __name__ == '__main__':
    main()
