# Torre de ruinas: cilindro octogonal con almenas, ventanas y tejado conico roto.
import os
import sys
import math
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bmesh
from mathutils import Matrix
import common as c

STONE = (0.86, 0.76, 0.58)
ROOF = (0.78, 0.20, 0.14)
DARK = (0.12, 0.09, 0.08)


def mats():
    return [c.material("stone", STONE), c.material("roof", ROOF), c.material("window", DARK)]


def roof():
    return Matrix.Translation((0.03, 0, 1.18)) @ Matrix.Rotation(math.radians(12), 4, "Y")


def hi(name):
    bm = bmesh.new()
    c.cone(bm, 0, 8, 0.36, 0.30, 0.12)
    c.cone(bm, 0, 8, 0.32, 0.28, 1.10, Matrix.Translation((0, 0, 0.10)))
    for i in range(8):
        if i == 5:
            continue  # almena caida
        a = (i + 0.5) * math.tau / 8
        m = Matrix.Translation((0.26 * math.cos(a), 0.26 * math.sin(a), 1.27)) @ Matrix.Rotation(a, 4, "Z")
        c.box(bm, 0, (0.08, 0.13, 0.14), m)
    for a, z in ((0.0, 0.55), (math.pi, 0.85)):
        m = Matrix.Rotation(a + math.tau / 16, 4, "Z") @ Matrix.Translation((0.285, 0, z))
        c.box(bm, 2, (0.05, 0.10, 0.18), m)
    c.cone(bm, 1, 8, 0.24, 0.0, 0.42, roof())
    c.ground(bm)
    c.finish(name, bm, mats())


def lo(name):
    bm = bmesh.new()
    c.cone(bm, 0, 5, 0.34, 0.28, 1.20)
    c.cone(bm, 1, 5, 0.24, 0.0, 0.42, roof())
    c.ground(bm)
    c.finish(name, bm, mats()[:2])


c.build("tower", hi)
c.build("tower_lo", lo)
