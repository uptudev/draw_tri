use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{
        Window,
        WindowBuilder,
    },
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if!{
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn)
                .expect("Couldn't initialize logger.")
        } else {
            env_logger::init();
        }
    }
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents resizing with CSS, so web deploys must be manually set.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body/");
    }
    
    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            window_id,
            ref event,
        } if window_id == state.window().id() => 
            if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input: KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // &&mut therefore double deref needed
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                }
            
        },
        Event::RedrawRequested(window_id) if window_id == state.window().id() => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // If surface lost or outdated, reconfigure
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) =>
                    state.resize(state.size),
                Err(wgpu::SurfaceError::OutOfMemory) =>
                    *control_flow = ControlFlow::Exit,
                Err(wgpu::SurfaceError::Timeout) =>
                    log::warn!("Surface timed out."),
            }
        },
        Event::RedrawEventsCleared => {
            // RedrawRequested will only trigger once unless manually requested.
            state.window().request_redraw();
        },
        _ => {}
    });

}

struct State {
    surface:    wgpu::Surface,
    device:     wgpu::Device,
    queue:      wgpu::Queue,
    config:     wgpu::SurfaceConfiguration,
    size:       winit::dpi::PhysicalSize<u32>,
    window:     Window,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_verts: u32,
}

impl State {
    // Creating some of the wgpu types requires async
    async fn new(window: Window) -> Self {
        let size = window.inner_size();
        
        let num_verts = VERTS.len() as u32;

        // The instance is the GPU handle
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor { 
                backends:               wgpu::Backends::all(), 
                dx12_shader_compiler:   Default::default(), 
            }
        );

        // # Safety
        //
        // The surface needs to live as long as the window that has created it.
        // State owns the window, so this should be safe.
        let surface = 
            unsafe { 
                instance.create_surface(&window)
            }.unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference:       wgpu::PowerPreference::default(),
                compatible_surface:     Some(&surface),
                force_fallback_adapter: false,
            }).await.unwrap();

        let (device, queue) = 
            adapter.request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        
        // Assumes sRGB surface texture format
        let surface_format = surface_caps.formats
            .iter()
            .copied()
            .filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage:          wgpu::TextureUsages::RENDER_ATTACHMENT,
            format:         surface_format,
            width:          size.width,
            height:         size.height,
            present_mode:   wgpu::PresentMode::AutoVsync,
            alpha_mode:     surface_caps.alpha_modes[0],
            view_formats:   vec![],
        };

        let shader = device.create_shader_module(
            wgpu::include_wgsl!("tri_shader.wgsl")
        );

        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            }
        );

        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState { 
                    module: &shader, 
                    entry_point: "vs_main", 
                    buffers: &[
                        Vertex::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState { 
                    module: &shader, 
                    entry_point: "fs_main", 
                    targets: &[Some(wgpu::ColorTargetState { 
                        format: config.format, 
                        blend: Some(wgpu::BlendState::REPLACE), 
                        write_mask: wgpu::ColorWrites::ALL, 
                    })],
                }),
                primitive: wgpu::PrimitiveState { 
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    strip_index_format: None, 
                    front_face: wgpu::FrontFace::Ccw, 
                    cull_mode: Some(wgpu::Face::Back), 
                    unclipped_depth: false, 
                    polygon_mode: wgpu::PolygonMode::Fill, 
                    conservative: false, 
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState { 
                    count: 1, 
                    mask: !0, 
                    alpha_to_coverage_enabled: false 
                },
                multiview: None,
            }
        );

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTS),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        surface.configure(&device, &config);

        Self {
            window, 
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            num_verts,
        }
}

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false //TODO change this once events need handling
    }

    fn update(&mut self) {
        //TODO Make update method
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder")
            }
        );
     
        let mut render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label:              Some("Render Pass"),
                color_attachments:  &[Some(wgpu::RenderPassColorAttachment {
                    view:           &view,
                    resolve_target: None,
                    ops:            wgpu::Operations {
                        load:       wgpu::LoadOp::Clear(wgpu::Color {
                            r:      1.0,
                            g:      1.0,
                            b:      1.0,
                            a:      1.0,
                        }),
                        store:      true,
                    }
                })],
                depth_stencil_attachment: None,
            }
        );

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.num_verts, 0..1);

        // `begin_render_pass()` borrows encoder mutably.
        // `encoder.finish()` cannot be called until the borrow is released.
        drop(render_pass);
     
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout { 
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, 
            step_mode: wgpu::VertexStepMode::Vertex, 
            attributes: &Self::ATTRIBS,
        }
    }
}

const VERTS: &[Vertex] =  &[
    Vertex {
        position:   [0.0, 0.5, 0.0],
        color:      [1.0, 0.0, 0.0],
    },
    Vertex {
        position:   [-0.5, -0.5, 0.0],
        color:      [0.0, 1.0, 0.0],
    },
    Vertex {
        position:   [0.5, -0.5, 0.0],
        color:      [0.0, 0.0, 1.0],
    },
];
