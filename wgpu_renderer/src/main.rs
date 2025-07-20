#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

fn tessellate_rectangle(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        Vertex { position: [x, y], color },
        Vertex { position: [x + width, y], color },
        Vertex { position: [x + width, y + height], color },
        Vertex { position: [x, y + height], color },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    (vertices, indices)
}

fn tessellate_circle(xc: f32, yc: f32, radius: f32, color: [f32; 4]) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = vec![Vertex { position: [xc, yc], color }];
    let mut indices = vec![];
    let step = 0.1 / radius;
    let mut angle: f32 = 0.0;
    let mut i = 1;
    while angle < 2.0 * std::f32::consts::PI {
        let x = xc + radius * angle.cos();
        let y = yc + radius * angle.sin();
        vertices.push(Vertex { position: [x, y], color });
        indices.push(0);
        indices.push(i);
        indices.push(i + 1);
        i += 1;
        angle += step;
    }
    let len = indices.len();
    if len > 0 {
        indices[len - 1] = 1;
    }

    (vertices, indices)
}

fn recursive_bezier(
    x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32,
    vertices: &mut Vec<Vertex>, color: [f32; 4]
) {
    let tolerance = 0.1;
    let x12 = (x1 + x2) / 2.0;
    let y12 = (y1 + y2) / 2.0;
    let x23 = (x2 + x3) / 2.0;
    let y23 = (y2 + y3) / 2.0;
    let x34 = (x3 + x4) / 2.0;
    let y34 = (y3 + y4) / 2.0;
    let x123 = (x12 + x23) / 2.0;
    let y123 = (y12 + y23) / 2.0;
    let x234 = (x23 + x34) / 2.0;
    let y234 = (y23 + y34) / 2.0;
    let x1234 = (x123 + x234) / 2.0;
    let y1234 = (y123 + y234) / 2.0;

    let dx = x4 - x1;
    let dy = y4 - y1;
    let d2 = ((x2 - x4) * dy - (y2 - y4) * dx).abs();
    let d3 = ((x3 - x4) * dy - (y3 - y4) * dx).abs();

    if (d2 + d3) * (d2 + d3) < tolerance * (dx * dx + dy * dy) {
        vertices.push(Vertex { position: [x4, y4], color });
        return;
    }

    recursive_bezier(x1, y1, x12, y12, x123, y123, x1234, y1234, vertices, color);
    recursive_bezier(x1234, y1234, x234, y234, x34, y34, x4, y4, vertices, color);
}

fn tessellate_bezier(
    x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32,
    width: f32, color: [f32; 4]
) -> (Vec<Vertex>, Vec<u16>) {
    let mut points = vec![];
    recursive_bezier(x1, y1, x2, y2, x3, y3, x4, y4, &mut points, color);

    let mut vertices = vec![];
    let mut indices = vec![];
    for i in 0..points.len() - 1 {
        let p1 = points[i].position;
        let p2 = points[i + 1].position;
        let (v, mut ind) = tessellate_line(p1, p2, width, color);
        for i in &mut ind {
            *i += vertices.len() as u16;
        }
        vertices.extend(v);
        indices.extend(ind);
    }
    (vertices, indices)
}

fn tessellate_line(p1: [f32; 2], p2: [f32; 2], width: f32, color: [f32; 4]) -> (Vec<Vertex>, Vec<u16>) {
    let dx = p2[0] - p1[0];
    let dy = p2[1] - p1[1];

    let length = (dx * dx + dy * dy).sqrt();
    if length == 0.0 {
        return (vec![], vec![]);
    }
    let nx = -dy / length;
    let ny = dx / length;

    let half_w = width / 2.0;

    let a = [p1[0] + nx * half_w, p1[1] + ny * half_w];
    let b = [p2[0] + nx * half_w, p2[1] + ny * half_w];
    let c = [p2[0] - nx * half_w, p2[1] - ny * half_w];
    let d = [p1[0] - nx * half_w, p1[1] - ny * half_w];

    let vertices = vec![
        Vertex { position: a, color },
        Vertex { position: b, color },
        Vertex { position: c, color },
        Vertex { position: d, color },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    (vertices, indices)
}


struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
}

impl State {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            device,
            queue,
            render_pipeline,
        }
    }

    fn render(&self, texture_view: &wgpu::TextureView, vertices: &[Vertex], indices: &[u16]) {
        use wgpu::util::DeviceExt;
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}

fn main() {
    env_logger::init();
    pollster::block_on(State::new());
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::path::Path;

    #[test]
    fn test_render_square() {
        let state = pollster::block_on(State::new());
        let texture_size = 256u32;
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = state.device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let (vertices, indices) = tessellate_rectangle(-0.5, -0.5, 1.0, 1.0, [1.0, 0.0, 0.0, 1.0]);

        state.render(&texture_view, &vertices, &indices);

        let get_texture_data = async {
            let mut encoder =
                state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer_size = (texture_size * texture_size * 4) as u64;
            let buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * texture_size),
                        rows_per_image: Some(texture_size),
                    },
                },
                texture_desc.size,
            );
            state.queue.submit(Some(encoder.finish()));

            let buffer_slice = buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            state.device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let data = buffer_slice.get_mapped_range().to_vec();
            buffer.unmap();
            data
        };

        let data = pollster::block_on(get_texture_data);

        let snapshot_path = Path::new("snapshot_square.png");
        if !snapshot_path.exists() {
            let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_raw(texture_size, texture_size, data).unwrap();
            image.save(snapshot_path).unwrap();
            println!("Snapshot created at snapshot_square.png");
        } else {
            let snapshot = image::open(snapshot_path).unwrap().to_rgba8();
            let snapshot_data = snapshot.into_raw();
            assert_eq!(data, snapshot_data, "Snapshot does not match");
        }
    }

    #[test]
    fn test_render_circle() {
        let state = pollster::block_on(State::new());
        let texture_size = 256u32;
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = state.device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let (vertices, indices) = tessellate_circle(0.0, 0.0, 0.5, [0.0, 1.0, 0.0, 1.0]);

        state.render(&texture_view, &vertices, &indices);

        let get_texture_data = async {
            let mut encoder =
                state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer_size = (texture_size * texture_size * 4) as u64;
            let buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * texture_size),
                        rows_per_image: Some(texture_size),
                    },
                },
                texture_desc.size,
            );
            state.queue.submit(Some(encoder.finish()));

            let buffer_slice = buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            state.device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let data = buffer_slice.get_mapped_range().to_vec();
            buffer.unmap();
            data
        };

        let data = pollster::block_on(get_texture_data);

        let snapshot_path = Path::new("snapshot_circle.png");
        if !snapshot_path.exists() {
            let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_raw(texture_size, texture_size, data).unwrap();
            image.save(snapshot_path).unwrap();
            println!("Snapshot created at snapshot_circle.png");
        } else {
            let snapshot = image::open(snapshot_path).unwrap().to_rgba8();
            let snapshot_data = snapshot.into_raw();
            assert_eq!(data, snapshot_data, "Snapshot does not match");
        }
    }

    #[test]
    fn test_render_bezier() {
        let state = pollster::block_on(State::new());
        let texture_size = 256u32;
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = state.device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let (vertices, indices) = tessellate_bezier(
            -0.5, 0.5, -0.25, -0.5, 0.25, 0.5, 0.5, -0.5, 0.02, [0.0, 0.0, 1.0, 1.0]
        );

        state.render(&texture_view, &vertices, &indices);

        let get_texture_data = async {
            let mut encoder =
                state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer_size = (texture_size * texture_size * 4) as u64;
            let buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * texture_size),
                        rows_per_image: Some(texture_size),
                    },
                },
                texture_desc.size,
            );
            state.queue.submit(Some(encoder.finish()));

            let buffer_slice = buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            state.device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let data = buffer_slice.get_mapped_range().to_vec();
            buffer.unmap();
            data
        };

        let data = pollster::block_on(get_texture_data);

        let snapshot_path = Path::new("snapshot_bezier.png");
        if !snapshot_path.exists() {
            let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_raw(texture_size, texture_size, data).unwrap();
            image.save(snapshot_path).unwrap();
            println!("Snapshot created at snapshot_bezier.png");
        } else {
            let snapshot = image::open(snapshot_path).unwrap().to_rgba8();
            let snapshot_data = snapshot.into_raw();
            assert_eq!(data, snapshot_data, "Snapshot does not match");
        }
    }

    #[test]
    fn test_render_combined() {
        let state = pollster::block_on(State::new());

        let texture_size = 256u32;

        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = state.device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let (quad_vertices, quad_indices) = tessellate_rectangle(-0.5, -0.5, 1.0, 1.0, [1.0, 0.0, 0.0, 1.0]);
        let (circle_vertices, circle_indices) = tessellate_circle(0.0, 0.0, 0.75, [0.0, 1.0, 0.0, 1.0]);
        let (bezier_vertices, bezier_indices) = tessellate_bezier(
            -0.5, 0.5, -0.25, -0.5, 0.25, 0.5, 0.5, -0.5, 0.02, [0.0, 0.0, 1.0, 1.0]
        );

        let mut vertices = vec![];
        let mut indices = vec![];

        let mut vertex_offset = 0;
        for (v, i) in [(quad_vertices, quad_indices), (circle_vertices, circle_indices), (bezier_vertices, bezier_indices)] {
            vertices.extend(v);
            indices.extend(i.iter().map(|i| i + vertex_offset));
            vertex_offset = vertices.len() as u16;
        }


        state.render(&texture_view, &vertices, &indices);

        let get_texture_data = async {
            let mut encoder =
                state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer_size = (texture_size * texture_size * 4) as u64;
            let buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * texture_size),
                        rows_per_image: Some(texture_size),
                    },
                },
                texture_desc.size,
            );
            state.queue.submit(Some(encoder.finish()));

            let buffer_slice = buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            state.device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let data = buffer_slice.get_mapped_range().to_vec();
            buffer.unmap();
            data
        };

        let data = pollster::block_on(get_texture_data);

        let snapshot_path = Path::new("snapshot_combined.png");
        if !snapshot_path.exists() {
            let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_raw(texture_size, texture_size, data).unwrap();
            image.save(snapshot_path).unwrap();
            println!("Snapshot created at snapshot_combined.png");
        } else {
            let snapshot = image::open(snapshot_path).unwrap().to_rgba8();
            let snapshot_data = snapshot.into_raw();
            assert_eq!(data, snapshot_data, "Snapshot does not match");
        }
    }
}
