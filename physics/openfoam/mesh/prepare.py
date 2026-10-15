#!/usr/bin/env python3
"""Make a labelled closed SI surface and a bounded full-liner snappy pilot case."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'cad/ct35'))
from geometry import load


def dictionary(name, text, cls='dictionary'):
    return f'FoamFile {{ format ascii; class {cls}; object {name}; }}\n' + text + '\n'


def write(case, name, text, cls='dictionary'):
    p = case / name
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(dictionary(p.name, text, cls))


def prepare(case, surface, genome=None, level=0):
    import numpy as np
    import trimesh
    case = Path(case).resolve()
    if not case.is_relative_to(ROOT / 'runs') or case.exists():
        raise ValueError('create a fresh case within runs; never overwrite evidence')
    obj, d = load(genome)
    p = obj['parameters']
    mesh = trimesh.load_mesh(surface, process=True)
    if not (mesh.is_watertight and mesh.is_winding_consistent and mesh.volume > 0):
        raise ValueError('surface must be closed and outward')
    if not np.allclose(mesh.bounds[1, 2], d['ze'] * .001, atol=1e-8):
        raise ValueError('surface scale or full-liner length differs from genome')
    triangles = mesh.triangles
    labels = np.full(len(triangles), 'wall', dtype=object)
    planes = {'inlet1': (0, -p['inlet_length_cfd_mm']*.001),
              'inlet2': (0, p['inlet_length_cfd_mm']*.001),
              'overflow': (2, -(2*d['ro']*p['orifice_length_over_do']+p['plenum_length_mm'])*.001),
              'underflow': (2, d['ze']*.001)}
    expected = {'inlet1': d['area']*1e-6, 'inlet2': d['area']*1e-6,
                'overflow': np.pi*p['plenum_radius_mm']**2*1e-6,
                'underflow': np.pi*d['r3']**2*1e-6}
    normal = {'inlet1': [-1,0,0], 'inlet2': [1,0,0], 'overflow': [0,0,-1], 'underflow': [0,0,1]}
    patches = {}
    for name, (axis, plane) in planes.items():
        # STL coordinates are float32. At the 1.64 m outlet one ULP exceeds 1e-8 m.
        plane_tolerance = max(1e-9, 2*float(np.spacing(np.float32(abs(plane)))))
        select = np.all(np.abs(triangles[:, :, axis] - plane) < plane_tolerance, axis=1)
        labels[select] = name
        area = mesh.area_faces[select].sum()
        orient = np.einsum('ij,j->i', mesh.face_normals[select], normal[name])
        if not select.any() or abs(area/expected[name]-1) > .002 or np.min(orient) < .999:
            raise ValueError(f'terminal {name} has wrong area or orientation')
        patches[name] = {'area_m2': float(area), 'relative_area_error': float(area/expected[name]-1),
                         'triangles': int(select.sum()), 'normal': normal[name],
                         'plane_axis':axis,'plane_position_m':plane,
                         'plane_tolerance_m': plane_tolerance}
    dest = case / 'constant/triSurface/ct35.stl'
    dest.parent.mkdir(parents=True)
    with dest.open('w') as f:
        for name in ['wall', *planes]:
            f.write(f'solid {name}\n')
            for i in np.flatnonzero(labels == name):
                f.write(' facet normal ' + ' '.join(f'{x:.12g}' for x in mesh.face_normals[i]) + '\n  outer loop\n')
                for v in triangles[i]:
                    f.write('   vertex ' + ' '.join(f'{x:.12g}' for x in v) + '\n')
                f.write('  endloop\n endfacet\n')
            f.write(f'endsolid {name}\n')
    write(case, 'system/controlDict', '''application snappyHexMesh;
startFrom startTime; startTime 0; stopAt endTime; endTime 1000; deltaT 1;
writeControl timeStep; writeInterval 1000; purgeWrite 2;
writeFormat ascii; writePrecision 12; writeCompression off; runTimeModifiable false;''')
    # Background cells approximately 6 mm. All physical walls and both complete outlets retained.
    write(case, 'system/blockMeshDict', '''convertToMeters 1;
vertices ((-.072 -.048 -.036) (.072 -.048 -.036) (.072 .048 -.036) (-.072 .048 -.036)
          (-.072 -.048 1.656) (.072 -.048 1.656) (.072 .048 1.656) (-.072 .048 1.656));
blocks (hex (0 1 2 3 4 5 6 7) (24 16 282) simpleGrading (1 1 1));
edges (); boundary (background {type patch; faces ((0 3 2 1)(4 5 6 7)(0 1 5 4)(1 2 6 5)(2 3 7 6)(3 0 4 7));});
mergePatchPairs ();''')
    region_names = '\n'.join(f'{n} {{ name {n}; }}' for n in ['wall', *planes])
    face_levels = '\n'.join(f'{n} {{ level ({(2 if n=="wall" else 3)+level} {(2 if n=="wall" else 3)+level}); patchInfo {{ type {"wall" if n=="wall" else "patch"}; }} }}' for n in ['wall', *planes])
    write(case, 'system/snappyHexMeshDict', f'''castellatedMesh true; snap true; addLayers false;
geometry {{
 ct35 {{ type triSurfaceMesh; file "ct35.stl"; name ct35; regions {{ {region_names} }} }}
 head {{type searchableBox; min (-.064 -.039 -.026); max (.064 .039 .19);}}
 bore {{type searchableBox; min (-.002 -.002 -.004); max (.002 .002 .003);}}
}}
castellatedMeshControls {{
 maxLocalCells 800000; maxGlobalCells 1200000; minRefinementCells 0; nCellsBetweenLevels 3;
 features ();
 refinementSurfaces {{ct35 {{level ({1+level} {1+level}); regions {{ {face_levels} }} }} }}
 resolveFeatureAngle 30;
 refinementRegions {{head {{mode inside; level {2+level};}} bore {{mode inside; level {5+level};}}}}
 insidePoint (0.00013 0.00017 .03513); allowFreeStandingZoneFaces true;
}}
snapControls {{nSmoothPatch 3; tolerance 1.5; nSolveIter 40; nRelaxIter 5;
 nFeatureSnapIter 10; implicitFeatureSnap true; explicitFeatureSnap false; multiRegionFeatureSnap true;}}
addLayersControls {{relativeSizes true; layers {{}} expansionRatio 1.2; finalLayerThickness .3;
 minThickness .1; nGrow 0; featureAngle 60; nRelaxIter 5; nSmoothSurfaceNormals 1;
 nSmoothNormals 3; nSmoothThickness 10; maxFaceThicknessRatio .5; maxThicknessToMedialRatio .3;
 minMedianAxisAngle 90; nBufferCellsNoExtrude 0; nLayerIter 50;}}
meshQualityControls {{maxNonOrtho 65; maxBoundarySkewness 20; maxInternalSkewness 4;
 maxConcave 80; minFlatness .5; minVol 1e-18; minTetQuality 1e-15; minArea -1; minTwist .02;
 minDeterminant .001; minFaceWeight .02; minVolRatio .01; minTriangleTwist -1;
 nSmoothScale 4; errorReduction .75; relaxed {{maxNonOrtho 75;}}}}
writeFlags (scalarLevels layerSets layerFields); mergeTolerance 1e-7;''')
    write(case, 'system/fvSchemes', 'ddtSchemes {default steadyState;} gradSchemes {default Gauss linear;} divSchemes {default none;} laplacianSchemes {default Gauss linear corrected;} interpolationSchemes {default linear;} snGradSchemes {default corrected;}')
    write(case, 'system/fvSolution', 'solvers {}')
    manifest = {'schema_version': 1, 'kind': 'full_liner_exploratory_mesh_input', 'units': 'm',
                'genome_file_sha256': hashlib.sha256(Path(genome or ROOT/'baselines/ct35/genome.json').read_bytes()).hexdigest(),
                'surface_sha256': hashlib.sha256(Path(surface).read_bytes()).hexdigest(),
                'labelled_surface_sha256': hashlib.sha256(dest.read_bytes()).hexdigest(),
                'surface_volume_m3': float(mesh.volume), 'analytic_volume_m3': d['analytic_fluid_volume_mm3']*1e-9,
                'patches': patches, 'refinement_increment': level,
                'limitations': ['mesh geometry and numerical convergence must be measured after meshing',
                                'no boundary layers in first exploratory mesh', 'not a qualified CFD evaluator']}
    (case/'mesh-input.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--case', type=Path, required=True)
    ap.add_argument('--surface', type=Path, required=True)
    ap.add_argument('--genome', type=Path)
    ap.add_argument('--level', type=int, choices=[0,1], default=0)
    a = ap.parse_args()
    prepare(a.case, a.surface, a.genome, a.level)
