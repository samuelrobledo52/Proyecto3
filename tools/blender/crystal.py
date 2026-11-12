# Cumulo de cristales: prismas puntiagudos inclinados sobre una base de roca.
import os
import sys
import math
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bmesh
from mathutils import Matrix
import common as c

CYAN = (0.45, 0.90, 1.00)
BASE = (0.38, 0.38, 0.46)


def prism(bm, segs, r, h, tip, m):
    c.cone(bm, 0, segs, r, r, h, m)
    c.cone(bm, 0, segs, r, 0.0, tip, m @ Matrix.Translation((0, 0, h)))


def tilt(az, ang):
    return Matrix.Rotation(math.radians(az), 4, "Z") @ Matrix.Rotation(math.radians(ang), 4, "X")


def hi(name):
    bm = bmesh.new()
    prism(bm, 6, 0.13, 0.85, 0.30, Matrix())
    for az, ang, r, h in ((0, 30, 0.09, 0.55), (80, 35, 0.08, 0.45),
                          (160, 28, 0.10, 0.60), (250, 38, 0.07, 0.38)):
        prism(bm, 6, r, h, r * 2.2, tilt(az, ang))
    c.ico(bm, 1, 1, 0.26, Matrix.Diagonal((1.2, 1.2, 0.45, 1)))
    c.ground(bm)
    c.finish(name, bm, [c.material("crystal", CYAN), c.material("base", BASE)])


def lo(name):
    bm = bmesh.new()
    prism(bm, 4, 0.13, 0.85, 0.30, Matrix())
    prism(bm, 4, 0.10, 0.55, 0.22, tilt(160, 30))
    c.ground(bm)
    c.finish(name, bm, [c.material("crystal", CYAN)])


c.build("crystal", hi)
c.build("crystal_lo", lo)
