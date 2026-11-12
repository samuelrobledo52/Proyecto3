# Arbol estilizado: tronco conico + copa redondeada facetada.
import os
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bmesh
from mathutils import Matrix
import common as c

BARK = (0.45, 0.27, 0.13)
LEAF = (0.28, 0.72, 0.24)


def mats():
    return [c.material("trunk", BARK), c.material("leaves", LEAF)]


def hi(name):
    bm = bmesh.new()
    c.cone(bm, 0, 6, 0.10, 0.06, 0.40)
    for z, r, sq in ((0.52, 0.36, 0.80), (0.82, 0.26, 0.85)):
        m = Matrix.Translation((0, 0, z)) @ Matrix.Diagonal((1, 1, sq, 1))
        c.ico(bm, 1, 2, r, m)
    c.cone(bm, 1, 6, 0.12, 0.0, 0.16, Matrix.Translation((0, 0, 0.98)))
    c.ground(bm)
    c.finish(name, bm, mats())


def lo(name):
    bm = bmesh.new()
    c.cone(bm, 0, 4, 0.10, 0.06, 0.40)
    c.cone(bm, 1, 5, 0.40, 0.0, 0.80, Matrix.Translation((0, 0, 0.30)))
    c.ground(bm)
    c.finish(name, bm, mats())


c.build("tree", hi)
c.build("tree_lo", lo)
