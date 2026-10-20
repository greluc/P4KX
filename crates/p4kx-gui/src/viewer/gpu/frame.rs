use eframe::egui_wgpu::wgpu;

use super::device::{guarded, to_f32};
use super::resources::MeshBuffers;
use super::target::Target;
use super::{BASE_TEXTURE, Drawn, SceneCallback, ViewerGpu, target_extent};
use crate::theme::palette;
use crate::viewer::scene::Scene;

impl ViewerGpu {
    /// Uploads the scene if it is new and records the pass that draws it, returning the command
    /// buffer to submit, or nothing when the picture is current or the work failed.
    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &SceneCallback,
    ) -> Vec<wgpu::CommandBuffer> {
        let serial = frame.scene.serial();
        if self.report.is_lost() || self.report.has_failed(serial) {
            return Vec::new();
        }
        let ((), upload_failed) = guarded(device, || {
            self.upload((device, queue), frame.scene.as_ref());
        });
        if upload_failed {
            self.mesh.clear();
            self.drawn = None;
            self.report.reject(serial);
            return Vec::new();
        }
        let wanted = Drawn {
            serial,
            uniforms: frame.uniforms,
            size: frame.size,
        };
        if self.target.is_some() && self.drawn == Some(wanted) {
            return Vec::new();
        }
        let (commands, failed) = guarded(device, || self.record(device, queue, frame));
        if failed {
            self.report.fail(serial);
            self.mesh.clear();
            self.target = None;
            self.drawn = None;
            return Vec::new();
        }
        self.drawn = Some(wanted);
        self.report.drew();
        commands.into_iter().collect()
    }

    /// Blits the picture of the scene with this `serial` into the panel.
    pub(super) fn paint(&self, pass: &mut wgpu::RenderPass<'static>, serial: u64) {
        if self.report.is_lost() || self.report.has_failed(serial) {
            return;
        }
        let current = self.drawn.is_some_and(|drawn| drawn.serial == serial);
        if let (true, Some(target)) = (current, &self.target) {
            pass.set_pipeline(&self.pipelines.blit);
            pass.set_bind_group(0, &target.blit_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    fn upload(&mut self, (device, queue): (&wgpu::Device, &wgpu::Queue), scene: &Scene) {
        let serial = scene.serial();
        if self.mesh.key() == Some(serial) || self.report.is_rejected(serial) {
            return;
        }
        self.mesh.clear();
        self.drawn = None;
        match MeshBuffers::upload((device, queue), scene, &self.report) {
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
            self.drawn = None;
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

    fn record(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &SceneCallback,
    ) -> Option<wgpu::CommandBuffer> {
        self.fit_target(device, frame.size);
        self.write_uniforms(queue, frame);
        let target = self.target.as_ref()?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("p4kx-viewer"),
        });
        self.draw(&mut encoder, target, frame);
        Some(encoder.finish())
    }

    fn draw(&self, encoder: &mut wgpu::CommandEncoder, target: &Target, frame: &SceneCallback) {
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
                    load: wgpu::LoadOp::Clear(background()),
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
}

/// The inset surface colour as the linear value the sRGB picture encodes back to it.
fn background() -> wgpu::Color {
    let linear = |channel: u8| {
        let encoded = f64::from(channel) / 255.0;
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    };
    wgpu::Color {
        r: linear(palette::SURFACE_INSET.r()),
        g: linear(palette::SURFACE_INSET.g()),
        b: linear(palette::SURFACE_INSET.b()),
        a: 1.0,
    }
}
