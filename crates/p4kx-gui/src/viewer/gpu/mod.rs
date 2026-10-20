//! The wgpu side of the viewer: the scene drawn into an offscreen picture inside an egui paint
//! callback and blitted into the panel.

mod device;
mod limits;
mod pipelines;
mod report;
mod resources;
mod slot;
mod target;

use std::sync::Arc;

use eframe::egui_wgpu::{self, wgpu};
use egui::Rect;

use device::{guarded, settle, supported_samples, to_f32};
pub use limits::{MAX_TEXTURE_SIDE, Rejected, check_buffer, check_texture, target_extent};
use pipelines::{BLIT_BYTES, FRAME_BYTES, Pipelines};
pub use report::{GpuReport, Unavailable};
use resources::{MaterialTexture, MeshBuffers};
use slot::Slot;
use target::Target;

use super::camera::Uniforms;
use super::scene::Scene;
use crate::theme::palette;

const BASE_TEXTURE: u64 = 0;
const MIN_TEXTURE_SIDE: u32 = 2048;
const MIN_BUFFER_BYTES: u64 = 64 * 1024 * 1024;
const WHITE: [u8; 4] = [255; 4];

/// The GPU resources of the viewer, kept in the callback resources of the egui renderer.
#[derive(Debug)]
pub(super) struct ViewerGpu {
    pipelines: Pipelines,
    frame_buffer: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    blit_buffer: wgpu::Buffer,
    blit_sampler: wgpu::Sampler,
    textures: Slot<MaterialTexture>,
    mesh: Slot<MeshBuffers>,
    target: Option<Target>,
    samples: u32,
    linear_target: bool,
    report: Arc<GpuReport>,
}

/// Creates the viewer on `state` and registers it with the renderer so paint callbacks find it.
///
/// # Errors
///
/// Returns [`Unavailable`] when the device is too limited or the shaders and pipelines cannot be
/// created; the rest of the window is not affected.
pub fn install(state: &egui_wgpu::RenderState) -> Result<Arc<GpuReport>, Unavailable> {
    let gpu = ViewerGpu::new(
        &state.device,
        &state.queue,
        &state.adapter,
        state.target_format,
    )?;
    let report = Arc::clone(&gpu.report);
    state.renderer.write().callback_resources.insert(gpu);
    Ok(report)
}

/// Builds the paint callback that draws `scene` seen through `uniforms` into `rect`, a view of
/// `size` pixels.
#[must_use]
pub(crate) fn paint_callback(
    rect: Rect,
    scene: Arc<Scene>,
    uniforms: Uniforms,
    size: [u32; 2],
) -> egui::PaintCallback {
    egui_wgpu::Callback::new_paint_callback(
        rect,
        SceneCallback {
            scene,
            uniforms,
            size,
        },
    )
}

struct SceneCallback {
    scene: Arc<Scene>,
    uniforms: Uniforms,
    size: [u32; 2],
}

impl egui_wgpu::CallbackTrait for SceneCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(gpu) = resources.get_mut::<ViewerGpu>() {
            gpu.prepare(device, queue, encoder, self);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(gpu) = resources.get::<ViewerGpu>() {
            gpu.paint(pass);
        }
    }
}

impl ViewerGpu {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        adapter: &wgpu::Adapter,
        target_format: wgpu::TextureFormat,
    ) -> Result<Self, Unavailable> {
        let limits = device.limits();
        if limits.max_texture_dimension_2d < MIN_TEXTURE_SIDE {
            return Err(Unavailable::TextureSide(limits.max_texture_dimension_2d));
        }
        if limits.max_buffer_size < MIN_BUFFER_BYTES {
            return Err(Unavailable::BufferSize(limits.max_buffer_size));
        }
        let samples = supported_samples(adapter);
        let report = Arc::new(GpuReport::default());
        let lost = Arc::clone(&report);
        device.set_device_lost_callback(move |_reason, _message| lost.device_lost());
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let gpu = Self::create(device, queue, samples, target_format, report);
        settle(validation.pop())
            .flatten()
            .map_or(gpu, |error| Err(Unavailable::Setup(error.to_string())))
    }

    fn create(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        samples: u32,
        target_format: wgpu::TextureFormat,
        report: Arc<GpuReport>,
    ) -> Result<Self, Unavailable> {
        let pipelines = Pipelines::new(device, samples, target_format);
        let buffer = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let frame_buffer = buffer("p4kx-viewer-frame", FRAME_BYTES);
        let blit_buffer = buffer("p4kx-viewer-blit", BLIT_BYTES);
        let frame_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p4kx-viewer-frame"),
            layout: &pipelines.frame_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_buffer.as_entire_binding(),
            }],
        });
        let material_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("p4kx-viewer-material"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("p4kx-viewer-blit"),
            ..wgpu::SamplerDescriptor::default()
        });
        let white = MaterialTexture::upload(
            device,
            queue,
            &pipelines.material_layout,
            &material_sampler,
            (1, 1, &WHITE),
        )
        .map_err(|error| Unavailable::Setup(error.to_string()))?;
        let mut textures = Slot::default();
        textures.store(BASE_TEXTURE, white);
        Ok(Self {
            pipelines,
            frame_buffer,
            frame_group,
            blit_buffer,
            blit_sampler,
            textures,
            mesh: Slot::default(),
            target: None,
            samples,
            linear_target: target_format.is_srgb(),
            report,
        })
    }

    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &SceneCallback,
    ) {
        if self.report.is_broken() {
            return;
        }
        let ((), upload_failed) = guarded(device, || self.upload(device, &frame.scene));
        if upload_failed {
            self.mesh.clear();
            self.report.reject(frame.scene.serial());
        }
        let ((), draw_failed) = guarded(device, || {
            self.fit_target(device, frame.size);
            self.write_uniforms(queue, frame);
            self.draw(encoder, frame);
        });
        if draw_failed {
            self.report.fail();
            self.mesh.clear();
            self.target = None;
        }
    }

    fn upload(&mut self, device: &wgpu::Device, scene: &Scene) {
        let serial = scene.serial();
        if self.mesh.key() == Some(serial) || self.report.is_rejected(serial) {
            return;
        }
        self.mesh.clear();
        match MeshBuffers::upload(device, scene, &self.report) {
            Ok(buffers) => {
                self.mesh.store(serial, buffers);
                self.report.uploaded(serial);
            }
            Err(_rejected) => self.report.reject(serial),
        }
    }

    fn fit_target(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        let wanted = target_extent(size, device.limits().max_texture_dimension_2d);
        if self.target.as_ref().map(|target| target.extent) != Some(wanted) {
            self.target = Some(Target::new(
                device,
                wanted,
                self.samples,
                (
                    &self.pipelines.blit_layout,
                    &self.blit_sampler,
                    &self.blit_buffer,
                ),
            ));
        }
    }

    fn write_uniforms(&self, queue: &wgpu::Queue, frame: &SceneCallback) {
        let uniforms = &frame.uniforms;
        let packed: [[f32; 4]; 6] = [
            uniforms.view_proj[0],
            uniforms.view_proj[1],
            uniforms.view_proj[2],
            uniforms.view_proj[3],
            uniforms.eye,
            uniforms.key_light,
        ];
        queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&packed));
        let extent = self.target.as_ref().map_or([1, 1], |target| target.extent);
        let scale = |side: u32, whole: u32| {
            f32::from(u16::try_from(side).unwrap_or(u16::MAX))
                / f32::from(u16::try_from(whole).unwrap_or(u16::MAX)).max(1.0)
        };
        let blit: [f32; 4] = [
            scale(frame.size[0].min(extent[0]), extent[0]),
            scale(frame.size[1].min(extent[1]), extent[1]),
            f32::from(u8::from(self.linear_target)),
            0.0,
        ];
        queue.write_buffer(&self.blit_buffer, 0, bytemuck::bytes_of(&blit));
    }

    fn draw(&self, encoder: &mut wgpu::CommandEncoder, frame: &SceneCallback) {
        let Some(target) = &self.target else {
            return;
        };
        let background = f64::from(palette::SURFACE_INSET.r()) / 255.0;
        let clear = wgpu::Color {
            r: background,
            g: f64::from(palette::SURFACE_INSET.g()) / 255.0,
            b: f64::from(palette::SURFACE_INSET.b()) / 255.0,
            a: 1.0,
        };
        let (view, resolve) = target
            .multisampled
            .as_ref()
            .map_or((&target.resolved, None), |samples| {
                (samples, Some(&target.resolved))
            });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("p4kx-viewer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: if resolve.is_some() {
                        wgpu::StoreOp::Discard
                    } else {
                        wgpu::StoreOp::Store
                    },
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let width = frame.size[0].min(target.extent[0]);
        let height = frame.size[1].min(target.extent[1]);
        pass.set_viewport(0.0, 0.0, to_f32(width), to_f32(height), 0.0, 1.0);
        pass.set_scissor_rect(0, 0, width, height);
        let (Some(mesh), Some(material)) = (
            self.mesh.get(frame.scene.serial()),
            self.textures.get(BASE_TEXTURE),
        ) else {
            return;
        };
        pass.set_pipeline(&self.pipelines.scene);
        pass.set_bind_group(0, &self.frame_group, &[]);
        pass.set_bind_group(1, &material.group, &[]);
        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }

    fn paint(&self, pass: &mut wgpu::RenderPass<'static>) {
        if self.report.is_broken() {
            return;
        }
        if let Some(target) = &self.target {
            pass.set_pipeline(&self.pipelines.blit);
            pass.set_bind_group(0, &target.blit_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
