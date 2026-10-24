// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The wgpu side of the viewer: the scene drawn into an offscreen picture inside an egui paint
//! callback and blitted into the panel.

mod device;
mod frame;
mod limits;
mod pipelines;
mod report;
mod resources;
mod skeleton;
mod slot;
mod target;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use eframe::egui_wgpu::{self, wgpu};
use egui::Rect;

use device::{settle, supported_samples};
pub use limits::{MAX_TEXTURE_SIDE, Rejected, check_buffer, check_texture, target_extent};
use pipelines::{BLIT_BYTES, FRAME_BYTES, Pipelines};
pub use report::{GpuReport, Unavailable};
use resources::{MaterialTexture, MeshBuffers};
use slot::Slot;
use target::Target;

use super::camera::Uniforms;
use super::scene::Scene;
use super::skeleton::Pose;

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
    drawn: Option<Drawn>,
    posed: u64,
    samples: u32,
    linear_target: bool,
    report: Arc<GpuReport>,
}

/// How the model is drawn on top of the scene: in which pose, and whether with its skeleton.
#[derive(Debug, Clone, Default)]
pub(crate) struct Look {
    /// The bone matrices of the pose to draw, or `None` for the bind pose.
    pub(crate) pose: Option<Arc<Pose>>,
    /// The width in device pixels of the skeleton lines, or `None` to leave the skeleton out.
    pub(crate) bones: Option<f32>,
}

/// What the picture in the target shows, so a repaint with the same input skips the pass.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drawn {
    pose: u64,
    bones: Option<f32>,
    serial: u64,
    uniforms: Uniforms,
    size: [u32; 2],
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

/// Builds the paint callback that draws `scene` in the `look` seen through `uniforms` into
/// `rect`, a view of `size` pixels.
#[must_use]
pub(crate) fn paint_callback(
    rect: Rect,
    scene: Arc<Scene>,
    uniforms: Uniforms,
    (size, look): ([u32; 2], Look),
) -> egui::PaintCallback {
    egui_wgpu::Callback::new_paint_callback(
        rect,
        SceneCallback {
            scene,
            uniforms,
            size,
            look,
        },
    )
}

struct SceneCallback {
    scene: Arc<Scene>,
    uniforms: Uniforms,
    size: [u32; 2],
    look: Look,
}

impl egui_wgpu::CallbackTrait for SceneCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        resources
            .get_mut::<ViewerGpu>()
            .map(|gpu| gpu.prepare(device, queue, self))
            .unwrap_or_default()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(gpu) = resources.get::<ViewerGpu>() {
            gpu.paint(pass, self.scene.serial());
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
            drawn: None,
            posed: 0,
            samples,
            linear_target: target_format.is_srgb(),
            report,
        })
    }
}
