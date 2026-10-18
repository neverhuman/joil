"""Minimal Blender side of the adapter. Run only through plate.py after semantic checks."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import bpy

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--glb', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--manifest', required=True)
    parser.add_argument('--frames', type=int, required=True)
    args,_=parser.parse_known_args(sys.argv[sys.argv.index('--')+1:])
    out=Path(args.output); out.mkdir(parents=True,exist_ok=True)
    manifest_path=Path(args.manifest); manifest=json.loads(manifest_path.read_text())
    completed=set()
    frame_hashes=manifest.get('frame_sha256',{})
    for frame in manifest.get('completed_frames',[]):
        path=out/f'frame-{frame:06d}.png'
        if path.is_file() and frame_hashes.get(str(frame)) == hashlib.sha256(path.read_bytes()).hexdigest():
            completed.add(frame)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=args.glb)
    # Keep source scientific meshes as imported: no subdivision, smoothing, or denoising.
    bpy.ops.object.camera_add(location=(0.22,-0.26,0.12)); cam=bpy.context.object
    cam.data.lens=50; bpy.context.scene.camera=cam
    from mathutils import Vector
    cam.rotation_euler=(Vector((0.0,0.0,0.08))-cam.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.light_add(type='AREA', location=(0.25,-0.2,0.3)); bpy.context.object.data.energy=800
    scene=bpy.context.scene; scene.render.resolution_x=1200; scene.render.resolution_y=900
    scene.render.resolution_percentage=100; scene.render.image_settings.file_format='PNG'
    scene.render.engine='CYCLES'; scene.cycles.device='CPU'
    scene.render.threads_mode='FIXED'; scene.render.threads=1
    scene.render.use_file_extension=True
    scene.cycles.use_denoising=False
    # Keep text in camera space so labels and legend never change the scientific mesh.
    def overlay(label, x, y):
        curve=bpy.data.curves.new('plate_overlay','FONT'); curve.body=label; curve.size=0.025
        obj=bpy.data.objects.new('plate_overlay',curve); scene.collection.objects.link(obj)
        obj.parent=cam; obj.location=(x,y,-0.5)
    overlay('Evidence: '+str(manifest.get('evidence_class','unknown')), -0.30, 0.17)
    overlay('Observable: '+str(manifest.get('observable','unspecified')), -0.30, 0.13)
    overlay('Unit: '+str(manifest.get('unit','unspecified')), -0.30, 0.09)
    overlay('Limitations: '+str(manifest.get('limitations',['unknown']))[:100], -0.30, 0.05)
    for frame in range(args.frames):
        if frame in completed: continue
        scene.frame_set(frame+1); target=out/f'frame-{frame:06d}.png'; scene.render.filepath=str(target)
        bpy.ops.render.render(write_still=True)
        manifest.setdefault('completed_frames',[]).append(frame)
        manifest.setdefault('frame_sha256',{})[str(frame)]=hashlib.sha256(target.read_bytes()).hexdigest()
        manifest['status']='rendering'; atomic_json(manifest_path,manifest)
    manifest['status']='rendered'; atomic_json(manifest_path,manifest)

def atomic_json(path,value):
    tmp=path.with_suffix(path.suffix+'.tmp')
    with tmp.open('w',encoding='utf-8') as stream:
        json.dump(value,stream,indent=2); stream.write('\n'); stream.flush(); os.fsync(stream.fileno())
    os.replace(tmp,path)
main()
