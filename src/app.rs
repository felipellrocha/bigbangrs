use crate::texture::Texture;
use cgmath::prelude::*;
//use flume::bounded;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

const NUM_INSTANCES: u32 = 5_000_000;

pub struct App {
    pipeline: Option<Pipeline>,
}

impl App {
    pub fn new() -> Self {
        Self { pipeline: None }
    }

    pub fn run() -> anyhow::Result<()> {
        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;
        let mut app = Self::new();
        event_loop.run_app(&mut app)?;

        Ok(())
    }
}

impl ApplicationHandler<Pipeline> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let mut pipeline = pollster::block_on(Pipeline::new(window)).unwrap();

        let size = pipeline.renderer.window.inner_size();
        pipeline
            .renderer
            .resize(size.width, size.height, &pipeline.shared.device);
        pipeline.renderer.window.request_redraw();

        self.pipeline = Some(pipeline);
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: Pipeline) {
        // This is where proxy.send_event() ends up
        self.pipeline = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let pipeline = match &mut self.pipeline {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::Resized(size) => {
                pipeline
                    .renderer
                    .resize(size.width, size.height, &pipeline.shared.device);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => pipeline.renderer.handle_key(
                event_loop,
                code,
                key_state.is_pressed(),
                &mut pipeline.camera_controller,
            ),
            WindowEvent::RedrawRequested => {
                pipeline
                    .camera_controller
                    .update_camera(&mut pipeline.camera);

                //pipeline.movement.update();
                let _ = pipeline
                    .movement
                    .update(&pipeline.shared.device, &pipeline.shared.queue);
                pipeline.renderer.update(
                    &pipeline.shared.queue,
                    &mut pipeline.camera,
                    &mut pipeline.camera_uniform,
                    &pipeline.camera_buffer,
                    &pipeline.camera_controller,
                );
                match pipeline.renderer.render(
                    &pipeline.shared.device,
                    &pipeline.shared.queue,
                    &pipeline.camera_bind_group,
                ) {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = pipeline.renderer.window.inner_size();
                        pipeline
                            .renderer
                            .resize(size.width, size.height, &pipeline.shared.device);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct Pipeline {
    shared: Shared,

    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    renderer: Renderer,
    movement: Movement,
}

impl Pipeline {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let (shared, surface) = Shared::new(window.clone()).await.unwrap();

        let instances = {
            let instances = (0..NUM_INSTANCES)
                .map(|_| {
                    use rand::RngExt;
                    let mut random_generator = rand::rng();

                    let x: f32 = random_generator.random_range(-100.0..=100.0);
                    let y: f32 = random_generator.random_range(-100.0..=100.0);
                    let z: f32 = random_generator.random_range(-100.0..=100.0);
                    let w: f32 = 1.0;
                    let translation = cgmath::Vector4 { x, y, z, w };

                    let rotation = if translation.is_zero() {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can affect scale if they're not created correctly
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(
                            translation.truncate().normalize(),
                            cgmath::Deg(45.0),
                        )
                    };
                    let r: f32 = random_generator.random();
                    let g: f32 = random_generator.random();
                    let b: f32 = random_generator.random();

                    let color = [r, g, b, 1.0];

                    Instance {
                        translation,
                        rotation,
                        color,
                    }
                })
                .collect::<Vec<_>>();
            let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
            shared
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: bytemuck::cast_slice(&instance_data),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                })
        };

        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let camera_controller = CameraController::new(0.2);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = shared
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let camera_bind_group_layout =
            shared
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("camera_bind_group_layout"),
                });

        let camera_bind_group = shared.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let movement = Movement::new(&shared.device, &shared.queue, &instances, &camera_buffer)
            .await
            .unwrap();
        let renderer = Renderer::new(
            window,
            surface,
            //
            &shared.instance,
            &shared.adapter,
            &shared.device,
            &shared.queue,
            //
            &camera_bind_group_layout,
            &movement.visible_instances,
            &movement.indirect_buffer,
        )
        .await
        .unwrap();

        Ok(Self {
            shared,
            renderer,
            movement,

            camera,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
        })
    }
}

pub struct Shared {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::Adapter,
}

impl Shared {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<(Self, wgpu::Surface<'static>)> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let adapter_limits = adapter.limits();
        println!("limits: {adapter_limits:#?}\n");
        //let mut required_limits = wgpu::Limits::default();
        //required_limits.max_buffer_size = adapter_limits.max_buffer_size;
        //println!("limits: {required_limits:#?}");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: adapter_limits,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        Ok((
            Self {
                instance,
                device,
                queue,
                adapter,
            },
            surface,
        ))
    }
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: wgpu::RenderPipeline,
    window: Arc<Window>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    visible_instances: wgpu::Buffer,
    indirect_buffer: wgpu::Buffer,
    depth: Texture,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        _instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        visible_instances: &wgpu::Buffer,
        indirect_buffer: &wgpu::Buffer,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("clip_space"),
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("paint"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview_mask: None, // 5.
            cache: None,          // 6.
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let depth = Texture::create_depth_texture(&device, &config, "depth_texture");

        Ok(Self {
            surface,
            config,
            is_surface_configured: false,
            render_pipeline,
            window,
            vertex_buffer,
            index_buffer,
            visible_instances: visible_instances.clone(),
            indirect_buffer: indirect_buffer.clone(),
            depth,
        })
    }

    fn resize(&mut self, width: u32, height: u32, device: &wgpu::Device) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(device, &self.config);
            self.is_surface_configured = true;
            self.depth = Texture::create_depth_texture(device, &self.config, "depth_texture");
        }
    }

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group: &wgpu::BindGroup,
    ) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            {
                // compute
            }
            {
                // draw
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                /*
                render_pass.set_vertex_buffer(1, self.instances.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..NUM_INSTANCES as _);
                */
                render_pass.set_vertex_buffer(1, self.visible_instances.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed_indirect(&self.indirect_buffer, 0);
            }
        }

        // submit will accept anything that implements IntoIter
        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn update(
        &mut self,
        queue: &wgpu::Queue,
        camera: &mut Camera,
        camera_uniform: &mut CameraUniform,
        camera_buffer: &wgpu::Buffer,
        camera_controller: &CameraController,
    ) {
        camera_controller.update_camera(camera);
        camera_uniform.update_view_proj(camera);
        //queue.write_buffer(camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
        queue.write_buffer(camera_buffer, 0, bytemuck::bytes_of(camera_uniform));
    }

    fn handle_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        code: KeyCode,
        is_pressed: bool,
        camera_controller: &mut CameraController,
    ) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {
                camera_controller.handle_key(code, is_pressed);
            }
        }
    }
}

pub struct Movement {
    pipeline: wgpu::ComputePipeline,
    simulation_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    #[allow(unused)]
    instances: wgpu::Buffer,
    visible_instances: wgpu::Buffer,
    indirect_buffer: wgpu::Buffer,
    start_time: Instant,
}

impl Movement {
    pub async fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        instances: &wgpu::Buffer,
        camera_buffer: &wgpu::Buffer,
    ) -> anyhow::Result<Self> {
        let shader = device.create_shader_module(wgpu::include_wgsl!("movement.wgsl"));

        let simulation_uniform = SimulationUniform {
            time: 0.0,
            amplitude: 1.0,
            frequency: 0.75,
            speed: 1.0,
            particle_count: NUM_INSTANCES,
            workgroups_per_row: 1,
            padding: Default::default(),
        };
        let simulation_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation Buffer"),
            contents: bytemuck::bytes_of(&simulation_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let visible_instances_size = (NUM_INSTANCES as wgpu::BufferAddress)
            * std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress;

        let visible_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visible Instances Buffer"),
            size: visible_instances_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indirect_args = DrawIndexedIndirectArgsStorage {
            index_count: INDICES.len() as u32,
            instance_count: 0,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        };

        let indirect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indirect Buffer"),
            contents: bytemuck::bytes_of(&indirect_args),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Movement Bind Group Layout"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Movement Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instances.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: simulation_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: visible_instances.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: indirect_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Movement Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Introduction Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Ok(Self {
            pipeline,
            bind_group,
            simulation_buffer,
            instances: instances.clone(),
            visible_instances,
            indirect_buffer,
            start_time: Instant::now(),
        })
    }

    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> anyhow::Result<()> {
        let mut encoder = device.create_command_encoder(&Default::default());

        let workgroup_size: u32 = 64;
        let total_workgroups = NUM_INSTANCES.div_ceil(workgroup_size);

        let workgroups_per_row: u32 = 1024;
        let workgroup_rows = total_workgroups.div_ceil(workgroups_per_row);

        {
            let elapsed_seconds = self.start_time.elapsed().as_secs_f32();
            let uniform = SimulationUniform {
                time: elapsed_seconds,
                amplitude: 1.0,
                frequency: 0.75,
                speed: 1.5,
                particle_count: NUM_INSTANCES,
                workgroups_per_row,
                padding: Default::default(),
            };

            queue.write_buffer(&self.simulation_buffer, 0, bytemuck::bytes_of(&uniform));
        }
        {
            // reset indirect args
            let indirect_args = DrawIndexedIndirectArgsStorage {
                index_count: INDICES.len() as u32,
                instance_count: 0,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            };

            queue.write_buffer(&self.indirect_buffer, 0, bytemuck::bytes_of(&indirect_args))
        }

        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(workgroups_per_row, workgroup_rows, 1);
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    //color: [f32; 3],
}
const QUAD_SIZE: f32 = 0.01;
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-QUAD_SIZE, -QUAD_SIZE, 0.0, 1.0],
    },
    Vertex {
        position: [QUAD_SIZE, -QUAD_SIZE, 0.0, 1.0],
    },
    Vertex {
        position: [QUAD_SIZE, QUAD_SIZE, 0.0, 1.0],
    },
    Vertex {
        position: [-QUAD_SIZE, QUAD_SIZE, 0.0, 1.0],
    },
];
const INDICES: &[u16] = &[
    0, 1, 2, //
    0, 2, 3,
];

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                //wgpu::VertexAttribute {
                //    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                //    shader_location: 1,
                //    format: wgpu::VertexFormat::Float32x3,
                //},
            ],
        }
    }
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}

struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    right: [f32; 4],
    up: [f32; 4],
    eye: [f32; 4],
    znear: f32,
    zfar: f32,
    _padding: [f32; 2], // alignment (important!)
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            view: cgmath::Matrix4::identity().into(),
            right: [1.0, 0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
            eye: [0.0, 0.0, 0.0, 0.0],
            znear: 0.1,
            zfar: 100.0,
            _padding: [0.0, 0.0],
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        use cgmath::InnerSpace;

        self.view_proj = camera.build_view_projection_matrix().into();
        let view = cgmath::Matrix4::look_at_rh(camera.eye, camera.target, camera.up);

        let forward = (camera.target - camera.eye).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();

        self.view = view.into();
        self.right = [right.x, right.y, right.z, 0.0];
        self.up = [up.x, up.y, up.z, 0.0];
        self.eye = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
        self.znear = camera.znear;
        self.zfar = camera.zfar;
    }
}

struct Instance {
    color: [f32; 4],
    translation: cgmath::Vector4<f32>,
    rotation: cgmath::Quaternion<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    color: [f32; 4],
    translation: [f32; 4],
    rotation: [f32; 4],
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            color: self.color.clone(),
            translation: self.translation.into(),
            rotation: self.rotation.into(),
        }
    }
}

impl InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SimulationUniform {
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
    particle_count: u32,
    workgroups_per_row: u32,
    padding: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawIndexedIndirectArgsStorage {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}
