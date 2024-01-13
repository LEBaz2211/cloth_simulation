use std::time::{Duration, Instant};

use crate::sim_gen::{generate_cloth, generate_sphere, Vertex};

use wgpu_bootstrap::{
    cgmath,
    context::Context,
    runner::App,
    util::orbit_camera::{CameraUniform, OrbitCamera},
    wgpu::{self, util::DeviceExt, TextureView},
    winit::event::Event,
};

const WORKGROUP_SIZE: u32 = 128;

const SPHERE_RADIUS: f32 = 1.0;
const CLOTH_OFFSET: f32 = 0.5;

const CLOTH_WIDTH: usize = 30;
const CLOTH_HEIGHT: usize = 30;
const CLOTH_SPACING: f32 = 0.1;

pub struct ClothSimApp {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    cloth_index_buffer: wgpu::Buffer,
    cloth_vertex_position_buffer: wgpu::Buffer,
    cloth_num_indices: u32,
    cloth_num_vertices: u32,
    compute_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,

    step: u32,
    camera: OrbitCamera,
    generation_duration: Duration,
    last_generation: Instant,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformsFloats {
    damping: f32,
    timeStep: f32,
    sphereRadius: f32,
    gridWidth: u32,
    gridHeight: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformsArrays {
    gravity: [f32; 4],
    sphereCenter: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformsSpring {
    structuralStiffness: f32,
    shearStiffness: f32,
    bendStiffness: f32,
    restLengthStructural: f32,
    restLengthShear: f32,
    restLengthBend: f32,
    // Add more parameters as needed
}

impl ClothSimApp {
    pub fn new(context: &mut Context) -> Self {
        context.window().set_title("Cloth Simulation App");

        // Generate the sphere

        let (sphere_vertices, sphere_indices) = generate_sphere(SPHERE_RADIUS, 16, 16);

        let sphere_vertices: &[Vertex] = &sphere_vertices
            .iter()
            .map(|v| v.clone())
            .collect::<Vec<Vertex>>();

        let sphere_indices: &[u32] = &sphere_indices;

        let index_buffer = context
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&sphere_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let num_indices = sphere_indices.len() as u32;

        let vertex_buffer =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&sphere_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        // Generate the cloth

        let (cloth_vertices, cloth_indices) = generate_cloth(
            CLOTH_WIDTH,
            CLOTH_HEIGHT,
            CLOTH_SPACING,
            SPHERE_RADIUS,
            CLOTH_OFFSET,
        );

        let cloth_vertices: &[Vertex] = &cloth_vertices
            .iter()
            .map(|v| v.clone())
            .collect::<Vec<Vertex>>();
        let cloth_indices: &[u32] = &cloth_indices;

        let cloth_index_buffer =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cloth Index Buffer"),
                    contents: bytemuck::cast_slice(&cloth_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let cloth_num_indices = cloth_indices.len() as u32;

        let cloth_num_vertices = cloth_vertices.len() as u32;

        let cloth_vertex_position_buffer =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cloth Vertex Position Buffer"),
                    contents: bytemuck::cast_slice(&cloth_vertices),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST,
                });

        // set up the uniforms

        let uniformsFloats = UniformsFloats {
            damping: 0.99,
            timeStep: 0.01,
            sphereRadius: SPHERE_RADIUS,
            gridWidth: CLOTH_WIDTH as u32,
            gridHeight: CLOTH_HEIGHT as u32,
        };

        let uniformsArrays = UniformsArrays {
            gravity: [0.0, -9.8, 0.0, 0.0],
            sphereCenter: [0.0, 0.0, 0.0, 0.0],
        };

        let uniformsSpring = UniformsSpring {
            structuralStiffness: 1000.0,
            shearStiffness: 1000.0,
            bendStiffness: 1000.0,
            restLengthStructural: CLOTH_SPACING,
            restLengthShear: CLOTH_SPACING * 2.0f32.sqrt(),
            restLengthBend: CLOTH_SPACING * 2.0,
        };

        let uniform_buffer_floats =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniformsFloats]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let uniform_buffer_arrays =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniformsArrays]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let uniform_buffer_spring =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniformsSpring]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // SETTING UP BIND GROUPS

        let compute_bind_group_layout =
            context
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("compute_bind_group_layout"),
                });

        let compute_bind_group = context
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: cloth_vertex_position_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform_buffer_floats.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buffer_arrays.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: uniform_buffer_spring.as_entire_binding(),
                    },
                ],
                label: Some("compute_bind_group"),
            });

        let mut camera = OrbitCamera::new(
            context,
            45.0,
            (context.config().width as f32) / (context.config().height as f32),
            0.1,
            100.0,
        );
        camera
            .set_target(cgmath::point3(0.0, 0.0, 0.0))
            .set_polar(cgmath::point3(2.0, 0.0, 0.0))
            .update(context);

        let shader = context
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let camera_bind_group_layout = context
            .device()
            .create_bind_group_layout(&CameraUniform::desc());

        let compute_pipeline_layout =
            context
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Compute Pipeline Layout"),
                    bind_group_layouts: &[&compute_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline_layout =
            context
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            context
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: context.config().format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Line, // Requires Features::NON_FILL_POLYGON_MODE
                        // Requires Features::DEPTH_CLIP_CONTROL
                        unclipped_depth: false,
                        // Requires Features::CONSERVATIVE_RASTERIZATION
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: *context.depth_format(),
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });

        let compute_shader = context
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("cloth_sim_compute.wgsl").into()),
            });

        let compute_pipeline =
            context
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    layout: Some(&compute_pipeline_layout),
                    module: &compute_shader,
                    entry_point: "main",
                });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices,

            cloth_index_buffer,
            cloth_num_indices,
            cloth_num_vertices,
            cloth_vertex_position_buffer,
            compute_bind_group,

            render_pipeline,
            compute_pipeline,

            step: 0,
            camera,
            generation_duration: Duration::new(0, 100_000_00),
            last_generation: Instant::now(),
        }
    }

    fn update(&mut self, context: &mut Context) {
        if self.last_generation + self.generation_duration < Instant::now() {
            let mut encoder =
                context
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Compute Encoder"),
                    });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                });
                compute_pass.set_pipeline(&self.compute_pipeline); // Compute pipeline you've created
                compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

                let workgroup_count =
                    (self.cloth_num_vertices as f32 / WORKGROUP_SIZE as f32).ceil() as u32;
                compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
            }

            context.queue().submit(std::iter::once(encoder.finish()));

            self.step += 1;
            self.last_generation = Instant::now();
        }
    }
}

impl App for ClothSimApp {
    fn input(&mut self, context: &mut Context, event: &Event<()>) {
        self.camera.process_events(context, event)
    }

    fn render(&mut self, context: &mut Context, view: &TextureView) {
        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: context.depth_texture_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            // Draw the sphere
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &self.camera.bind_group(), &[]);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

            // Draw the cloth
            render_pass.set_vertex_buffer(0, self.cloth_vertex_position_buffer.slice(..));
            render_pass
                .set_index_buffer(self.cloth_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.cloth_num_indices, 0, 0..1);
        }

        // submit will accept anything that implements IntoIter
        context.queue().submit(std::iter::once(encoder.finish()));
    }

    fn update(&mut self, context: &mut Context, delta_time: f32) {
        self.update(context);
    }
}
