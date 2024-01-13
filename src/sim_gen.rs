use std::{f32::NAN, vec};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    // Position of the vertex in 3D space
    pub position: [f32; 4],

    // Color of the vertex, useful for debugging or visual effects
    pub color: [f32; 4],

    // Velocity of the vertex in 3D space
    pub velocity: [f32; 4],
    // // Texture coordinates, if you plan to apply a texture to the cloth
    // pub tex_coords: [f32; 2],

    // Mass of the vertex
    pub mass: [f32; 4],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub fn generate_cloth(
    width: usize,
    height: usize,
    spacing: f32,
    sphere_radius: f32,
    offset: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Calculate the height above the sphere
    let cloth_height = sphere_radius + offset;

    // Generate vertices
    for i in 0..height {
        for j in 0..width {
            vertices.push(Vertex {
                position: [
                    j as f32 * spacing - width as f32 * spacing / 2.0, // Centering the cloth on X-axis
                    cloth_height, // Positioning above the sphere
                    i as f32 * spacing - height as f32 * spacing / 2.0, // Centering the cloth on Z-axis
                    0.0,
                ],
                color: [NAN, NAN, 1.0, 0.0],
                // normal: [0.0, 1.0, 0.0], // pointing up
                // tex_coords: [j as f32 / width as f32, i as f32 / height as f32],
                velocity: [0.0, 0.0, 0.0, 0.0],
                mass: [1.0, 0.0, 0.0, 0.0],
            });
        }
    }

    // Generate indices for triangles
    for i in 0..(height - 1) {
        for j in 0..(width - 1) {
            let top_left = i * width + j;
            let top_right = top_left + 1;
            let bottom_left = (i + 1) * width + j;
            let bottom_right = bottom_left + 1;

            indices.extend(&[
                top_left as u32,
                bottom_left as u32,
                top_right as u32,
                bottom_left as u32,
                bottom_right as u32,
                top_right as u32,
            ]);
        }
    }

    (vertices, indices)
}

pub fn generate_sphere(radius: f32, sectors: usize, stacks: usize) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let sector_step = 2.0 * std::f32::consts::PI / sectors as f32;
    let stack_step = std::f32::consts::PI / stacks as f32;

    for i in 0..=stacks {
        let stack_angle = std::f32::consts::PI / 2.0 - i as f32 * stack_step;
        let xy = radius * stack_angle.cos();
        let z = radius * stack_angle.sin();

        for j in 0..=sectors {
            let sector_angle = j as f32 * sector_step;

            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();

            vertices.push(Vertex {
                position: [x, y, z, 0.0],
                color: [1.0, NAN, NAN, 0.0], // red color
                velocity: [0.0, 0.0, 0.0, 0.0],
                // normal: [x, y, z],      // normals are the same as positions for a sphere
                // tex_coords: [j as f32 / sectors as f32, i as f32 / stacks as f32],
                mass: [1.0, 0.0, 0.0, 0.0],
            });
        }
    }

    // Generate indices for triangles
    for i in 0..stacks {
        let k1 = i * (sectors + 1);
        let k2 = k1 + sectors + 1;

        for j in 0..sectors {
            if i != 0 {
                indices.extend(&[(k1 + j) as u32, (k2 + j) as u32, (k1 + j + 1) as u32]);
            }
            if i != (stacks - 1) {
                indices.extend(&[(k1 + j + 1) as u32, (k2 + j) as u32, (k2 + j + 1) as u32]);
            }
        }
    }

    (vertices, indices)
}
