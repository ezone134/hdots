//! Vulkan renderer — the same scene graph as the GL backend, driven by
//! `ZEN_VULKAN=1`.
//!
//! Mirrors the GL passes exactly: a rounded-rect SDF solid pass and a
//! premultiplied glyph-texture pass, batched into two dynamic vertex buffers.
//! NDC is baked into the vertex data on the CPU (no uniforms), and the shaders
//! are authored in WGSL (y-up, naga flips for Vulkan's y-down clip space),
//! compiled to SPIR-V at startup with naga.

use ash::vk;
use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

use crate::render::Tex;

// ----------------------------------------------------------------------------
// WGSL shaders (same math as the GLSL ones in render.rs)
// ----------------------------------------------------------------------------

const SOLID_VERT: &str = r#"
struct VSIn {
    @location(0) a_pos: vec2<f32>,
    @location(1) a_color: vec4<f32>,
    @location(2) a_local: vec4<f32>,
    @location(3) a_half: vec2<f32>,
    @location(4) a_radii: vec4<f32>,
};
struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) v_color: vec4<f32>,
    @location(1) v_local: vec4<f32>,
    @location(2) v_half: vec2<f32>,
    @location(3) v_radii: vec4<f32>,
};
@vertex fn main(in: VSIn) -> VSOut {
    var out: VSOut;
    out.pos = vec4<f32>(in.a_pos, 0.0, 1.0);
    out.v_color = in.a_color;
    out.v_local = in.a_local;
    out.v_half = in.a_half;
    out.v_radii = in.a_radii;
    return out;
}
"#;

const SOLID_FRAG: &str = r#"
struct FSIn {
    @location(0) v_color: vec4<f32>,
    @location(1) v_local: vec4<f32>,
    @location(2) v_half: vec2<f32>,
    @location(3) v_radii: vec4<f32>,
};
fn sd_round_per_corner(p: vec2<f32>, b: vec2<f32>, radii: vec4<f32>) -> f32 {
    let ap = abs(p);
    var r: f32;
    if (p.x >= 0.0) {
        if (p.y >= 0.0) { r = radii.z; } else { r = radii.y; }
    } else {
        if (p.y >= 0.0) { r = radii.w; } else { r = radii.x; }
    }
    if (r >= 0.0) {
        let q = ap - b + r;
        return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
    } else {
        let ar = -r;
        let rect_d = max(ap.x - b.x, ap.y - b.y);
        let circle_d = ar - length(ap - b);
        return max(rect_d, circle_d);
    }
}
@fragment fn main(in: FSIn) -> @location(0) vec4<f32> {
    let d = sd_round_per_corner(in.v_local.xy - in.v_half, in.v_half, in.v_radii);
    // v_local.z > 0 → stroked outline: a band of that width around the edge.
    let a = select(
        1.0 - smoothstep(0.0, 1.0, d),
        1.0 - smoothstep(in.v_local.z - 1.0, in.v_local.z, abs(d)),
        in.v_local.z > 0.0,
    );
    return vec4<f32>(in.v_color.rgb * a, in.v_color.a * a);
}
"#;

const TEX_VERT: &str = r#"
struct VSIn {
    @location(0) a_pos: vec2<f32>,
    @location(1) a_uv: vec2<f32>,
    @location(2) a_color: vec4<f32>,
};
struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) v_uv: vec2<f32>,
    @location(1) v_color: vec4<f32>,
};
@vertex fn main(in: VSIn) -> VSOut {
    var out: VSOut;
    out.pos = vec4<f32>(in.a_pos, 0.0, 1.0);
    out.v_uv = in.a_uv;
    out.v_color = in.a_color;
    return out;
}
"#;

const TEX_FRAG: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(0) var samp: sampler;
struct FSIn {
    @location(0) v_uv: vec2<f32>,
    @location(1) v_color: vec4<f32>,
};
@fragment fn main(in: FSIn) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.v_uv) * in.v_color;
}
"#;

/// WGSL → SPIR-V via naga.
fn compile_spirv(src: &str) -> Result<Vec<u32>, String> {
    let module = naga::front::wgsl::parse_str(src).map_err(|e| e.to_string())?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| e.to_string())?;
    // ADJUST_COORDINATE_SPACE flips position.y (WGSL y-up → Vulkan y-down),
    // which is exactly what we want for the baked y-up NDC. Keep defaults.
    naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
        .map_err(|e| e.to_string())
}

// ----------------------------------------------------------------------------
// backing resources for a glyph texture
// ----------------------------------------------------------------------------

struct VkTex {
    image: vk::Image,
    mem: vk::DeviceMemory,
    view: vk::ImageView,
    set: vk::DescriptorSet,
}

pub struct Vk {
    entry: ash::Entry,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
    physical: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    wayland_loader: ash::khr::wayland_surface::Instance,
    surface_loader: ash::khr::surface::Instance,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    format: vk::Format,
    extent: vk::Extent2D,
    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    solid_pipeline: vk::Pipeline,
    solid_layout: vk::PipelineLayout,
    tex_pipeline: vk::Pipeline,
    tex_layout: vk::PipelineLayout,
    desc_pool: vk::DescriptorPool,
    desc_layout: vk::DescriptorSetLayout,
    sampler: vk::Sampler,
    cmd_pool: vk::CommandPool,
    cmd: vk::CommandBuffer,
    solid_buf: vk::Buffer,
    solid_mem: vk::DeviceMemory,
    solid_map: *mut u8,
    tex_buf: vk::Buffer,
    tex_mem: vk::DeviceMemory,
    tex_map: *mut u8,
    acquire_sem: vk::Semaphore,
    render_sem: vk::Semaphore,
    fence: vk::Fence,
    w: i32,
    h: i32,

    solid_verts: Vec<f32>,
    tex_quads: Vec<(u64, Vec<f32>)>,
    /// texture slots; `Tex.id` is the slot index. Evicted textures become
    /// `None` and their slot is recycled via `text_free`, so the cache stays
    /// bounded instead of leaking GPU image memory for every unique string.
    textures: Vec<Option<VkTex>>,
    /// freed texture slots to reuse on next upload
    text_free: Vec<u64>,

    /// glyph cache — same key as the GL backend
    pub text_cache: HashMap<(String, u32, u32, bool, u8, u8), Tex>,
    /// image cache — `ImageStore` keys
    pub img_cache: HashMap<String, Tex>,
    /// LRU recency lists (front = most recently used; back = eviction target)
    /// so text_cache / img_cache stay bounded instead of leaking forever.
    text_lru: VecDeque<(String, u32, u32, bool, u8, u8)>,
    img_lru: VecDeque<String>,
}

/// Upper bound on cached glyph textures — see GL backend (`render.rs`) for why.
const TEXT_CAP: usize = 512;
/// Upper bound on cached image textures.
const IMG_CAP: usize = 384;

const SOLID_STRIDE: usize = 16; // pos2 color4 local4 half2 radii4
const TEX_STRIDE: usize = 8; // pos2 uv2 color4
const VERT_BUF_BYTES: usize = 4 << 20; // 4 MiB per pass — plenty for a shell scene

impl Vk {
    /// Create the Vulkan instance + surface + swapchain. `display_ptr` is the
    /// raw `wl_display`, `surface_ptr` the raw `wl_surface` proxy.
    pub unsafe fn init(
        display_ptr: *mut c_void,
        surface_ptr: *mut c_void,
        w: i32,
        h: i32,
    ) -> Result<Self, String> {
        let tr = std::env::var("ZEN_TRACE").is_ok();
        macro_rules! tr {
            ($($a:tt)*) => { if tr { eprintln!("zen: VK: {}", format!($($a)*)); } };
        }

        tr!("load entry");
        let entry = ash::Entry::load().map_err(|e| format!("vulkan loader: {e}"))?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"zen-shell")
            .api_version(vk::make_api_version(0, 1, 0, 0));
        let ext_names = [
            vk::KHR_SURFACE_NAME.as_ptr(),
            vk::KHR_WAYLAND_SURFACE_NAME.as_ptr(),
        ];
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&ext_names);
        tr!("create instance");
        let instance = entry
            .create_instance(&create_info, None)
            .map_err(|e| format!("vkCreateInstance: {e}"))?;

        let wayland_loader = ash::khr::wayland_surface::Instance::new(&entry, &instance);
        let surface_info = vk::WaylandSurfaceCreateInfoKHR::default()
            .display(display_ptr as *mut vk::wl_display)
            .surface(surface_ptr as *mut vk::wl_surface);
        tr!("create wayland surface");
        let surface = wayland_loader
            .create_wayland_surface(&surface_info, None)
            .map_err(|e| format!("vkCreateWaylandSurfaceKHR: {e}"))?;
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        // --- physical device + queue family (graphics + present) ---
        tr!("pick device");
        let phys_devices = instance
            .enumerate_physical_devices()
            .map_err(|e| format!("enumerate devices: {e}"))?;
        let mut chosen: Option<(vk::PhysicalDevice, u32)> = None;
        for phys in phys_devices {
            let props = instance.get_physical_device_queue_family_properties(phys);
            for (i, qf) in props.iter().enumerate() {
                let present = surface_loader
                    .get_physical_device_surface_support(phys, i as u32, surface)
                    .unwrap_or(false);
                if qf.queue_flags.contains(vk::QueueFlags::GRAPHICS) && present {
                    chosen = Some((phys, i as u32));
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        let (physical, queue_family) = chosen.ok_or("no usable physical device + queue family")?;

        let queue_priorities = [1.0f32];
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&queue_priorities);
        let dev_ext = [vk::KHR_SWAPCHAIN_NAME.as_ptr()];
        let queue_infos = [queue_info];
        let dev_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&dev_ext);
        tr!("create device");
        let device = instance
            .create_device(physical, &dev_info, None)
            .map_err(|e| format!("vkCreateDevice: {e}"))?;
        let queue = device.get_device_queue(queue_family, 0);
        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);

        // --- swapchain ---
        let caps = surface_loader
            .get_physical_device_surface_capabilities(physical, surface)
            .map_err(|e| format!("surface capabilities: {e}"))?;
        let formats = surface_loader
            .get_physical_device_surface_formats(physical, surface)
            .map_err(|e| format!("surface formats: {e}"))?;
        let format = formats
            .iter()
            .find(|f| f.format == vk::Format::B8G8R8A8_UNORM)
            .or_else(|| formats.first())
            .ok_or("no surface formats")?
            .format;

        let present_modes = surface_loader
            .get_physical_device_surface_present_modes(physical, surface)
            .map_err(|e| format!("present modes: {e}"))?;
        let present_mode = if present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else {
            vk::PresentModeKHR::FIFO
        };

        let extent = vk::Extent2D {
            width: (w.max(1) as u32).clamp(caps.min_image_extent.width, caps.max_image_extent.width.max(1)),
            height: (h.max(1) as u32).clamp(caps.min_image_extent.height, caps.max_image_extent.height.max(1)),
        };
        let min_image_count = caps.min_image_count.clamp(2, 4);

        let composite_alpha = if caps.supported_composite_alpha.contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
        } else if caps.supported_composite_alpha.contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
        } else {
            vk::CompositeAlphaFlagsKHR::OPAQUE
        };

        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(min_image_count)
            .image_format(format)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(composite_alpha)
            .present_mode(present_mode)
            .clipped(true);
        tr!("create swapchain");
        let swapchain = swapchain_loader
            .create_swapchain(&swapchain_info, None)
            .map_err(|e| format!("vkCreateSwapchainKHR: {e}"))?;
        let images = swapchain_loader
            .get_swapchain_images(swapchain)
            .map_err(|e| format!("get swapchain images: {e}"))?;

        // --- render pass (premultiplied, clear to transparent) ---
        let color_attach = vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
        let color_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let color_refs = [color_ref];
        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_refs);
        let dep = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);
        let attachments = [color_attach];
        let subpasses = [subpass];
        let dependencies = [dep];
        let rp_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        let render_pass = device
            .create_render_pass(&rp_info, None)
            .map_err(|e| format!("create render pass: {e}"))?;

        // --- image views + framebuffers ---
        let mut views = Vec::with_capacity(images.len());
        for &img in &images {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1));
            views.push(device.create_image_view(&view_info, None).map_err(|e| format!("image view: {e}"))?);
        }
        let mut framebuffers = Vec::with_capacity(views.len());
        for &view in &views {
            let attachments = [view];
            let fb_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            framebuffers.push(device.create_framebuffer(&fb_info, None).map_err(|e| format!("framebuffer: {e}"))?);
        }

        // --- pipelines ---
        let solid_mod = create_shader_module(&device, SOLID_VERT)?;
        let solid_frag_mod = create_shader_module(&device, SOLID_FRAG)?;
        let tex_mod = create_shader_module(&device, TEX_VERT)?;
        let tex_frag_mod = create_shader_module(&device, TEX_FRAG)?;

        let solid_layout = device
            .create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)
            .map_err(|e| format!("solid layout: {e}"))?;

        let tex_desc_layout = {
            let binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(&[binding]),
                    None,
                )
                .map_err(|e| format!("tex desc layout: {e}"))?
        };
        let tex_layout = device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default().set_layouts(&[tex_desc_layout]),
                None,
            )
            .map_err(|e| format!("tex layout: {e}"))?;

        // solid pass: pos2 color4 local4 half2 radii4
        let solid_bindings = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride((SOLID_STRIDE * 4) as u32)
            .input_rate(vk::VertexInputRate::VERTEX)];
        let solid_attrs = [
            vk::VertexInputAttributeDescription::default().location(0).binding(0).format(vk::Format::R32G32_SFLOAT).offset(0),
            vk::VertexInputAttributeDescription::default().location(1).binding(0).format(vk::Format::R32G32B32A32_SFLOAT).offset(8),
            vk::VertexInputAttributeDescription::default().location(2).binding(0).format(vk::Format::R32G32B32A32_SFLOAT).offset(24),
            vk::VertexInputAttributeDescription::default().location(3).binding(0).format(vk::Format::R32G32_SFLOAT).offset(40),
            vk::VertexInputAttributeDescription::default().location(4).binding(0).format(vk::Format::R32G32B32A32_SFLOAT).offset(48),
        ];
        let solid_vi = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&solid_bindings)
            .vertex_attribute_descriptions(&solid_attrs);
        // text pass: pos2 uv2 color4
        let tex_bindings = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride((TEX_STRIDE * 4) as u32)
            .input_rate(vk::VertexInputRate::VERTEX)];
        let tex_attrs = [
            vk::VertexInputAttributeDescription::default().location(0).binding(0).format(vk::Format::R32G32_SFLOAT).offset(0),
            vk::VertexInputAttributeDescription::default().location(1).binding(0).format(vk::Format::R32G32_SFLOAT).offset(8),
            vk::VertexInputAttributeDescription::default().location(2).binding(0).format(vk::Format::R32G32B32A32_SFLOAT).offset(16),
        ];
        let tex_vi = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&tex_bindings)
            .vertex_attribute_descriptions(&tex_attrs);

        fn graphics(
            device: &ash::Device,
            layout: vk::PipelineLayout,
            render_pass: vk::RenderPass,
            vi: vk::PipelineVertexInputStateCreateInfo<'_>,
            vs_mod: vk::ShaderModule,
            fs_mod: vk::ShaderModule,
        ) -> Result<vk::Pipeline, String> {
            let stages = [
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vs_mod)
                    .name(c"main"),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fs_mod)
                    .name(c"main"),
            ];
            let ia = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let vs = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
            let rs = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .line_width(1.0);
            let ms = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let blend = vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::ONE)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A);
            let blends = [blend];
            let cb = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blends);
            let dynamic = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dyn_state = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic);
            let info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .vertex_input_state(&vi)
                .input_assembly_state(&ia)
                .viewport_state(&vs)
                .rasterization_state(&rs)
                .multisample_state(&ms)
                .color_blend_state(&cb)
                .dynamic_state(&dyn_state)
                .layout(layout)
                .render_pass(render_pass)
                .subpass(0);
            let infos = [info];
            let p = unsafe {
                device
                    .create_graphics_pipelines(vk::PipelineCache::null(), &infos, None)
                    .map_err(|e| format!("create_graphics_pipelines: {e:?}"))?
            };
            Ok(p[0])
        }

        let solid_pipeline = graphics(&device, solid_layout, render_pass, solid_vi, solid_mod, solid_frag_mod)?;
        let tex_pipeline = graphics(&device, tex_layout, render_pass, tex_vi, tex_mod, tex_frag_mod)?;

        device.destroy_shader_module(solid_mod, None);
        device.destroy_shader_module(solid_frag_mod, None);
        device.destroy_shader_module(tex_mod, None);
        device.destroy_shader_module(tex_frag_mod, None);

        // --- descriptor pool + sampler ---
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(256)];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(256)
            .pool_sizes(&pool_sizes);
        let desc_pool = device
            .create_descriptor_pool(&pool_info, None)
            .map_err(|e| format!("desc pool: {e}"))?;
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::NEAREST)
            .min_filter(vk::Filter::NEAREST)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let sampler = device
            .create_sampler(&sampler_info, None)
            .map_err(|e| format!("sampler: {e}"))?;

        // --- command pool + buffer ---
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family);
        let cmd_pool = device
            .create_command_pool(&pool_info, None)
            .map_err(|e| format!("cmd pool: {e}"))?;
        let cmd_alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmds = device
            .allocate_command_buffers(&cmd_alloc)
            .map_err(|e| format!("allocate cmd: {e}"))?;

        // --- dynamic vertex buffers (host-visible, mapped once) ---
        let (solid_buf, solid_mem, solid_map) =
            create_vertex_buffer(&instance, &device, physical, VERT_BUF_BYTES)?;
        let (tex_buf, tex_mem, tex_map) =
            create_vertex_buffer(&instance, &device, physical, VERT_BUF_BYTES)?;

        // --- sync ---
        let acquire_sem = device
            .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            .map_err(|e| format!("acquire sem: {e}"))?;
        let render_sem = device
            .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            .map_err(|e| format!("render sem: {e}"))?;
        let fence = device
            .create_fence(&vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED), None)
            .map_err(|e| format!("fence: {e}"))?;

        Ok(Vk {
            entry,
            instance,
            surface,
            physical,
            device,
            queue,
            wayland_loader,
            surface_loader,
            swapchain_loader,
            swapchain,
            format,
            extent,
            images,
            views,
            framebuffers,
            render_pass,
            solid_pipeline,
            solid_layout,
            tex_pipeline,
            tex_layout,
            desc_pool,
            desc_layout: tex_desc_layout,
            sampler,
            cmd_pool,
            cmd: cmds[0],
            solid_buf,
            solid_mem,
            solid_map,
            tex_buf,
            tex_mem,
            tex_map,
            acquire_sem,
            render_sem,
            fence,
            w,
            h,
            solid_verts: Vec::new(),
            tex_quads: Vec::new(),
            textures: Vec::new(),
            text_free: Vec::new(),
            text_cache: HashMap::new(),
            img_cache: HashMap::new(),
            text_lru: VecDeque::new(),
            img_lru: VecDeque::new(),
        })
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.w = w;
        self.h = h;
        if w <= 0 || h <= 0 {
            return;
        }
        unsafe { self.recreate_swapchain(w.max(1), h.max(1)) };
    }

    unsafe fn recreate_swapchain(&mut self, w: i32, h: i32) {
        self.device.device_wait_idle().ok();
        for &fb in &self.framebuffers {
            self.device.destroy_framebuffer(fb, None);
        }
        for &v in &self.views {
            self.device.destroy_image_view(v, None);
        }
        self.framebuffers.clear();
        self.views.clear();
        self.swapchain_loader.destroy_swapchain(self.swapchain, None);

        let caps = self
            .surface_loader
            .get_physical_device_surface_capabilities(self.physical, self.surface)
            .unwrap_or_default();
        let extent = vk::Extent2D {
            width: (w as u32).clamp(caps.min_image_extent.width, caps.max_image_extent.width.max(1)),
            height: (h as u32).clamp(caps.min_image_extent.height, caps.max_image_extent.height.max(1)),
        };
        let min_image_count = caps.min_image_count.clamp(2, 4);
        let composite_alpha = if caps.supported_composite_alpha.contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
        } else if caps.supported_composite_alpha.contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
        } else {
            vk::CompositeAlphaFlagsKHR::OPAQUE
        };
        let info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(min_image_count)
            .image_format(self.format)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(composite_alpha)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true);
        let Ok(swapchain) = self.swapchain_loader.create_swapchain(&info, None) else { return };
        self.swapchain = swapchain;
        let Ok(images) = self.swapchain_loader.get_swapchain_images(swapchain) else { return };
        self.images = images;
        for &img in &self.images {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(self.format)
                .subresource_range(vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1));
            if let Ok(v) = self.device.create_image_view(&view_info, None) {
                self.views.push(v);
            }
        }
        for &view in &self.views {
            let attachments = [view];
            let fb_info = vk::FramebufferCreateInfo::default()
                .render_pass(self.render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            if let Ok(fb) = self.device.create_framebuffer(&fb_info, None) {
                self.framebuffers.push(fb);
            }
        }
        self.extent = extent;
    }

    pub fn begin_frame(&mut self) {
        self.solid_verts.clear();
        self.tex_quads.clear();
    }

    fn premul(color: u32) -> [f32; 4] {
        let a = (color & 0xff) as f32 / 255.0;
        let r = ((color >> 24) & 0xff) as f32 / 255.0;
        let g = ((color >> 16) & 0xff) as f32 / 255.0;
        let b = ((color >> 8) & 0xff) as f32 / 255.0;
        [r * a, g * a, b * a, a]
    }

    /// pixel (px, py), y-down surface → y-up NDC for WGSL (naga flips to Vulkan)
    fn ndc(&self, px: f32, py: f32) -> [f32; 2] {
        let w = self.w.max(1) as f32;
        let h = self.h.max(1) as f32;
        [(px / w) * 2.0 - 1.0, 1.0 - (py / h) * 2.0]
    }

    /// Rounded-rect quad — same geometry as the GL backend. Degenerate
    /// (negative) rects clamp to a zero radius instead of panicking.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
        self.rect_ex(x, y, w, h, r, 0.0, color);
    }

    /// Rounded-rect quad with an optional stroke (`stroke > 0` → outline-only
    /// band of that thickness; the stroke is centered on the edge).
    ///
    /// The quad is expanded by `AA_PAD` on every side so the fragment
    /// shader's anti-aliasing band never clips at the quad edge — a full
    /// circle (r = w/2) otherwise loses its outer AA ring at the cardinal
    /// points and renders as a lumpy dot. The SDF stays in the ORIGINAL
    /// shape coordinates (v_half/radii unchanged), so shading is identical.
    #[allow(clippy::too_many_arguments)]
    pub fn rect_ex(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, stroke: f32, color: u32) {
        const AA_PAD: f32 = 2.0;
        let max_r = h.min(w).max(0.0) / 2.0;
        let r = r.clamp(0.0, max_r);
        self.quad(
            [x - AA_PAD, y - AA_PAD],
            [(x + w) + AA_PAD, y - AA_PAD],
            [(x + w) + AA_PAD, (y + h) + AA_PAD],
            [x - AA_PAD, (y + h) + AA_PAD],
            [-AA_PAD, -AA_PAD],
            [w + AA_PAD, -AA_PAD],
            [w + AA_PAD, h + AA_PAD],
            [-AA_PAD, h + AA_PAD],
            stroke,
            w / 2.0,
            h / 2.0,
            [r, r, r, r],
            color,
        );
    }

    /// Rounded rect with per-corner radii — negative = concave corner.
    #[allow(clippy::too_many_arguments)]
    pub fn rect_concave(&mut self, x: f32, y: f32, w: f32, h: f32, r_tl: f32, r_tr: f32, r_br: f32, r_bl: f32, color: u32) {
        let max_r = h.min(w).max(0.0) / 2.0;
        self.quad(
            [x, y],
            [(x + w), y],
            [(x + w), (y + h)],
            [x, (y + h)],
            [0.0, 0.0],
            [w, 0.0],
            [w, h],
            [0.0, h],
            0.0,
            w / 2.0,
            h / 2.0,
            [
                r_tl.clamp(-max_r, max_r),
                r_tr.clamp(-max_r, max_r),
                r_br.clamp(-max_r, max_r),
                r_bl.clamp(-max_r, max_r),
            ],
            color,
        );
    }

    /// Emit one quad from 4 corner positions + matching local-space coords.
    /// Because vertex attributes interpolate linearly, the corners may be
    /// placed anywhere — rotating them while keeping axis-aligned local
    /// coords renders an arbitrary rotated rounded rect with the unchanged
    /// SDF shader. `tri` order matches `rect_ex` (two shared-edge tris).
    #[allow(clippy::too_many_arguments)]
    fn quad(
        &mut self,
        a: [f32; 2],
        b: [f32; 2],
        c: [f32; 2],
        d: [f32; 2],
        la: [f32; 2],
        lb: [f32; 2],
        lc: [f32; 2],
        ld: [f32; 2],
        stroke: f32,
        hx: f32,
        hy: f32,
        radii: [f32; 4],
        color: u32,
    ) {
        let col = Self::premul(color);
        for (p, l) in [(a, la), (b, lb), (c, lc), (a, la), (c, lc), (d, ld)] {
            let n = self.ndc(p[0], p[1]);
            self.solid_verts
                .extend_from_slice(&[n[0], n[1], col[0], col[1], col[2], col[3], l[0], l[1], stroke, 0.0, hx, hy, radii[0], radii[1], radii[2], radii[3]]);
        }
    }

    /// Thick line segment between two points — a capsule oriented along the
    /// segment. Used for heartbeat-style graph lines.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, color: u32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.01 {
            // degenerate → round dot
            self.rect(x0 - t / 2.0, y0 - t / 2.0, t, t, t / 2.0, color);
            return;
        }
        let ux = dx / len;
        let uy = dy / len;
        let px = -uy * t / 2.0; // perpendicular
        let py = ux * t / 2.0;
        // extend both ends by half thickness so joints between segments
        // overlap into seamless capsules
        let e = t / 2.0;
        let ax = x0 - ux * e;
        let ay = y0 - uy * e;
        let bx = x1 + ux * e;
        let by = y1 + uy * e;
        let l = len + t;
        let rad = t / 2.0;
        self.quad(
            [ax + px, ay + py],
            [bx + px, by + py],
            [bx - px, by - py],
            [ax - px, ay - py],
            [0.0, 0.0],
            [l, 0.0],
            [l, t],
            [0.0, t],
            0.0,
            l / 2.0,
            t / 2.0,
            [rad, rad, rad, rad],
            color,
        );
    }

    /// Textured quad — batched per texture, same as GL.
    pub fn text_quad(&mut self, tex: &Tex, x: f32, y: f32, w: f32, h: f32) {
        self.text_quad_tinted(tex, x, y, w, h, 0xffffffff);
    }

    /// Textured quad with a tint (0xRRGGBBAA); `0xffffffff` ≡ `text_quad`.
    /// Only ever used on the GL wallpaper path — kept for enum parity.
    pub fn text_quad_tinted(&mut self, tex: &Tex, x: f32, y: f32, w: f32, h: f32, tint: u32) {
        self.text_quad_uv_tinted(tex, x, y, w, h, 0.0, 0.0, 1.0, 1.0, tint);
    }

    /// UV-windowed textured quad — world-map band texture (pan/zoom moves the
    /// window, the raster is never re-run per frame).
    #[allow(clippy::too_many_arguments)]
    pub fn text_quad_uv(&mut self, tex: &Tex, x: f32, y: f32, w: f32, h: f32, u0: f32, v0: f32, u1: f32, v1: f32) {
        self.text_quad_uv_tinted(tex, x, y, w, h, u0, v0, u1, v1, 0xffffffff);
    }

    #[allow(clippy::too_many_arguments)]
    fn text_quad_uv_tinted(&mut self, tex: &Tex, x: f32, y: f32, w: f32, h: f32, u0: f32, v0: f32, u1: f32, v1: f32, tint: u32) {
        let x2 = x + w;
        let y2 = y + h;
        let c = Self::premul(tint);
        let mut verts = Vec::with_capacity(TEX_STRIDE * 6);
        for [px, py, u, vt] in [
            [x, y, u0, v0],
            [x2, y, u1, v0],
            [x2, y2, u1, v1],
            [x, y, u0, v0],
            [x2, y2, u1, v1],
            [x, y2, u0, v1],
        ] {
            let n = self.ndc(px, py);
            verts.extend_from_slice(&[n[0], n[1], u, vt, c[0], c[1], c[2], c[3]]);
        }
        match self.tex_quads.iter_mut().find(|(id, _)| *id == tex.id) {
            Some((_, v)) => v.extend_from_slice(&verts),
            None => self.tex_quads.push((tex.id, verts)),
        }
    }

    /// World-band texture upload — LINEAR filtering so stretched pan-zoom
    /// stays soft instead of blocky.
    pub fn upload_texture_linear(&mut self, w: u32, h: u32, pixels: &[u8]) -> Result<Tex, String> {
        self.upload_texture(w, h, pixels)
    }

    /// Upload premultiplied RGBA8 pixels as a sampled texture.
    pub fn upload_texture(&mut self, w: u32, h: u32, pixels: &[u8]) -> Result<Tex, String> {
        unsafe {
            let vk_tex = self.create_texture(w, h, pixels)?;
            let id = match self.text_free.pop() {
                Some(slot) => {
                    self.textures[slot as usize] = Some(vk_tex);
                    slot
                }
                None => {
                    self.textures.push(Some(vk_tex));
                    (self.textures.len() - 1) as u64
                }
            };
            Ok(Tex { id, w, h })
        }
    }

    unsafe fn create_texture(&mut self, w: u32, h: u32, pixels: &[u8]) -> Result<VkTex, String> {
        // staging buffer
        let size = (w * h * 4) as usize;
        let (staging, staging_mem, ptr) = create_buffer_with_memory(
            &self.instance,
            &self.device,
            self.physical,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), ptr, size);

        // device-local image
        let img_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D { width: w, height: h, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .format(vk::Format::R8G8B8A8_UNORM)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);
        let image = self
            .device
            .create_image(&img_info, None)
            .map_err(|e| format!("create image: {e}"))?;
        let mem_req = self.device.get_image_memory_requirements(image);
        let mem_type = find_memory_type(
            &self.instance,
            self.physical,
            mem_req.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or("no device-local memory type")?;
        let mem = self
            .device
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(mem_req.size)
                    .memory_type_index(mem_type),
                None,
            )
            .map_err(|e| format!("alloc image mem: {e}"))?;
        self.device.bind_image_memory(image, mem, 0).map_err(|e| format!("bind image: {e}"))?;

        // one-shot copy
        let alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmds = self.device.allocate_command_buffers(&alloc).map_err(|e| format!("alloc cb: {e}"))?;
        let cb = cmds[0];
        self.device
            .begin_command_buffer(
                cb,
                &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .map_err(|e| format!("begin cb: {e}"))?;
        // UNDEFINED → TRANSFER_DST
        let barrier = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1));
        let barriers = [barrier];
        let dep_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
        self.device.cmd_pipeline_barrier2(cb, &dep_info);
        let region = vk::BufferImageCopy::default()
            .image_subresource(vk::ImageSubresourceLayers::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .layer_count(1))
            .image_extent(vk::Extent3D { width: w, height: h, depth: 1 });
        self.device.cmd_copy_buffer_to_image(cb, staging, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[region]);
        // TRANSFER_DST → SHADER_READ_ONLY
        let barrier2 = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1));
        let barriers2 = [barrier2];
        let dep_info2 = vk::DependencyInfo::default().image_memory_barriers(&barriers2);
        self.device.cmd_pipeline_barrier2(cb, &dep_info2);
        self.device.end_command_buffer(cb).map_err(|e| format!("end cb: {e}"))?;
        let fence = self.device.create_fence(&vk::FenceCreateInfo::default(), None).map_err(|e| format!("fence: {e}"))?;
        self.device
            .queue_submit(self.queue, &[vk::SubmitInfo::default().command_buffers(&[cb])], fence)
            .map_err(|e| format!("submit: {e}"))?;
        self.device.wait_for_fences(&[fence], true, u64::MAX).ok();
        self.device.destroy_fence(fence, None);
        self.device.free_command_buffers(self.cmd_pool, &[cb]);
        self.device.destroy_buffer(staging, None);
        self.device.free_memory(staging_mem, None);

        // image view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1));
        let view = self.device.create_image_view(&view_info, None).map_err(|e| format!("view: {e}"))?;

        // descriptor set
        let layouts = [self.desc_layout];
        let alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.desc_pool)
            .set_layouts(&layouts);
        let sets = self.device.allocate_descriptor_sets(&alloc).map_err(|e| format!("alloc set: {e}"))?;
        let set = sets[0];
        let img_info = vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        let img_infos = [img_info];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&img_infos);
        self.device.update_descriptor_sets(&[write], &[]);

        Ok(VkTex { image, mem, view, set })
    }

    pub fn text_contains(&self, key: &(String, u32, u32, bool, u8, u8)) -> bool {
        self.text_cache.contains_key(key)
    }
    pub fn text_get(&mut self, key: &(String, u32, u32, bool, u8, u8)) -> Option<Tex> {
        if !self.text_cache.contains_key(key) {
            return None;
        }
        self.touch_text(key.clone());
        self.text_cache.get(key).copied()
    }
    pub fn text_insert(&mut self, key: (String, u32, u32, bool, u8, u8), t: Tex) {
        self.text_cache.insert(key.clone(), t);
        self.touch_text(key);
        while self.text_cache.len() > TEXT_CAP {
            if !self.evict_text() {
                break;
            }
        }
    }

    fn touch_text(&mut self, key: (String, u32, u32, bool, u8, u8)) {
        if let Some(pos) = self.text_lru.iter().position(|k| k == &key) {
            self.text_lru.remove(pos);
        }
        self.text_lru.push_front(key);
    }

    /// Free the least-recently-used glyph texture (GPU resources + cache key).
    fn evict_text(&mut self) -> bool {
        let Some(key) = self.text_lru.pop_back() else { return false };
        if let Some(t) = self.text_cache.remove(&key) {
            self.free_tex(t);
        }
        true
    }

    pub fn img_contains(&self, key: &str) -> bool {
        self.img_cache.contains_key(key)
    }
    pub fn img_insert(&mut self, key: String, t: Tex) {
        self.img_cache.insert(key.clone(), t);
        self.touch_img(&key);
        while self.img_cache.len() > IMG_CAP {
            if !self.evict_img() {
                break;
            }
        }
    }
    pub fn img_get(&mut self, key: &str) -> Option<Tex> {
        if !self.img_cache.contains_key(key) {
            return None;
        }
        self.touch_img(key);
        self.img_cache.get(key).copied()
    }

    /// Drop a texture immediately (live camera frames) — the next draw of
    /// that key re-uploads from the `ImageStore` pixels.
    pub fn img_remove(&mut self, key: &str) {
        if let Some(t) = self.img_cache.remove(key) {
            self.img_lru.retain(|k| k != key);
            self.free_tex(t);
        }
    }

    fn touch_img(&mut self, key: &str) {
        if let Some(pos) = self.img_lru.iter().position(|k| k.as_str() == key) {
            self.img_lru.remove(pos);
        }
        self.img_lru.push_front(key.to_string());
    }

    fn evict_img(&mut self) -> bool {
        let Some(key) = self.img_lru.pop_back() else { return false };
        if let Some(t) = self.img_cache.remove(&key) {
            self.free_tex(t);
        }
        true
    }

    fn free_tex(&mut self, t: Tex) {
        if t.id >= self.textures.len() as u64 {
            return;
        }
        if let Some(vk_tex) = self.textures[t.id as usize].take() {
            unsafe {
                self.device.destroy_image_view(vk_tex.view, None);
                self.device.destroy_image(vk_tex.image, None);
                self.device.free_memory(vk_tex.mem, None);
            }
        }
        self.text_free.push(t.id);
    }

    /// Record + submit + present one frame.
    pub fn flush(&mut self) {
        unsafe {
            // wait for the previous frame so the cmd buffer + VBOs are free
            self.device
                .wait_for_fences(&[self.fence], true, u64::MAX)
                .ok();
            self.device.reset_fences(&[self.fence]).ok();

            let (idx, _) = match self
                .swapchain_loader
                .acquire_next_image(self.swapchain, u64::MAX, self.acquire_sem, vk::Fence::null())
            {
                Ok(v) => v,
                Err(_) => return, // out of date — next configure will resize
            };

            // upload vertex data
            let solid_bytes = self.solid_verts.len() * 4;
            if solid_bytes <= VERT_BUF_BYTES {
                std::ptr::copy_nonoverlapping(self.solid_verts.as_ptr() as *const u8, self.solid_map, solid_bytes);
            }
            let mut tex_offset = 0usize;
            for (_, verts) in &self.tex_quads {
                let bytes = verts.len() * 4;
                if tex_offset + bytes > VERT_BUF_BYTES {
                    break;
                }
                std::ptr::copy_nonoverlapping(verts.as_ptr() as *const u8, self.tex_map.add(tex_offset), bytes);
                tex_offset += bytes;
            }

            let cmd = self.cmd;
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .ok();
            self.device
                .begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())
                .ok();

            let clear = [vk::ClearValue {
                color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0] },
            }];
            let rp_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[idx as usize])
                .render_area(vk::Rect2D::default().extent(self.extent))
                .clear_values(&clear);
            self.device.cmd_begin_render_pass(cmd, &rp_info, vk::SubpassContents::INLINE);

            let viewport = vk::Viewport::default()
                .width(self.extent.width as f32)
                .height(self.extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0);
            let scissor = vk::Rect2D::default().extent(self.extent);
            self.device.cmd_set_viewport(cmd, 0, &[viewport]);
            self.device.cmd_set_scissor(cmd, 0, &[scissor]);

            // solid pass
            if !self.solid_verts.is_empty() {
                self.device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.solid_pipeline);
                self.device.cmd_bind_vertex_buffers(cmd, 0, &[self.solid_buf], &[0]);
                self.device
                    .cmd_draw(cmd, (self.solid_verts.len() / SOLID_STRIDE) as u32, 1, 0, 0);
            }
            // text pass, batched per texture
            let mut offset = 0u64;
            for (tex_id, verts) in &self.tex_quads {
                let bytes = (verts.len() * 4) as u64;
                if offset + bytes > VERT_BUF_BYTES as u64 {
                    break;
                }
                let Some(vk_tex) = self.textures.get(*tex_id as usize).and_then(|t| t.as_ref()) else { offset += bytes; continue };
                self.device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.tex_pipeline);
                self.device.cmd_bind_vertex_buffers(cmd, 0, &[self.tex_buf], &[offset]);
                self.device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.tex_layout,
                    0,
                    &[vk_tex.set],
                    &[],
                );
                self.device.cmd_draw(cmd, (verts.len() / TEX_STRIDE) as u32, 1, 0, 0);
                offset += bytes;
            }

            self.device.cmd_end_render_pass(cmd);
            self.device.end_command_buffer(cmd).ok();

            let wait_sems = [self.acquire_sem];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let cmd_bufs = [cmd];
            let signal_sems = [self.render_sem];
            let submit = vk::SubmitInfo::default()
                .wait_semaphores(&wait_sems)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&cmd_bufs)
                .signal_semaphores(&signal_sems);
            let submits = [submit];
            self.device.queue_submit(self.queue, &submits, self.fence).ok();

            let present_sems = [self.render_sem];
            let swapchains = [self.swapchain];
            let indices = [idx];
            let present = vk::PresentInfoKHR::default()
                .wait_semaphores(&present_sems)
                .swapchains(&swapchains)
                .image_indices(&indices);
            if let Err(vk::Result::ERROR_OUT_OF_DATE_KHR) = self.swapchain_loader.queue_present(self.queue, &present) {
                // surface size changed — next configure acks the resize
            }
        }
    }
}

unsafe fn create_shader_module(device: &ash::Device, wgsl: &str) -> Result<vk::ShaderModule, String> {
    let words = compile_spirv(wgsl)?;
    device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&words),
            None,
        )
        .map_err(|e| format!("shader module: {e}"))
}

unsafe fn create_vertex_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical: vk::PhysicalDevice,
    size: usize,
) -> Result<(vk::Buffer, vk::DeviceMemory, *mut u8), String> {
    create_buffer_with_memory(
        instance,
        device,
        physical,
        size,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )
}

unsafe fn create_buffer_with_memory(
    instance: &ash::Instance,
    device: &ash::Device,
    physical: vk::PhysicalDevice,
    size: usize,
    usage: vk::BufferUsageFlags,
    props: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory, *mut u8), String> {
    let info = vk::BufferCreateInfo::default()
        .size(size as u64)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buf = device.create_buffer(&info, None).map_err(|e| format!("create buffer: {e}"))?;
    let req = device.get_buffer_memory_requirements(buf);
    let mem_type = find_memory_type(instance, physical, req.memory_type_bits, props)
        .ok_or("no suitable memory type")?;
    let mem = device
        .allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(req.size)
                .memory_type_index(mem_type),
            None,
        )
        .map_err(|e| format!("alloc buffer mem: {e}"))?;
    device.bind_buffer_memory(buf, mem, 0).map_err(|e| format!("bind buffer: {e}"))?;
    let ptr = device
        .map_memory(mem, 0, req.size, vk::MemoryMapFlags::empty())
        .map_err(|e| format!("map buffer: {e}"))? as *mut u8;
    Ok((buf, mem, ptr))
}

fn find_memory_type(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    type_bits: u32,
    props: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical) };
    mem_props.memory_types.iter().enumerate().find_map(|(i, t)| {
        if type_bits & (1 << i) != 0 && t.property_flags.contains(props) {
            Some(i as u32)
        } else {
            None
        }
    })
}

impl Drop for Vk {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            for tex in &self.textures {
                let Some(tex) = tex else { continue };
                self.device.destroy_image_view(tex.view, None);
                self.device.destroy_image(tex.image, None);
                self.device.free_memory(tex.mem, None);
            }
            self.device.destroy_fence(self.fence, None);
            self.device.destroy_semaphore(self.acquire_sem, None);
            self.device.destroy_semaphore(self.render_sem, None);
            self.device.unmap_memory(self.solid_mem);
            self.device.unmap_memory(self.tex_mem);
            self.device.destroy_buffer(self.solid_buf, None);
            self.device.destroy_buffer(self.tex_buf, None);
            self.device.free_memory(self.solid_mem, None);
            self.device.free_memory(self.tex_mem, None);
            self.device.destroy_descriptor_pool(self.desc_pool, None);
            self.device.destroy_descriptor_set_layout(self.desc_layout, None);
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_pipeline(self.solid_pipeline, None);
            self.device.destroy_pipeline(self.tex_pipeline, None);
            self.device.destroy_pipeline_layout(self.solid_layout, None);
            self.device.destroy_pipeline_layout(self.tex_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device.destroy_command_pool(self.cmd_pool, None);
            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }
            for &v in &self.views {
                self.device.destroy_image_view(v, None);
            }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
            let _ = &self.wayland_loader;
            let _ = &self.entry;
        }
    }
}

// silence unused warnings for helpers that take `c_char` (kept for parity with GL)
#[allow(dead_code)]
fn _c_char(_: *const c_char) {}
#[allow(dead_code)]
fn _cstr(_: &CStr) {}
