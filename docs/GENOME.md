# Design language and geometry contract

The delivered `ct35-parametric-v1` seed represents the latest supplied baseline. It derives every station/area from named physical ratios. `crates/joil-core/src/geometry.rs` and `cad/ct35/build.py` independently derive and check its dimensions. The exact CAD driver uses this JSON directly, avoiding the supplied STEP script's second hard-coded parameter list.

Future representation, owned through P01/P02:

```rust
struct DesignProgram {
    schema: u32,
    interface_sha256: Digest,
    fluid_recipe: FluidRecipe,
    solid_recipe: SolidRecipe,
    manufacturing: ManufacturingVariant,
}
enum FluidRecipe {
    Ct35Reference(Ct35Ratios),
    ChannelGraph { nodes: Vec<Junction>, edges: Vec<Sweep>, chamber: WallField },
    Implicit { domain: Box3, basis: FieldBasis, coefficients: Vec<f64> },
}
struct Sweep {
    stable_id: FeatureId,
    endpoints: [JunctionId; 2],
    centerline: BSpline3,
    sections: Vec<Section>, // area, shape, orientation along arc length
}
enum Section { RoundedRectangle { width: f64, height: f64, radius: f64 },
               Superellipse { a: f64, b: f64, exponent: f64 }, Circle { radius: f64 } }
```

This is proposed API guidance, not an already implemented graph CAD kernel. Graph branch count is variable. Bound complexity by measured manufacturability, numerical resolution and budget, rather than a fixed 128-vector ceiling. Raise those limits through a versioned campaign. B-spline/Fourier/RBF wall fields use low-frequency coefficients first and refinement operators later. The search domain must have a minimum resolved feature scale to prevent exploitable aliasing.

Fluid and solid domains are complementary only within the actual insert envelope. CFD extensions are tagged synthetic hydraulic boundary devices, excluded from mass/fit/structural hardware. Printed head and CFD bore lengths in supplied files differ slightly; distinguish them explicitly rather than asserting identical wet geometry. Mapped loads require the shared hardware wetted faces and the exact external pressure environment.

Fixed contract for first research lane:

- z=0 is inner end-wall face; +z downstream. CAD uses mm; solver geometry uses metres with recorded transform.
- D2=35 mm, D1=70 mm, L1=70 mm, reducing-cone half angle=10°, fine taper length=770 mm, D3=17.5 mm, tail length=700 mm.
- Baseline overflow bore 1.4 mm, CFD bore length 2.8 mm and plenum R6×20 mm. Plenum is not printed hardware.
- Twin rectangular inlets, each 120.26409377023427 mm², width=7.754485597711633 mm and height=15.508971195423266 mm. Baseline inlet roof z=0 is a reconstruction assumption.
- Inlet1 CFD face x=-60, outward -x, y in [R1-w,R1], inward velocity +x. Inlet2 is a 180° copy with outward +x and inward -x. Both generate negative angular momentum about +z in this latest file. Older ZIP has the opposite convention. Do not infer sign from the older drawing.
- Hardware inlet faces x=±47 mm. CFD feed extensions and printed port faces are distinct surfaces.
- Reducing cone ends at z=169.24743184330993 mm. Head ends z=186.74743184330993 mm. Freeze the final 17.5 mm collar and downstream taper/tail; evolve upstream interior and compatible solid only.
- Full fluid inventory, including CFD extensions/plenum, is approximately 1,105,802.5227442954 mm³. V/Q at 4 m³/h is about 0.9952 s; it is not an RTD or the earlier HTML's 0.258 s value.

The initial kernel has no unrestricted topology mutation. P02 must first reproduce baseline, then conventional controls, then branch/rejoin/freeform cases with the same fixed interface. A deterministic seed plus canonical serialization/parameter hash binds every candidate. Hash the resulting B-rep/surface and record tolerances too; genotype identity alone cannot bind a changed CAD kernel.

Required F0 checks: valid single solids; positive volume; closure/orientation and no self-intersections; exact terminal area/normal/position; same fixed collar/tail; section Jacobians; minimum wall/gap/hydraulic diameter; overhang/span/removal/accessibility at declared build orientation; conservative approximations/refinement. Gate errors report geometry invalid, manufacturing infeasible, kernel failure or unknown separately. Never repair a changed interface without declaring a new subject.

Representation challenge corpus: baseline zero-mutation round trip; translated/rotated SI-equivalent subject; asymmetric dual feed; smooth helical entry; branch/rejoin manifold; obstructed dead pocket; trapped powder cavity; near-touching passages; nonsewable B-rep; spline fold; tiny aperture; unknown schema/operator; inactive-gene duplicates. Use independent B-rep and voxel/SDF measurements, with convergence where discretized. Novel geometry must earn an applicability check before search trusts its CFD prediction.
