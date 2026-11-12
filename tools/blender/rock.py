# Roca: icosfera deformada con semilla fija y aplanada.
import os
import sys
import random
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bmesh
import common as c

GRAY = (0.50, 0.45, 0.40)


def make(name, subdiv):
    rng = random.Random(7)
    bm = bmesh.new()
    verts = c.ico(bm, 0, subdiv, 0.5)
    key = lambda v: (round(v.co.x, 4), round(v.co.y, 4), round(v.co.z, 4))
    for v in sorted(verts, key=key):
        v.co *= 1.0 + rng.uniform(-0.12, 0.12)
    for v in bm.verts:
        v.co.y *= 0.85
        v.co.z *= 0.85
        v.co.z = max(v.co.z, -0.16)  # base plana para apoyarse
    c.ground(bm)
    c.finish(name, bm, [c.material("stone", GRAY)])


c.build("rock", lambda n: make(n, 2))
c.build("rock_lo", lambda n: make(n, 1))
