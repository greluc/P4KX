// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::sync::Arc;

use super::device::guarded;
use super::limits::{Rejected, check_buffer};
use super::pipelines::{COLOUR_FORMAT, DEPTH_FORMAT};
use super::resources::{MaterialTexture, VERTEX_STRIDE, filled};
use super::{AssemblyDraw, BASE_TEXTURE, Drawn, Frame, GpuReport, Renderer, Shading};
use crate::assembly::AssemblyScene;

const SHADER: &str = include_str!("assembly.wgsl");
const INSTANCE_BYTES: u64 = 64;

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x3,
    2 => Float32x2,
];

const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x4,
    6 => Float32x4,
];

/// The vertex buffers of the instanced pipelines: the shared mesh, then the instances.
pub(super) const fn buffer_layouts() -> [Option<wgpu::VertexBufferLayout<'static>>; 2] {
    [
        Some(wgpu::VertexBufferLayout {
            array_stride: VERTEX_STRIDE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: INSTANCE_BYTES,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        }),
    ]
}
/// The pipelines that draw the instances of an Assembly: opaque ones with depth writes, then
/// translucent ones that blend over them.
#[derive(Debug)]
pub(super) struct AssemblyPipelines {
    pub(super) opaque: wgpu::RenderPipeline,
    pub(super) translucent: wgpu::RenderPipeline,
}

impl AssemblyPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        (frame, material): (&wgpu::BindGroupLayout, &wgpu::BindGroupLayout),
        samples: u32,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("p4kx-viewer-assembly"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p4kx-viewer-assembly"),
            bind_group_layouts: &[Some(frame), Some(material)],
            immediate_size: 0,
        });
        Self {
            opaque: pipeline(device, &layout, &module, samples, false),
            translucent: pipeline(device, &layout, &module, samples, true),
        }
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    samples: u32,
    translucent: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("p4kx-viewer-assembly"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &buffer_layouts(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!translucent),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOUR_FORMAT,
                blend: translucent.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The shared meshes, the instance buffer and the textures of an Assembly, destroyed when
/// dropped.
#[derive(Debug)]
pub(super) struct AssemblyBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    capacity: usize,
    textures: Vec<Option<MaterialTexture>>,
    report: Arc<GpuReport>,
}

impl AssemblyBuffers {
    fn upload(
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        (layout, sampler): (&wgpu::BindGroupLayout, &wgpu::Sampler),
        scene: &AssemblyScene,
        report: &Arc<GpuReport>,
    ) -> Result<Self, Rejected> {
        let limit = device.limits().max_buffer_size;
        let bytes = |count: usize, each: u64| {
            u64::try_from(count)
                .ok()
                .and_then(|count| count.checked_mul(each))
                .unwrap_or(u64::MAX)
        };
        let capacity = scene.item_count().max(1);
        check_buffer(bytes(scene.vertices().len(), VERTEX_STRIDE), limit)?;
        check_buffer(bytes(scene.indices().len(), 4), limit)?;
        check_buffer(bytes(capacity, INSTANCE_BYTES), limit)?;
        let vertices = filled(
            (device, queue),
            ("p4kx-viewer-assembly-vertices", wgpu::BufferUsages::VERTEX),
            bytemuck::cast_slice(scene.vertices()),
        );
        let indices = filled(
            (device, queue),
            ("p4kx-viewer-assembly-indices", wgpu::BufferUsages::INDEX),
            bytemuck::cast_slice(scene.indices()),
        );
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("p4kx-viewer-assembly-instances"),
            size: bytes(capacity, INSTANCE_BYTES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        report.buffer_created(3);
        report.assembly_created();
        let textures: Vec<Option<MaterialTexture>> = scene
            .textures()
            .iter()
            .map(|texture| {
                let pixels = texture.take_pixels();
                MaterialTexture::upload(
                    device,
                    queue,
                    layout,
                    sampler,
                    (texture.width, texture.height, &pixels),
                )
                .ok()
            })
            .collect();
        report.texture_created(textures.iter().flatten().count());
        Ok(Self {
            vertices,
            indices,
            instances,
            capacity,
            textures,
            report: Arc::clone(report),
        })
    }
}

impl Drop for AssemblyBuffers {
    fn drop(&mut self) {
        self.vertices.destroy();
        self.indices.destroy();
        self.instances.destroy();
        self.report.buffer_destroyed(3);
        self.report.assembly_destroyed();
        self.report
            .texture_destroyed(self.textures.iter().flatten().count());
    }
}

impl Renderer {
    pub(super) fn prepare_assembly(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame,
        view: &AssemblyDraw,
    ) -> Vec<wgpu::CommandBuffer> {
        let serial = view.scene.serial();
        if self.report.is_lost() || self.report.has_failed(serial) {
            return Vec::new();
        }
        self.mesh.clear();
        self.posed = 0;
        let ((), upload_failed) = guarded(device, || {
            self.upload_assembly((device, queue), &view.scene);
        });
        if upload_failed {
            self.assembly.clear();
            self.drawn = None;
            self.report.reject(serial);
            return Vec::new();
        }
        let wanted = Drawn {
            pose: 0,
            list: view.list.serial(),
            bones: None,
            serial,
            uniforms: frame.uniforms,
            size: frame.size,
            colours: frame.colours,
            shading: frame.shading,
        };
        if self.target.is_some() && self.drawn == Some(wanted) {
            return Vec::new();
        }
        let (commands, failed) = guarded(device, || self.record(device, queue, frame));
        if failed {
            self.report.fail(serial);
            self.assembly.clear();
            self.target = None;
            self.drawn = None;
            return Vec::new();
        }
        self.drawn = Some(wanted);
        self.report.drew();
        commands.into_iter().collect()
    }

    fn upload_assembly(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        scene: &AssemblyScene,
    ) {
        let serial = scene.serial();
        if self.assembly.key() == Some(serial) || self.report.is_rejected(serial) {
            return;
        }
        self.assembly.clear();
        self.drawn = None;
        let layouts = (&self.pipelines.material_layout, &self.material_sampler);
        match AssemblyBuffers::upload((device, queue), layouts, scene, &self.report) {
            Ok(buffers) => {
                self.assembly.store(serial, buffers);
                self.report.uploaded(serial);
            }
            Err(_rejected) => self.report.reject(serial),
        }
    }

    pub(super) fn write_instances(&self, queue: &wgpu::Queue, view: &AssemblyDraw) {
        let Some(buffers) = self.assembly.get(view.scene.serial()) else {
            return;
        };
        let instances = view.list.instances();
        let shown = instances.get(..buffers.capacity).unwrap_or(instances);
        queue.write_buffer(&buffers.instances, 0, bytemuck::cast_slice(shown));
    }

    pub(super) fn draw_assembly(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        view: &AssemblyDraw,
        shading: Shading,
    ) {
        let (Some(buffers), Some(white)) = (
            self.assembly.get(view.scene.serial()),
            self.textures.get(BASE_TEXTURE),
        ) else {
            return;
        };
        pass.set_bind_group(0, &self.frame_group, &[]);
        pass.set_vertex_buffer(0, buffers.vertices.slice(..));
        pass.set_vertex_buffer(1, buffers.instances.slice(..));
        pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
        let (mut instances, mut calls) = (0_u64, 0_u64);
        let assembly = &self.pipelines.assembly;
        let solid = [
            (&assembly.opaque, Some(false), true),
            (&assembly.translucent, Some(true), true),
        ];
        let hologram;
        let steps: &[_] = match (shading, self.pipelines.hologram.as_ref()) {
            (Shading::Hologram(look), Some(pipelines)) => {
                let cap = f64::from(look.cap());
                pass.set_blend_constant(wgpu::Color {
                    r: cap,
                    g: cap,
                    b: cap,
                    a: cap,
                });
                hologram = [(&pipelines.colour, None, true)];
                &hologram
            }
            _ => &solid,
        };
        for &(pipeline, only, counted) in steps {
            pass.set_pipeline(pipeline);
            for run in view.list.draws() {
                let Some(group) = view.scene.groups().get(run.group as usize) else {
                    continue;
                };
                if only.is_some_and(|translucent| group.translucent != translucent) {
                    continue;
                }
                let texture = group
                    .texture
                    .and_then(|at| buffers.textures.get(at as usize))
                    .and_then(Option::as_ref)
                    .unwrap_or(white);
                pass.set_bind_group(1, &texture.group, &[]);
                let first = group.first_index;
                pass.draw_indexed(
                    first..first.saturating_add(group.index_count),
                    0,
                    run.first..run.first.saturating_add(run.count),
                );
                if counted {
                    instances += u64::from(run.count);
                }
                calls += 1;
            }
        }
        self.report.drew_instances(instances, calls);
    }
}
