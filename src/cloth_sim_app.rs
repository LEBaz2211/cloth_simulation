use core::num;

use crate::sim_gen::{generate_cloth, generate_sphere, Vertex};

use wgpu_bootstrap::{
    cgmath,
    context::Context,
    runner::App,
    util::orbit_camera::{CameraUniform, OrbitCamera},
    wgpu::{self, util::DeviceExt, TextureView},
    winit::event::Event,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ClothRange {
    start: u32,
    end: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ClothVertexState {
    position: [f32; 3],
    velocity: [f32; 3],
}

const WORKGROUP_SIZE: u32 = 8;

const CLOTH_WIDTH: usize = 10;

const CLOTH_HEIGHT: usize = 10;

const CLOTH_SPACING: f32 = 0.1;

const CLOTH_OFFSET: f32 = 0.5;

const NUM_CLOTH_VERTICES: usize = CLOTH_WIDTH * CLOTH_HEIGHT;

const NUM_CLOTH_INDICES: usize = CLOTH_WIDTH * CLOTH_HEIGHT * 6;

const SPHERE_RADIUS: f32 = 1.0;

pub struct ClothSimApp {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    cloth_vertices: Vec<Vertex>,
    cloth_vertex_buffer: wgpu::Buffer,
    cloth_index_buffer: wgpu::Buffer,
    bind_group: [wgpu::BindGroup; 2],
    step: u32,
    camera: OrbitCamera,
    compute_pipeline: wgpu::ComputePipeline,
    simulation_time: f32,
}

impl ClothSimApp {
    pub fn new(context: &mut Context) -> Self {
        context.window().set_title("Cube App");

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        };

        let (sphere_vertices, sphere_indices) = generate_sphere(SPHERE_RADIUS, 5, 5);

        println!("sphere indices: {:?}", sphere_indices);
        // cast vec to array
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

        let vertex_buffer =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&sphere_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let max_sphere_index = sphere_indices.iter().max().unwrap_or(&0);

        let (cloth_vertices, cloth_indices) = generate_cloth(
            CLOTH_WIDTH,
            CLOTH_HEIGHT,
            CLOTH_SPACING,
            SPHERE_RADIUS,
            CLOTH_OFFSET,
            *max_sphere_index,
        );

        let cloth_num_indices = cloth_indices.len() as u32;

        //put all the indices together
        let cloth_indices: Vec<u32> = sphere_indices
            .iter()
            .map(|i| i.clone())
            .chain(cloth_indices.iter().map(|i| i.clone() + *max_sphere_index))
            .collect();

        println!("cloth indices: {:?}", cloth_indices);

        let num_indices = (sphere_indices.len() + cloth_indices.len()) as u32;

        // cast vec to array
        let cloth_vertices: &[Vertex] = &cloth_vertices
            .iter()
            .map(|v| v.clone())
            .collect::<Vec<Vertex>>();
        let cloth_indices: &[u32] = &cloth_indices;

        let cloth_vertex_buffer =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cloth Vertex Buffer"),
                    contents: bytemuck::cast_slice(&cloth_vertices),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC,
                });

        let cloth_index_buffer =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cloth Index Buffer"),
                    contents: bytemuck::cast_slice(&cloth_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        // Assuming the cloth vertices start after the sphere vertices
        let cloth_range = ClothRange {
            start: *max_sphere_index as u32, // total number of sphere vertices
            end: (max_sphere_index + cloth_num_indices) as u32, // total number of sphere and cloth vertices
        };

        let cloth_range_buffer =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cloth Range Buffer"),
                    contents: bytemuck::cast_slice(&[cloth_range]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let initial_state: Vec<ClothVertexState> = cloth_vertices
            .iter()
            .map(|v| ClothVertexState {
                position: v.position,
                velocity: v.velocity,
            })
            .collect();

        let cloth_vertex_storage_buffers: [Vec<wgpu::Buffer>; 2] = [
            (0..2)
                .map(|_| {
                    context
                        .device()
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Cloth Vertex Storage Buffer"),
                            contents: bytemuck::cast_slice(&initial_state),
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        })
                })
                .collect(),
            (0..2)
                .map(|_| {
                    context
                        .device()
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Cloth Vertex Storage Buffer"),
                            contents: bytemuck::cast_slice(&initial_state),
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        })
                })
                .collect(),
        ];

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

        let bind_group_layout =
            context
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Camera Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<
                                    CameraUniform,
                                >(
                                )
                                    as u64),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = [
            context
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Bind Group Ping"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: camera.buffer().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: cloth_range_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: cloth_vertex_storage_buffers[0][0].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: cloth_vertex_storage_buffers[1][0].as_entire_binding(),
                        },
                    ],
                }),
            context
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Bind Group Pong"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: camera.buffer().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: cloth_range_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: cloth_vertex_storage_buffers[1][0].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: cloth_vertex_storage_buffers[0][0].as_entire_binding(),
                        },
                    ],
                }),
        ];

        let shader = context
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let pipeline_layout =
            context
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
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
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("cloth_sim_compute.wgsl")
                        .replace("WORKGROUP_SIZE", &format!("{}", WORKGROUP_SIZE))
                        .into(),
                ),
            });

        let compute_pipeline =
            context
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &compute_shader,
                    entry_point: "computeMain",
                });

        Self {
            vertex_buffer,
            index_buffer,
            render_pipeline,
            num_indices,
            cloth_vertices: cloth_vertices.to_vec(),
            cloth_vertex_buffer,
            cloth_index_buffer,
            compute_pipeline,
            step: 0,
            bind_group,
            camera,
            simulation_time: 0.0,
        }
    }

    fn update(&mut self, context: &mut Context) {
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

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group[(self.step % 2) as usize], &[]);

            let workgroup_count = (NUM_CLOTH_INDICES as f32 / WORKGROUP_SIZE as f32).ceil() as u32;
            compute_pass.dispatch_workgroups(workgroup_count, workgroup_count, 1);
        }

        context.queue().submit(std::iter::once(encoder.finish()));

        self.step += 1;
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.cloth_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.cloth_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &self.bind_group[(self.step % 2) as usize], &[]);

            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // submit will accept anything that implements IntoIter
        context.queue().submit(std::iter::once(encoder.finish()));
    }

    fn update(&mut self, context: &mut Context, delta_time: f32) {
        self.update(context);
    }
}
