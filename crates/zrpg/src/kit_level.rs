//! Hand-built levels from asset kits, in the `level.bin` format written by
//! `tools/make_ruins.py` and `tools/make_dreamscape.py`: each module's
//! primitives (material slot, positions, normals, UVs, 16-bit indices),
//! then every placed instance (module, 3x3 linear part by columns,
//! translation), all in the kit's metres. A trailing section is the
//! caller's: wall faces for the ruins' foliage, lights for a dreamscape.
//!
//! Every instance is baked into the merged mesh its slot joins, scaled by
//! [`SCALE`] once, so a whole level draws in a handful of calls.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Kit metres to game units: one 2 m kit cell is one game cell.
pub const SCALE: f32 = 0.5;

/// Little-endian reader for `level.bin`.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> &[u8] {
        let s = &self.bytes[self.at..self.at + n];
        self.at += n;
        s
    }

    pub fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }

    pub fn f32s<const N: usize>(&mut self, count: usize) -> Vec<[f32; N]> {
        (0..count)
            .map(|_| core::array::from_fn(|_| f32::from_le_bytes(self.take(4).try_into().unwrap())))
            .collect()
    }

    fn u16s(&mut self, count: usize) -> Vec<u16> {
        let v = (0..count)
            .map(|_| u16::from_le_bytes(self.take(2).try_into().unwrap()))
            .collect();
        // Index lists are padded to four bytes.
        if count % 2 == 1 {
            self.take(2);
        }
        v
    }

    /// Whether everything has been read.
    pub fn is_done(&self) -> bool {
        self.at == self.bytes.len()
    }
}

struct Primitive {
    slot: usize,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u16>,
}

#[derive(Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// A module's primitives, ready to place.
pub struct Module(Vec<Primitive>);

fn read_modules(r: &mut Reader) -> Vec<Module> {
    (0..r.u32())
        .map(|_| {
            Module(
                (0..r.u32())
                    .map(|_| {
                        let slot = r.u32() as usize;
                        let n = r.u32() as usize;
                        let positions = r.f32s::<3>(n);
                        let normals = r.f32s::<3>(n);
                        let uvs = r.f32s::<2>(n);
                        let count = r.u32() as usize;
                        Primitive {
                            slot,
                            positions,
                            normals,
                            uvs,
                            indices: r.u16s(count),
                        }
                    })
                    .collect(),
            )
        })
        .collect()
}

/// Read one placement: the module, then its 3x3 linear part by columns
/// and its translation, in kit metres.
fn read_placement(r: &mut Reader) -> (Mat3, Vec3) {
    let [m] = r.f32s::<12>(1).try_into().unwrap();
    (Mat3::from_cols_slice(&m[..9]), Vec3::from_slice(&m[9..]))
}

impl Module {
    /// Bake this module, placed by `linear` and `translation`, into the
    /// meshes `pick(slot)` chooses.
    fn bake(
        &self,
        linear: Mat3,
        translation: Vec3,
        out: &mut [MeshData],
        pick: impl Fn(usize) -> usize,
    ) {
        let normal_matrix = linear.inverse().transpose();
        // A mirroring transform would turn faces inside out.
        let flip = linear.determinant() < 0.0;
        for prim in &self.0 {
            let mesh = &mut out[pick(prim.slot)];
            let base = mesh.positions.len() as u32;
            for ((p, n), uv) in prim.positions.iter().zip(&prim.normals).zip(&prim.uvs) {
                let world = (linear * Vec3::from_array(*p) + translation) * SCALE;
                mesh.positions.push(world.to_array());
                mesh.normals.push(
                    (normal_matrix * Vec3::from_array(*n))
                        .normalize()
                        .to_array(),
                );
                mesh.uvs.push(*uv);
            }
            for tri in prim.indices.chunks_exact(3) {
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| base + i as u32);
                mesh.indices
                    .extend_from_slice(&if flip { [a, c, b] } else { [a, b, c] });
            }
        }
    }
}

/// Read the modules and place every instance, merging into `meshes` meshes:
/// `pick(slot, instance)` says which one each primitive joins. Returns them
/// and the reader, at the caller's trailing section.
pub fn parse(
    bytes: &[u8],
    meshes: usize,
    pick: impl Fn(usize, u32) -> usize,
) -> (Vec<MeshData>, Reader<'_>) {
    let mut r = Reader { bytes, at: 0 };
    assert_eq!(r.take(4), b"ZRL1", "level.bin header");
    let modules = read_modules(&mut r);
    let mut out: Vec<MeshData> = (0..meshes).map(|_| MeshData::default()).collect();
    for i in 0..r.u32() {
        let module = r.u32() as usize;
        let (linear, translation) = read_placement(&mut r);
        modules[module].bake(linear, translation, &mut out, |slot| pick(slot, i));
    }
    (out, r)
}

/// A library of modules shared by several floors (`modules.bin`, written by
/// `tools/make_patchwork.py`): each module once per material remap.
pub struct Library(Vec<Module>);

impl Library {
    pub fn parse(bytes: &[u8]) -> Library {
        let mut r = Reader { bytes, at: 0 };
        assert_eq!(r.take(4), b"ZRM1", "modules.bin header");
        let modules = read_modules(&mut r);
        assert!(r.is_done(), "modules.bin has trailing data");
        Library(modules)
    }

    /// Place a floor's instances (a `floor_NN.bin`: each a library module,
    /// its theme, and its placement), merging into `themes` x 4 meshes, one
    /// per theme and role. Returns them and the reader, at the floor's
    /// lights.
    pub fn place<'a>(&self, floor: &'a [u8], themes: usize) -> (Vec<MeshData>, Reader<'a>) {
        let mut r = Reader {
            bytes: floor,
            at: 0,
        };
        assert_eq!(r.take(4), b"ZRV1", "dream floor header");
        let mut out: Vec<MeshData> = (0..themes * 4).map(|_| MeshData::default()).collect();
        for _ in 0..r.u32() {
            let module = r.u32() as usize;
            let theme = r.u32() as usize;
            let (linear, translation) = read_placement(&mut r);
            self.0[module].bake(linear, translation, &mut out, |slot| theme * 4 + slot);
        }
        (out, r)
    }
}

/// Read bytes as they come: for the caller's trailing sections.
impl Reader<'_> {
    pub fn bytes(&mut self, n: usize) -> &[u8] {
        self.take(n)
    }
}
