#[cfg(all(target_os = "horizon", feature = "deko3d"))]
use std::borrow::Cow;

#[cfg(all(target_os = "horizon", feature = "deko3d"))]
fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DEKO3D,
        ..Default::default()
    });
    let surface = instance
        .create_surface_deko3d_default(wgpu::Deko3dDefaultSurface)
        .expect("the explicit Deko3D default surface must be available");
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .expect("the explicit Deko3D adapter must be available");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("deko3d explicit triangle device"),
        required_features: wgpu::Features::PASSTHROUGH_SHADERS
            | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .expect("the Deko3D device must be available");

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width: 1280,
        height: 720,
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    let vertices: [[f32; 2]; 3] = [[-0.5, -0.5], [0.5, -0.5], [0.0, 0.5]];
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("deko3d explicit triangle vertices"),
        size: std::mem::size_of_val(&vertices) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::MAP_WRITE,
        mapped_at_creation: true,
    });
    vertex_buffer
        .get_mapped_range_mut()
        .copy_from_slice(bytemuck::cast_slice(&vertices));
    vertex_buffer.unmap();

    let vertex_shader = unsafe {
        device.create_shader_module_deko3d_dksh(wgpu::Deko3dDkshShaderModuleDescriptor {
            label: Some("deko3d explicit triangle vertex"),
            num_workgroups: (0, 0, 0),
            dksh: Cow::Borrowed(include_bytes!("../assets/triangle_vsh.dksh")),
        })
    };
    let fragment_shader = unsafe {
        device.create_shader_module_deko3d_dksh(wgpu::Deko3dDkshShaderModuleDescriptor {
            label: Some("deko3d explicit triangle fragment"),
            num_workgroups: (0, 0, 0),
            dksh: Cow::Borrowed(include_bytes!("../assets/color_fsh.dksh")),
        })
    };
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("deko3d explicit triangle layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("deko3d explicit triangle pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_shader,
            entry_point: Some("main"),
            buffers: &[vertex_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &fragment_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    let frame = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame)
        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        result => panic!("the Deko3D default surface must acquire a frame: {result:?}"),
    };
    let view = frame.texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("deko3d explicit triangle pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.draw(0..3, 0..1);
    }
    queue.submit([encoder.finish()]);
    frame.present();
}

#[cfg(not(all(target_os = "horizon", feature = "deko3d")))]
fn main() {
    panic!("build this proof with --target aarch64-none-elf --features deko3d");
}
