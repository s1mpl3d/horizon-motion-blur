// Motion Blur: once the world is drawn, each pixel is blurred along the way it moved on screen
// since the last frame. The motion comes from the camera: the world's depth gives each pixel's
// place, last frame's matrices give where that place was then (the reprojection of
// natural-motionblur and Noryea's motionblur). The hand, HUD and screens are drawn after it, so
// they stay sharp. This module lives in the renderer crate: the game half only hands it the
// settings (`publish`), the render thread does the rest (`needs`, `draw`).
use glam::{DVec3, Mat4};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const ID: &str = "motion_blur";
pub const NAME: &str = "Motion Blur";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str =
    "Blurs the world along the camera's motion, like a camera's shutter. The hand and HUD stay sharp.";

/// Shutter, in percent of a frame's motion (of a 60 FPS frame's with `steady`).
pub const STRENGTHS: [u8; 6] = [25, 50, 75, 100, 150, 200];
/// Samples along each pixel's motion.
pub const SAMPLES: [u8; 4] = [8, 12, 16, 24];
/// The longest blur, in parts of the screen.
const MAX_LENGTH: f32 = 0.08;
/// A camera that moved further than this in a frame teleported: no blur that frame.
const TELEPORT: f64 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub strength: u8,
    pub samples: u8,
    /// The same blur at any frame rate (as at 60 FPS), not less blur the faster the game runs.
    pub steady: bool,
}

impl Settings {
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let num = |key: &str, allowed: &[u8], default: u8| {
            get(key)
                .and_then(|v| v.trim().parse::<u8>().ok())
                .filter(|v| allowed.contains(v))
                .unwrap_or(default)
        };
        Self {
            strength: num("strength", &STRENGTHS, 75),
            samples: num("samples", &SAMPLES, 12),
            steady: get("steady").is_none_or(|v| v.trim() != "false"),
        }
    }
    pub fn labels(&self) -> Vec<String> {
        vec![
            format!("Strength: {}%", self.strength),
            format!("Quality: {} samples", self.samples),
            format!("Same at any FPS: {}", if self.steady { "ON" } else { "OFF" }),
        ]
    }
    pub fn click(&mut self, i: usize) {
        let next = |list: &[u8], v: u8| {
            let at = list.iter().position(|&x| x == v).unwrap_or(0);
            list[(at + 1) % list.len()]
        };
        match i {
            0 => self.strength = next(&STRENGTHS, self.strength),
            1 => self.samples = next(&SAMPLES, self.samples),
            2 => self.steady = !self.steady,
            _ => {}
        }
    }
    pub fn stored(&self) -> [(&'static str, String); 3] {
        [
            ("strength", self.strength.to_string()),
            ("samples", self.samples.to_string()),
            ("steady", self.steady.to_string()),
        ]
    }
    /// How much of a frame's motion to blur, given the frame's length in seconds.
    pub fn scale(&self, frame_secs: f32) -> f32 {
        let shutter = f32::from(self.strength) / 100.0;
        if !self.steady || !(frame_secs > 0.0) {
            return shutter;
        }
        shutter * ((1.0 / 60.0) / frame_secs).clamp(0.25, 8.0)
    }
}

// What the game thread hands the render thread.
static ON: AtomicBool = AtomicBool::new(false);
static SCALE: AtomicU32 = AtomicU32::new(0);
static SAMPLE_COUNT: AtomicU32 = AtomicU32::new(12);

struct Shared {
    settings: Option<Settings>,
    /// The frame's length, smoothed.
    frame_secs: f32,
}
static SHARED: Mutex<Shared> = Mutex::new(Shared {
    settings: None,
    frame_secs: 0.0,
});

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    *s.settings.get_or_insert_with(|| Settings::read(get))
}

/// The settings page changed them.
pub fn set_settings(settings: Settings) {
    SHARED.lock().unwrap_or_else(|e| e.into_inner()).settings = Some(settings);
}

/// Once a frame on the game thread: whether the mod is on, and the frame's length.
pub fn publish(on: bool, frame_secs: f32, get: impl Fn(&str) -> Option<String>) {
    let settings = settings(get);
    let secs = {
        let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
        if frame_secs.is_finite() && frame_secs > 0.0 && frame_secs < 1.0 {
            s.frame_secs = if s.frame_secs > 0.0 {
                s.frame_secs + (frame_secs - s.frame_secs) * 0.2
            } else {
                frame_secs
            };
        }
        s.frame_secs
    };
    SCALE.store(settings.scale(secs).to_bits(), Ordering::Relaxed);
    SAMPLE_COUNT.store(u32::from(settings.samples), Ordering::Relaxed);
    ON.store(on, Ordering::Relaxed);
}

/// Asked as each frame is built: the blur reads the world's depth.
pub fn needs(world_depth: &mut bool) {
    if ON.load(Ordering::Relaxed) {
        *world_depth = true;
    }
}

const SHADER: &str = "
struct U {
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    // the camera's move since last frame
    delta: vec4<f32>,
    // scale, samples, longest blur
    params: vec4<f32>,
    // colour size, depth size
    sizes: vec4<f32>,
}
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var color: texture_2d<f32>;
@group(0) @binding(2) var depth: texture_depth_2d;

@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let size = u.sizes.xy;
    let last = vec2<i32>(size) - 1;
    let here = textureLoad(color, vec2<i32>(at.xy), 0);
    let uv = at.xy / size;
    let dlast = vec2<i32>(u.sizes.zw) - 1;
    let d = textureLoad(depth, clamp(vec2<i32>(uv * u.sizes.zw), vec2(0), dlast), 0);
    // camera-relative, homogeneous: the sky (reversed depth 0) is a direction
    let p = u.inv_view_proj * vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, d, 1.0);
    let prev = u.prev_view_proj * vec4(p.xyz + u.delta.xyz * p.w, p.w);
    if prev.w <= 1e-6 {
        return here;
    }
    let prev_uv = vec2(prev.x / prev.w * 0.5 + 0.5, 0.5 - prev.y / prev.w * 0.5);
    var v = (uv - prev_uv) * u.params.x;
    let len = length(v);
    if len > u.params.z {
        v = v * (u.params.z / len);
    }
    if len * max(size.x, size.y) < 0.5 {
        return here;
    }
    let n = max(i32(u.params.y), 1);
    var sum = vec3(0.0);
    for (var i = 0; i < n; i++) {
        let t = (f32(i) + 0.5) / f32(n) - 0.5;
        let q = clamp(vec2<i32>((uv + v * t) * size), vec2(0), last);
        sum += textureLoad(color, q, 0).rgb;
    }
    return vec4(sum / f32(n), here.a);
}
";

struct Gpu {
    format: wgpu::TextureFormat,
    size: [u32; 2],
    copy: wgpu::Texture,
    copy_view: wgpu::TextureView,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
}

struct Last {
    view_proj: Mat4,
    camera: DVec3,
}

struct Render {
    gpu: Option<Gpu>,
    last: Option<Last>,
}
static RENDER: Mutex<Render> = Mutex::new(Render {
    gpu: None,
    last: None,
});

const UNIFORM_FLOATS: usize = 16 + 16 + 4 + 4 + 4;

fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Motion Blur bindings"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Motion Blur layout"),
        bind_group_layouts: &[Some(&layout)],
        ..Default::default()
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Motion Blur"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Motion Blur"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(format.into())],
        }),
        multiview_mask: None,
        cache: None,
    });
    (layout, pipeline)
}

fn gpu(device: &wgpu::Device, color: &wgpu::Texture, format: wgpu::TextureFormat, size: [u32; 2]) -> Gpu {
    let (layout, pipeline) = pipeline(device, format);
    let view_formats: &[wgpu::TextureFormat] = if color.format() == format { &[] } else { &[format] };
    let copy = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Motion Blur copy"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: color.format(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats,
    });
    let copy_view = copy.create_view(&wgpu::TextureViewDescriptor {
        format: Some(format),
        ..Default::default()
    });
    let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Motion Blur uniforms"),
        size: (UNIFORM_FLOATS * 4) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    Gpu {
        format,
        size,
        copy,
        copy_view,
        layout,
        pipeline,
        uniforms,
    }
}

/// The world is drawn: blur it.
pub fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    color: &wgpu::Texture,
    color_view: &wgpu::TextureView,
    color_format: wgpu::TextureFormat,
    size: [u32; 2],
    depth: &wgpu::TextureView,
    depth_size: [u32; 2],
    view_proj: Mat4,
    camera: DVec3,
) {
    let mut render = RENDER.lock().unwrap_or_else(|e| e.into_inner());
    let last = render.last.replace(Last { view_proj, camera });
    if !ON.load(Ordering::Relaxed) {
        render.gpu = None;
        return;
    }
    let Some(last) = last else { return };
    let delta = camera - last.camera;
    if delta.length() > TELEPORT
        || size.contains(&0)
        || depth_size.contains(&0)
        || !color.usage().contains(wgpu::TextureUsages::COPY_SRC)
    {
        return;
    }
    if !render
        .gpu
        .as_ref()
        .is_some_and(|g| g.format == color_format && g.size == size && g.copy.format() == color.format())
    {
        render.gpu = Some(gpu(device, color, color_format, size));
    }
    let Some(g) = render.gpu.as_ref() else { return };

    let mut u = [0f32; UNIFORM_FLOATS];
    u[0..16].copy_from_slice(&view_proj.inverse().to_cols_array());
    u[16..32].copy_from_slice(&last.view_proj.to_cols_array());
    u[32..35].copy_from_slice(&delta.as_vec3().to_array());
    u[36] = f32::from_bits(SCALE.load(Ordering::Relaxed));
    u[37] = SAMPLE_COUNT.load(Ordering::Relaxed) as f32;
    u[38] = MAX_LENGTH;
    u[40..44].copy_from_slice(&[
        size[0] as f32,
        size[1] as f32,
        depth_size[0] as f32,
        depth_size[1] as f32,
    ]);
    let bytes: Vec<u8> = u.iter().flat_map(|f| f.to_le_bytes()).collect();
    queue.write_buffer(&g.uniforms, 0, &bytes);

    encoder.copy_texture_to_texture(color.as_image_copy(), g.copy.as_image_copy(), g.copy.size());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Motion Blur"),
        layout: &g.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: g.uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&g.copy_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(depth),
            },
        ],
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Motion Blur"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    pass.set_pipeline(&g.pipeline);
    pass.set_bind_group(0, &group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
    }

    #[test]
    fn reads_defaults_and_round_trips() {
        let d = Settings::read(stored(&[]));
        assert_eq!(d, Settings { strength: 75, samples: 12, steady: true });
        assert_eq!(Settings::read(stored(&[("strength", "33")])).strength, 75);
        let mut s = d;
        s.click(0);
        s.click(1);
        s.click(2);
        assert_eq!(s, Settings { strength: 100, samples: 16, steady: false });
        let round: Vec<(&str, String)> = s.stored().into();
        let back: Vec<(&str, &str)> = round.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Settings::read(stored(&back)), s);
        assert_eq!(s.labels()[0], "Strength: 100%");
    }

    #[test]
    fn steady_scales_to_sixty_fps() {
        let s = Settings { strength: 100, samples: 12, steady: true };
        assert!((s.scale(1.0 / 60.0) - 1.0).abs() < 1e-4);
        assert!((s.scale(1.0 / 240.0) - 4.0).abs() < 1e-3);
        assert!((s.scale(1.0) - 0.25).abs() < 1e-4);
        let off = Settings { steady: false, ..s };
        assert_eq!(off.scale(1.0 / 240.0), 1.0);
    }
}
