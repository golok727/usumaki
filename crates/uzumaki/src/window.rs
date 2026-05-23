use anyhow::{Context, Result};
use std::sync::Arc;
use vello::peniko::Color;
use vello::{AaSupport, RenderParams, RendererOptions, Scene};

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::EventLoopProxy;
use winit::window::Window as WinitWindow;

use crate::app::{UserEvent, WindowEntryId, WindowShared};
use crate::cursor::UzCursorIcon;
use crate::gpu::GpuContext;
use crate::text::TextRenderer;

/// Per-window GPU resources owned by the main thread. The JS thread never
/// touches anything in here. Built frames arrive via [`WindowShared::pending_frame`].
pub struct GpuWindow {
    pub winit_window: Arc<WinitWindow>,
    pub shared: Arc<WindowShared>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: vello::Renderer,
    vello_target: Option<(wgpu::Texture, wgpu::TextureView)>,
    gpu: GpuContext,
    transparent: bool,
    valid_surface: bool,
}

impl GpuWindow {
    pub fn new(
        gpu: &GpuContext,
        winit_window: Arc<WinitWindow>,
        shared: Arc<WindowShared>,
        transparent: bool,
    ) -> Result<Self> {
        let surface = gpu
            .instance
            .create_surface(winit_window.clone())
            .context("Error creating surface")?;

        let size = winit_window.inner_size();
        let valid_surface = size.width != 0 && size.height != 0;

        let surface_caps = surface.get_capabilities(&gpu.adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
            .unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
        let alpha_mode = choose_alpha_mode(&surface_caps.alpha_modes, transparent);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![format],
        };
        surface.configure(&gpu.device, &surface_config);

        let renderer = vello::Renderer::new(
            &gpu.device,
            RendererOptions {
                antialiasing_support: AaSupport::area_only(),
                ..Default::default()
            },
        )
        .context("Error creating renderer")?;

        Ok(Self {
            winit_window,
            shared,
            surface,
            surface_config,
            renderer,
            vello_target: None,
            gpu: gpu.clone(),
            transparent,
            valid_surface,
        })
    }

    pub fn id(&self) -> winit::window::WindowId {
        self.winit_window.id()
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        if self.transparent == transparent {
            return;
        }
        self.transparent = transparent;
        self.reconfigure_surface_alpha();
        if self.valid_surface {
            self.surface
                .configure(&self.gpu.device, &self.surface_config);
        }
        self.winit_window.set_transparent(transparent);
        self.winit_window.request_redraw();
    }

    /// Present the scene the JS thread parked in `shared.pending_frame`. If
    /// no frame is parked (e.g. JS hasn't responded yet) this is a no-op.
    pub fn present_pending_frame(&mut self) {
        if !self.valid_surface {
            return;
        }
        let Some(scene) = self.shared.pending_frame.lock().unwrap().take() else {
            return;
        };
        self.present_scene(&scene);
    }

    fn present_scene(&mut self, scene: &Scene) {
        let device = &self.gpu.device;
        let queue = &self.gpu.queue;
        let width = self.surface_config.width;
        let height = self.surface_config.height;

        let target_view = Self::ensure_vello_target(&mut self.vello_target, device, width, height);
        let render_params = RenderParams {
            base_color: if self.transparent {
                Color::from_rgba8(0, 0, 0, 0)
            } else {
                Color::from_rgba8(24, 24, 37, 255)
            },
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.renderer
            .render_to_texture(device, queue, scene, target_view, &render_params)
            .expect("Failed to render");

        let surface_texture = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.surface.configure(device, &self.surface_config);
                match self.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Failed to get surface texture: {e}");
                        return;
                    }
                }
            }
        };
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_config.format),
                ..Default::default()
            });

        let blitter = wgpu::util::TextureBlitter::new(device, self.surface_config.format);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        blitter.copy(device, &mut encoder, target_view, &surface_view);
        queue.submit([encoder.finish()]);
        surface_texture.present();
    }

    fn ensure_vello_target<'a>(
        target: &'a mut Option<(wgpu::Texture, wgpu::TextureView)>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> &'a wgpu::TextureView {
        if target.is_none() {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("vello_target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            *target = Some((texture, view));
        }
        &target.as_ref().unwrap().1
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.reconfigure_surface_alpha();
        self.surface
            .configure(&self.gpu.device, &self.surface_config);
        self.vello_target = None;
    }

    pub fn on_resize(&mut self, width: u32, height: u32) -> bool {
        if width != 0 && height != 0 {
            self.resize_surface(width, height);
            self.valid_surface = true;
            true
        } else {
            self.valid_surface = false;
            false
        }
    }

    fn reconfigure_surface_alpha(&mut self) {
        let surface_caps = self.surface.get_capabilities(&self.gpu.adapter);
        self.surface_config.alpha_mode =
            choose_alpha_mode(&surface_caps.alpha_modes, self.transparent);
    }
}

/// JS-thread handle to a window. Holds the [`TextRenderer`] (font/layout
/// contexts used by every layout pass) and the `Arc<WindowShared>` so layout
/// can read size/scale and parked frames are published in place.
///
/// All winit interaction goes through `proxy` so the main thread retains
/// exclusive ownership of platform-specific APIs.
pub struct Window {
    pub shared: Arc<WindowShared>,
    pub text_renderer: TextRenderer,
    proxy: EventLoopProxy<UserEvent>,
    current_cursor: UzCursorIcon,
}

impl Window {
    pub fn new(shared: Arc<WindowShared>, proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            shared,
            text_renderer: TextRenderer::new(),
            proxy,
            current_cursor: UzCursorIcon::Default,
        }
    }

    pub fn id(&self) -> WindowEntryId {
        self.shared.window_id
    }

    pub fn scale_factor(&self) -> f64 {
        self.shared.load_scale_factor()
    }

    pub fn inner_size(&self) -> (u32, u32) {
        self.shared.load_inner_size()
    }

    pub fn request_redraw(&self) {
        self.shared.winit.request_redraw();
    }

    pub fn set_cursor(&mut self, icon: UzCursorIcon) {
        if self.current_cursor == icon {
            return;
        }
        self.current_cursor = icon;
        let _ = self.proxy.send_event(UserEvent::SetCursor {
            id: self.shared.window_id,
            icon,
        });
    }

    pub fn set_ime_cursor_area(&self, position: LogicalPosition<f64>, size: LogicalSize<f32>) {
        let _ = self.proxy.send_event(UserEvent::SetImeArea {
            id: self.shared.window_id,
            position,
            size,
        });
    }
}

pub(crate) fn choose_alpha_mode(
    modes: &[wgpu::CompositeAlphaMode],
    transparent: bool,
) -> wgpu::CompositeAlphaMode {
    use wgpu::CompositeAlphaMode::*;

    let preferred: &[wgpu::CompositeAlphaMode] = if transparent {
        &[PreMultiplied, PostMultiplied, Inherit]
    } else {
        &[Opaque]
    };

    preferred
        .iter()
        .find(|mode| modes.contains(mode))
        .or_else(|| modes.first())
        .copied()
        .unwrap_or(Auto)
}

#[cfg(test)]
mod tests {
    use super::choose_alpha_mode;

    #[test]
    fn transparent_windows_prefer_compositing_alpha_modes() {
        let modes = [
            wgpu::CompositeAlphaMode::Opaque,
            wgpu::CompositeAlphaMode::PostMultiplied,
        ];

        assert_eq!(
            choose_alpha_mode(&modes, true),
            wgpu::CompositeAlphaMode::PostMultiplied
        );
    }

    #[test]
    fn opaque_windows_prefer_opaque_alpha_mode() {
        let modes = [
            wgpu::CompositeAlphaMode::Inherit,
            wgpu::CompositeAlphaMode::Opaque,
        ];

        assert_eq!(
            choose_alpha_mode(&modes, false),
            wgpu::CompositeAlphaMode::Opaque
        );
    }

    #[test]
    fn alpha_mode_selection_falls_back_to_supported_mode() {
        let modes = [wgpu::CompositeAlphaMode::Opaque];

        assert_eq!(
            choose_alpha_mode(&modes, true),
            wgpu::CompositeAlphaMode::Opaque
        );
    }
}
