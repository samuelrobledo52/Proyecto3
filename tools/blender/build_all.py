# Genera todos los modelos: blender -b --python tools/blender/build_all.py
import os
import runpy

here = os.path.dirname(os.path.abspath(__file__))
for s in ("tree", "rock", "tower", "crystal"):
    runpy.run_path(os.path.join(here, s + ".py"), run_name="__main__")
