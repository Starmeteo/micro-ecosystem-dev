"""Master artwork for Under the Beech. Regenerate with Python 3 and Pillow."""

from argparse import ArgumentParser
from math import atan2, cos, pi, sin
from pathlib import Path
from random import Random

from PIL import Image, ImageDraw


WIDTH, HEIGHT = 480, 270
OUT = Path(__file__).resolve().parents[1] / "assets"


class Cel:
    def __init__(self, scale, size=(WIDTH, HEIGHT)):
        self.scale = scale
        self.image = Image.new("RGBA", tuple(round(axis * scale) for axis in size))
        self.draw = ImageDraw.Draw(self.image)

    def points(self, points):
        return [(round(x * self.scale), round(y * self.scale)) for x, y in points]

    def poly(self, color, *points):
        self.draw.polygon(self.points(points), fill=color)

    def line(self, color, width, *points):
        self.draw.line(self.points(points), fill=color, width=max(1, round(width * self.scale)))

    def box(self, color, x, y, w, h):
        a, b = self.points([(x, y), (x + w - 1, y + h - 1)])
        self.draw.rectangle((a, b), fill=color)

    def oval(self, color, x, y, w, h):
        a, b = self.points([(x, y), (x + w - 1, y + h - 1)])
        self.draw.ellipse((a, b), fill=color)


def skies(scale):
    cel = Cel(scale)
    # Broad color masses show through between trunks. The warm opening is offset
    # left; the cooler right-hand hollows set up contrast behind the fern.
    cel.box("#284337", 0, 0, WIDTH, HEIGHT)
    cel.poly("#3a5843", (112, 0), (310, 0), (271, 45), (337, 99), (312, 144),
             (205, 151), (84, 107), (98, 63))
    cel.poly("#50664a", (145, 0), (281, 0), (243, 42), (270, 76), (243, 119),
             (148, 109), (129, 71))
    cel.poly("#687750", (166, 0), (249, 0), (225, 43), (239, 73), (208, 89),
             (156, 72), (166, 46))
    cel.poly("#899056", (182, 0), (228, 0), (213, 29), (223, 48), (197, 63),
             (170, 51), (187, 29))
    cel.poly("#344d3d", (273, 103), (371, 44), (480, 30), (480, 162), (303, 164))
    cel.poly("#49664e", (0, 92), (75, 114), (139, 100), (213, 124), (258, 111),
             (366, 128), (480, 108), (480, 156), (0, 159))

    far = [(9, -8, 26, 120), (49, 12, 64, 132), (88, -6, 95, 130),
           (127, 10, 135, 138), (155, -9, 148, 140), (186, 2, 194, 137),
           (223, -11, 216, 137), (242, 6, 230, 136), (272, -10, 268, 142),
           (311, 0, 292, 135), (327, -7, 337, 139), (367, 13, 347, 143),
           (407, -10, 382, 142), (442, 6, 436, 144), (472, 12, 461, 148)]
    for index, (x, top, end, bottom) in enumerate(far):
        width = 2 + index % 4
        color = ["#3c5844", "#53694c", "#687851", "#3e5948"][index % 4]
        cel.poly(color, (x, top), (x + width, top), (end + width, bottom - 8),
                 (end + width * 3, bottom), (end - width, bottom), (end - width, bottom - 15))
        if index % 2 == 0:
            cel.line(color, width * .65, (x - 15, top + 36), (x + 2, top + 56),
                     (x + (end - x) * .6, top + 87))
    return cel


def cluster(cel, x, y, width, height, colors, seed, amount=20):
    """A group of leaf lobes clustered around a common canopy or mound."""
    shade, body, rim = colors
    cel.poly(shade, (x-width, y+height*.1), (x-width*.8, y-height*.3),
             (x-width*.46, y-height*.24), (x-width*.33, y-height*.76),
             (x+width*.08, y-height), (x+width*.29, y-height*.63),
             (x+width*.52, y-height*.73), (x+width*.72, y-height*.33),
             (x+width, y-height*.09), (x+width*.78, y+height*.34),
             (x+width*.38, y+height*.36), (x+width*.18, y+height*.64),
             (x-width*.38, y+height*.62), (x-width*.58, y+height*.35))
    rng = Random(seed)
    for i in range(amount):
        px = x + rng.uniform(-.9, .85) * width
        py = y + rng.uniform(-.75, .32) * height * (1 - abs(px-x) / (width*1.15))
        w = rng.uniform(.09, .28) * width
        h = rng.uniform(.12, .36) * height
        fill = rim if py < y-height*.12 and i % 4 == 0 else body
        cel.poly(fill, (px-w, py+h*.1), (px-w*.4, py-h*.33),
                 (px-w*.15, py-h*.8), (px+w*.46, py-h*.65),
                 (px+w*.54, py-h*.31), (px+w, py-h*.1), (px+w*.81, py+h*.28),
                 (px+w*.13, py+h*.39), (px-w*.39, py+h*.27))


def backdrop(scale):
    cel = Cel(scale)
    for x, y, w, h in ((29, 109, 35, 21), (71, 114, 37, 16), (111, 112, 43, 21),
                       (151, 119, 30, 17), (185, 122, 32, 12), (236, 118, 43, 16),
                       (277, 118, 35, 20), (313, 113, 46, 26), (360, 118, 44, 24),
                       (415, 116, 49, 28), (461, 118, 36, 28)):
        cluster(cel, x, y, w, h, ("#314d3c", "#3d5a43", "#4b6448"), x, 14)
    # Standing trunks are hand blocked, with larger dark gaps instead of equally
    # spaced bars. Small bark breaks follow the direction of each trunk.
    trunks = [
        (40, 75, 88, 135, "#24382f", "#3c5040", "#57634a"),
        (94, 112, 133, 135, "#2e4537", "#4a5641", "#5e654a"),
        (293, 301, 286, 140, "#324938", "#485d44", "#677052"),
        (347, 370, 360, 139, "#24382f", "#394c3c", "#535b45"),
        (409, 435, 412, 149, "#22362e", "#35483a", "#4c5841"),
        (456, 493, 466, 155, "#1e322c", "#344538", "#465441"),
    ]
    for left, right, base, bottom, dark, middle, rim in trunks:
        cel.poly(dark, (left, -5), (right, -5), (right-3, 54), (base+11, bottom-26),
                 (base+31, bottom-3), (base-6, bottom), (base-23, bottom-13),
                 (base-7, bottom-28), (left+4, 50))
        cel.poly(middle, (left+4, -5), (left+13, -5), (left+14, 48), (base+1, bottom-34),
                 (base-6, bottom-16), (base-17, bottom-13), (base-7, bottom-29),
                 (left+7, 49))
        cel.line(rim, 2, (left+10, 5), (left+11, 37), (left+15, 58))
        cel.line(rim, 2, (left+20, 77), (base-8, bottom-36), (base-12, bottom-25))
        branch = -1 if left > 250 else 1
        cel.poly(dark, (left+7, 15), (left+branch*17, 7), (left+branch*43, -7),
                 (left+branch*48, -7), (left+branch*34, 8),
                 (left+branch*16, 22), (left+8, 31))
    for x, y, w, h in ((1, 20, 67, 32), (66, 12, 43, 21), (112, 5, 35, 16),
                       (153, 7, 26, 13), (260, 1, 42, 24), (309, 13, 40, 24),
                       (366, 20, 31, 21), (414, 13, 46, 27), (471, 25, 43, 27)):
        cluster(cel, x, y, w, h, ("#172e28", "#243c2e", "#37513a"), x+301)
    return cel


def ground(scale):
    cel = Cel(scale)
    cel.poly("#2a392a", (0, 121), (46, 127), (92, 139), (128, 132), (169, 137),
             (198, 127), (239, 131), (280, 127), (331, 141), (360, 138), (408, 146),
             (446, 138), (480, 136), (480, 270), (0, 270))
    cel.poly("#454a33", (0, 167), (86, 159), (133, 150), (177, 154), (199, 146),
             (226, 148), (253, 138), (290, 145), (314, 151), (342, 149),
             (397, 168), (480, 169), (480, 245), (0, 235))
    cel.poly("#564f39", (480, 188), (439, 197), (390, 186), (347, 179), (308, 178),
             (294, 165), (256, 163), (235, 152), (210, 157), (218, 167), (184, 179),
             (171, 198), (120, 199), (100, 217), (20, 222), (0, 236), (0, 270), (480, 270))
    cel.poly("#685c41", (253, 166), (285, 173), (293, 181), (323, 178), (369, 194),
             (386, 213), (347, 221), (328, 241), (267, 241), (225, 251), (152, 246),
             (130, 235), (96, 234), (127, 216), (181, 203), (185, 190), (231, 183))
    cel.poly("#74674a", (264, 177), (304, 183), (320, 193), (353, 197), (359, 211),
             (327, 219), (306, 233), (246, 225), (208, 238), (175, 232), (187, 221),
             (222, 211), (221, 196), (241, 188))
    cel.poly("#60563e", (0, 250), (67, 247), (98, 254), (144, 250), (190, 260),
             (267, 254), (299, 266), (480, 240), (480, 270), (0, 270))
    rng = Random(98)
    # Narrow broken strata follow the receding ground. Keep the center quiet
    # enough for the animal silhouette to read, with denser near-frame edges.
    for i in range(210 if scale == 1 else 124):
        y = rng.randrange(140, 274)
        x = rng.randrange(0, WIDTH)
        w = rng.randrange(3, 12) * (.58 + (y-140) / 120)
        if 182 < x < 342 and 184 < y < 228 and i % 3:
            continue
        color = ["#4b4934", "#5b5239", "#625943", "#786749", "#81704e"][i % 5]
        cel.poly(color, (x, y), (x+w*.45, y-2), (x+w, y-1), (x+w*.72, y+2),
                 (x+w*.28, y+3), (x+w*.17, y+1))
    for x, y, w, h in ((17, 168, 41, 13), (66, 165, 38, 15), (115, 154, 40, 14),
                       (151, 163, 34, 15), (182, 147, 27, 9), (347, 159, 34, 13),
                       (394, 175, 48, 18), (461, 167, 38, 16), (463, 217, 43, 26),
                       (65, 221, 32, 16), (33, 239, 47, 21)):
        cluster(cel, x, y, w, h, ("#343d29", "#495234", "#5c6039"), x+987, 21)
    return cel


def frond(cel, points, width, colors, near=False):
    """Stems follow a drawn curve. Opposite leaflets taper toward its tip."""
    stem, underside, face, edge = colors
    start, control, end = points
    count = 9 if cel.scale == 1 else 7

    def at(t):
        u = 1 - t
        return tuple(u*u*a + 2*u*t*b + t*t*c for a, b, c in zip(start, control, end))

    cel.line(stem, 2 if near else 1, *(at(i / 16) for i in range(17)))
    for i in range(count, 0, -1):
        t = .16 + (i-1) * .79 / count
        x, y = at(t)
        a, b = at(min(1, t+.04))
        angle = atan2(b-y, a-x)
        length = width * (sin(pi * (t*.93+.06)) ** .8) * (1.05-t*.4)
        for sign in (-1, 1):
            ax = angle + sign*1.05
            tx, ty = (x + cos(ax)*length, y + sin(ax)*length)
            cx, cy = (cos(angle)*length*.36, sin(angle)*length*.36)
            cel.poly(underside, (x, y), (x+cx*.35, y+cy*.35),
                     ((x+tx)*.5-cx*.22, (y+ty)*.5-cy*.22), (tx, ty),
                     (tx+cx*.15, ty+cy*.15), ((x+tx)*.5+cx, (y+ty)*.5+cy))
            cel.poly(face, (x, y), ((x+tx)*.5, (y+ty)*.5), (tx, ty),
                     ((x+tx)*.5+cx*.46, (y+ty)*.5+cy*.46))
            if near and i < count - 2:
                cel.line(edge, 1, ((x+tx)*.5, (y+ty)*.5), (tx, ty))


def fern(cel, origin, tips, width, colors, near=False):
    x, y = origin
    for dx, dy, bow in tips:
        frond(cel, (origin, (x+dx*.37, y+dy*bow), (x+dx, y+dy)), width, colors, near)


def leaf(cel, x, y, length, width, angle, colors):
    shadow, lit, vein = colors

    def point(f, side):
        return (x + cos(angle)*length*f - sin(angle)*width*side,
                y + sin(angle)*length*f + cos(angle)*width*side)

    cel.poly(shadow, *(point(f,s) for f,s in ((0,0),(.18,-.57),(.35,-1),(.61,-.82),
              (.77,-.56),(1,0),(.84,.32),(.64,.76),(.29,.89),(.12,.43))))
    cel.poly(lit, *(point(f,s) for f,s in ((.04,0),(.18,-.52),(.35,-.87),(.61,-.76),
              (.77,-.45),(1,0),(.63,.04),(.41,.17),(.23,.07))))
    if length > 12 or cel.scale == 1:
        cel.line(vein, 1, point(-.08, .03), point(.22, .05), point(.52, -.06), point(.86, .02))
        if length > 17:
            for f in (.26, .41, .54):
                cel.line(vein, 1, point(f, .04), point(f+.14, -.59))


def deadwood(cel):
    # Root and log planes run along the left bank, away from the movable rock.
    cel.poly("#201f1e", (-3, 181), (59, 155), (80, 147), (122, 146), (146, 151),
             (168, 159), (148, 177), (113, 189), (51, 199), (0, 203))
    cel.poly("#5a432e", (-4, 176), (28, 165), (56, 150), (82, 143), (104, 147),
             (122, 143), (148, 149), (161, 152), (157, 165), (140, 168),
             (117, 179), (80, 181), (43, 193), (0, 196))
    cel.poly("#765737", (0, 173), (55, 152), (84, 143), (104, 147), (122, 143),
             (148, 149), (156, 153), (133, 158), (112, 163), (66, 172), (48, 180),
             (18, 184), (0, 190))
    cel.poly("#8b6a43", (3, 176), (54, 156), (79, 150), (100, 152), (124, 146),
             (148, 150), (131, 153), (113, 159), (73, 164), (52, 172), (27, 178))
    cel.poly("#3b3225", (160, 152), (168, 155), (165, 163), (156, 170),
             (139, 175), (133, 167), (137, 162), (144, 156))
    cel.line("#a3855a", 2, (159, 155), (149, 159), (142, 164), (143, 170), (153, 167), (161, 162))
    cel.line("#756043", 2, (158, 158), (152, 160), (147, 165), (155, 165))
    for colors, points in (
        ("#392c22", ((3, 186), (28, 177), (47, 176), (82, 165), (113, 163), (131, 157))),
        ("#4a3525", ((8, 192), (37, 187), (76, 175), (97, 175), (127, 168))),
        ("#ae8a59", ((24, 173), (52, 163), (71, 160), (88, 153), (106, 154))),
        ("#946744", ((11, 188), (39, 181), (56, 179), (68, 174))),
        ("#5e4630", ((69, 154), (83, 153), (94, 156), (109, 153))),
    ):
        cel.line(colors, 2, *points)
    # Hand-placed shelf fungi give scale and a muted warm accent.
    for x, y, w in ((117, 173, 14), (106, 179, 11), (114, 183, 7), (24, 189, 9)):
        cel.poly("#35271f", (x-w, y+3), (x-w*.76, y-2), (x, y-4), (x+w*.3, y-2),
                 (x+w*.58, y+1), (x+w*.39, y+4), (x-w*.58, y+5))
        cel.line("#c0a47a", 2, (x-w*.75,y), (x-w*.22,y-2), (x+w*.29,y-1), (x+w*.47,y+1))
        cel.line("#8c6143", 1, (x-w*.63,y+2), (x+w*.39,y+2))


def wood(scale):
    cel = Cel(scale)
    deadwood(cel)
    return cel


def moss(scale):
    cel = Cel(scale)
    for x, y, w, h, seed in ((20, 166, 28, 9, 4), (47, 156, 27, 10, 2),
                              (72, 151, 27, 12, 1), (96, 146, 25, 11, 3),
                              (112, 146, 20, 8, 5), (134, 151, 13, 6, 8)):
        cluster(cel, x, y, w, h, ("#333b25", "#526038", "#87904d"), seed, 23)
    for x,y,w,h in ((35, 207, 28, 11),(165, 177, 14, 7),(189, 169, 17, 7),
                    (336, 170, 19, 8),(395, 189, 29, 11),(419, 199, 25, 11),
                    (462, 188, 24, 11),(93,211,18,7),(365,213,20,9)):
        cluster(cel, x, y, w, h, ("#343e28", "#5e693a", "#899252"), x+y, 16)
    return cel


def ferns(scale):
    cel = Cel(scale)
    fern(cel, (145, 158), ((-33,-31,1.1),(-20,-46,.87),(-5,-59,.77),
                          (16,-50,.98),(31,-39,1.13),(34,-23,1.3)), 11,
         ("#88985b", "#3b5738", "#69834c", "#a5ae66"), True)
    fern(cel, (402, 186), ((-77,-27,1.68),(-71,-56,1.06),(-43,-69,.94),
                          (-9,-74,.82),(21,-69,.95),(47,-48,1.23),(56,-27,1.53)), 19,
         ("#aab16c", "#395138", "#66884d", "#9da966"), True)
    fern(cel, (373, 189), ((-37,-18,1.3),(-21,-36,1.08),(-2,-39,.66),(14,-25,1.04)),
         10, ("#8f9c56", "#4c5836", "#75864a", "#a9ae65"), True)
    return cel


def litter(scale):
    cel = Cel(scale)
    for x, y, length, width, angle in (
        (70,218,27,5,-.38),(106,214,23,6,.23),(119,231,33,8,-.35),(165,231,20,4,2.72),
        (200,204,22,4,-.49),(211,218,17,4,.29),(226,239,37,9,2.92),
        (271,223,20,5,2.36),(315,222,31,7,.45),(334,238,20,5,2.43),
        (379,218,25,6,-.19),(393,227,32,7,.46),(418,215,30,6,-.57),
        (432,233,27,6,2.68),(160,185,17,4,.56),(287,167,17,4,2.75),
        (158,246,20,4,.35),(355,250,26,6,-.53),(141,203,24,5,.42),
    ):
        colors = ("#593929", "#9b6542", "#c38e5d") if y > 200 else ("#55432c", "#7b613c", "#a08754")
        leaf(cel, x, y, length, width, angle, colors)
    return cel


def foliage(scale):
    cel = ferns(scale)
    cel.image.alpha_composite(litter(scale).image)
    return cel


def foreground(scale):
    cel = Cel(scale)
    cel.poly("#292b22", (0, 222), (24, 226), (32, 241), (86, 246), (117, 257),
             (181, 261), (214, 270), (0, 270))
    cel.poly("#283025", (480, 190), (467, 216), (441, 228), (436, 244), (397, 255),
             (357, 256), (336, 270), (480, 270))
    near_greens = ("#879754", "#253b2b", "#4d6340", "#748653")
    fern(cel, (12, 268), ((-36,-67,1.1),(-14,-101,.81),(26,-88,.87),(57,-58,1.19),
                         (82,-26,1.6)), 23, near_greens, True)
    fern(cel, (460, 269), ((-60,-23,1.47),(-44,-50,1.1),(-18,-56,.92),(14,-54,.84),
                          (36,-44,.91)), 19, near_greens, True)
    for x,y,w,h in ((8,257,25,11),(45,252,17,9),(74,263,21,11),(458,257,27,14),
                    (427,266,23,8),(384,268,18,8)):
        cluster(cel, x, y, w, h, ("#28392a", "#405738", "#63794c"), x+y, 18)
    for x, y, w, h, angle in ((57,265,41,10,-.25), (72,260,33,14,.21), (108,269,43,13,-.23),
                               (399,270,43,11,-.29), (438,267,48,15,3.85), (464,248,30,9,2.95)):
        leaf(cel, x, y, w, h, angle, ("#382b24", "#61462f", "#8c6c43"))
    return cel


def light(scale):
    cel = Cel(scale)
    # Separate soft-edged shaft masses and small pools of light placed on the earth.
    cel.poly((255,223,144,15), (177,0),(225,0),(362,206),(300,224),(238,155))
    cel.poly((255,228,145,18), (197,0),(212,0),(331,185),(292,208),(247,157))
    cel.poly((255,236,170,22), (202,0),(208,0),(307,188),(289,194),(251,156))
    cel.poly((245,225,160,12), (145,0),(154,0),(238,173),(213,181),(171,80))
    cel.poly((251,220,149,24), (238,180),(262,174),(278,179),(301,192),(287,197),
             (273,195),(260,202),(236,196),(226,189))
    cel.poly((255,233,165,26), (228,202),(248,199),(256,202),(249,208),(226,211),(211,207))
    for x,y,w,h in ((204,56,2,2),(227,77,2,2),(242,96,2,1),(267,124,3,2),(261,149,1,1),
                    (287,153,2,2),(183,43,2,1),(217,139,2,2),(306,173,2,1)):
        if cel.scale == 1 or w >= 2:
            cel.box((255,234,174,154), x, y, w, h)
    return cel


def rock(scale):
    cel = Cel(scale, (82, 56))
    cel.poly("#292e2a", (9,45),(9,37),(16,25),(26,14),(35,11),(47,10),(56,14),
             (67,17),(74,28),(76,39),(70,47),(60,50),(46,50),(33,52),(19,49))
    cel.poly("#536055", (9,40),(15,28),(27,16),(37,13),(47,13),(55,17),(65,19),
             (70,29),(67,37),(55,41),(38,45),(21,42))
    cel.poly("#788374", (14,33),(28,17),(36,14),(45,14),(51,17),(58,18),(64,23),
             (56,30),(44,31),(38,33),(28,31),(22,37))
    cel.poly("#9a9d85", (17,31),(28,19),(37,15),(48,16),(56,20),(55,23),(45,22),
             (41,25),(35,24),(30,27),(26,27),(23,31))
    cel.poly("#b1ad90", (28,20),(36,16),(42,16),(46,18),(40,20),(34,20),(30,23),(24,24))
    cel.poly("#646d61", (61,23),(67,21),(73,29),(73,36),(68,42),(59,44),(54,40),(59,36))
    cel.poly("#434f47", (10,40),(20,40),(29,36),(36,38),(40,34),(46,34),(48,38),
             (44,42),(49,45),(41,47),(25,48),(15,46))
    cel.poly("#38453d", (48,39),(55,36),(59,39),(58,43),(70,40),(72,45),(60,48),(51,48))
    cel.line("#4a5247", 2, (32,31),(41,27),(45,25),(49,24),(55,24))
    cel.line("#adad91", 1, (36,29),(41,28),(45,26),(52,25))
    cel.line("#93917b", 1, (22,38),(30,35),(34,36),(39,35))
    for x,y,w,h in ((19,39,8,3),(26,34,10,5),(61,25,10,4),(67,38,7,4),(54,19,8,3)):
        cluster(cel, x, y, w, h, ("#43533a", "#63764a", "#929757"), x+y, 8)
    return cel


def contact(scale):
    cel = Cel(scale, (82, 56))
    cel.oval((23,27,24,50), 5, 38, 76, 17)
    cel.oval((22,25,23,86), 8, 42, 67, 12)
    cel.oval((22,23,21,111), 14, 45, 56, 8)
    return cel


def isopod(scale, frame):
    cel = Cel(scale, (34, 26))
    if frame == "shelter":
        cel.oval((22,22,22,85), 7, 17, 21, 6)
        cel.poly("#232827", (7,15),(8,10),(12,8),(16,7),(22,9),(26,13),(26,17),
                 (22,21),(16,22),(10,20))
        cel.poly("#7b7c73", (8,15),(9,12),(13,9),(17,9),(20,10),(22,14),(18,17),(11,17))
        cel.line("#a2a094", 1, (12,10),(13,12),(11,15))
        cel.line("#555852", 2, (16,10),(17,12),(15,15),(14,18))
        cel.line("#424743", 2, (21,12),(21,15),(18,17),(18,20))
        return cel

    cel.oval((25,23,20,92), 4, 16, 26, 7)
    for index, x in enumerate((10, 13, 16, 19, 22)):
        step = (index + frame) % 3 - 1
        cel.line("#3c3b35", 2, (x, 17), (x-2+step, 20), (x-2+step, 22))
        cel.line("#958d7d", 1, (x, 18), (x-2+step, 20))
    cel.poly("#242928", (3,15),(6,12),(9,10),(13,9),(18,9),(22,10),(27,12),
             (29,14),(28,17),(25,19),(19,20),(12,20),(6,18))
    cel.poly("#777d77", (4,15),(8,12),(13,10),(19,10),(24,11),(27,13),(26,15),
             (21,17),(15,18),(8,17))
    cel.poly("#999c8e", (7,13),(12,11),(17,10),(22,11),(25,12),(24,13),(17,13),(12,14))
    cel.poly("#535951", (6,16),(12,17),(18,17),(24,15),(24,17),(19,19),(10,19))
    for x,y in ((8, 12),(12,11),(16,10),(20,10),(24,11)):
        cel.line("#3c433e", 1, (x,y+1),(x+1,y+4),(x,y+6))
        if scale == 1:
            cel.box("#c3beaa", x+1,y,2,1)
    cel.box("#1c211f", 26, 13, 2, 2)
    cel.box("#d3ba92", 27, 13, 1, 1)
    cel.line("#9d917c", 1, (26,12),(29,9+frame%2),(32,7+frame%2))
    cel.line("#918675", 1, (28,15),(32,14),(33,12+frame%2))
    return cel


def write_layer(cel, name, directory):
    cel.image.save(directory / f"{name}.png", optimize=True)


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--comparison", action="store_true", help="also render the density comparison sheet")
    arguments = parser.parse_args()
    for density in (1, 2):
        scale = density / 2
        directory = OUT / f"d{density}"
        directory.mkdir(parents=True, exist_ok=True)
        layers = (("sky", skies), ("trees", backdrop), ("soil", ground), ("wood", wood),
                  ("moss", moss), ("foliage", foliage), ("ferns", ferns), ("litter", litter),
                  ("front", foreground), ("light", light),
                  ("rock", rock), ("contact", contact))
        for name, draw in layers:
            write_layer(draw(scale), name, directory)
        for frame in ("shelter", 0, 1, 2):
            write_layer(isopod(scale, frame), f"woodlouse-{frame}", directory)
    if arguments.comparison:
        compare_densities()


def compare_densities():
    sheet = Image.new("RGB", (1016, 477), "#14221b")
    draw = ImageDraw.Draw(sheet)
    for density, x in ((1, 18), (2, 518)):
        scale = density / 2
        directory = OUT / f"d{density}"

        def load(name):
            with Image.open(directory / f"{name}.png") as image:
                return image.convert("RGBA")

        frame = load("sky")
        for name in ("trees", "soil", "wood", "moss", "foliage"):
            frame.alpha_composite(load(name))
        for name, px, py, ax, ay in (
            ("contact", 264, 194, 41, 46), ("rock", 264, 194, 41, 46),
            ("woodlouse-0", 209, 212, 17, 17), ("woodlouse-1", 284, 222, 17, 17),
            ("woodlouse-2", 315, 226, 17, 17),
        ):
            frame.alpha_composite(load(name), (round((px-ax)*scale), round((py-ay)*scale)))
        frame.alpha_composite(load("front"))
        frame.alpha_composite(load("light"))
        draw.text((x, 12), f"Density source {240*density} x {135*density}   /   display 480 x 270", fill="#e4d49e")
        sheet.paste(frame.resize((480,270), Image.Resampling.NEAREST), (x, 31))
        draw.text((x, 311), "Same source at 240 x 135 display", fill="#e4d49e")
        sheet.paste(frame.resize((240,135), Image.Resampling.NEAREST), (x, 329))
    sheet.save(Path(__file__).with_name("density-comparison.png"), optimize=True)


if __name__ == "__main__":
    main()
