# Utilidades compartidas: limpiar escena, materiales, armar mallas con bmesh y exportar OBJ.
import os
import bpy
import bmesh
from mathutils import Matrix

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models")


def clear_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def material(name, rgb):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.diffuse_color = (*rgb, 1.0)
    try:
        mat.use_nodes = True
    except Exception:
        pass
    if mat.node_tree:
        for n in mat.node_tree.nodes:
            if n.type == "BSDF_PRINCIPLED":
                n.inputs["Base Color"].default_value = (*rgb, 1.0)
                n.inputs["Roughness"].default_value = 1.0
    return mat


def tag(bm, before, idx):
    # asigna material a las caras nuevas
    for f in bm.faces:
        if f not in before:
            f.material_index = idx


def cone(bm, idx, segs, r1, r2, h, mat=Matrix()):
    # cono/cilindro con base en z=0 (bmesh lo crea centrado)
    before = set(bm.faces)
    m = mat @ Matrix.Translation((0, 0, h / 2))
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=True, segments=segs,
                          radius1=r1, radius2=r2, depth=h, matrix=m)
    tag(bm, before, idx)


def ico(bm, idx, subdiv, r, mat=Matrix()):
    before = set(bm.faces)
    res = bmesh.ops.create_icosphere(bm, subdivisions=subdiv, radius=r, matrix=mat)
    tag(bm, before, idx)
    return res["verts"]


def box(bm, idx, size, mat=Matrix()):
    before = set(bm.faces)
    m = mat @ Matrix.Diagonal((*size, 1.0))
    bmesh.ops.create_cube(bm, size=1.0, matrix=m)
    tag(bm, before, idx)


def ground(bm):
    # desplaza la malla para que su minimo en z sea 0
    zmin = min(v.co.z for v in bm.verts)
    for v in bm.verts:
        v.co.z -= zmin


def finish(name, bm, mats):
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for m in mats:
        me.materials.append(m)
    for p in me.polygons:
        p.use_smooth = False
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def export(name):
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name + ".obj")
    bpy.ops.wm.obj_export(
        filepath=path,
        export_selected_objects=False,
        export_triangulated_mesh=True,
        export_normals=True,
        export_materials=True,
        export_uv=False,
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )
    print("exportado", path)


def build(name, fn):
    # escena limpia -> fn() crea el objeto -> exporta
    clear_scene()
    fn(name)
    export(name)
