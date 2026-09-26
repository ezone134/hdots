use std::ffi::c_void;
use std::ffi::CStr;

use ash::vk;
use ash::{Device, Entry, Instance};

pub const UBO_SIZE: u64 = 16;

#[derive(Clone, Copy)]
pub struct View {
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}

pub enum RenderStatus {
    Rendered,
    Busy,
    OutOfDate,
}

pub struct Vulkan {
    instance: Instance,
    phys: vk::PhysicalDevice,
    device: Device,
    queue_family: u32,
    queue: vk::Queue,

    surface: vk::SurfaceKHR,
    surface_ext: ash::khr::surface::Instance,
    swap_ext: ash::khr::swapchain::Device,

    format: vk::Format,
    present_mode: vk::PresentModeKHR,
    extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,

    render_pass: vk::RenderPass,
    dsl: vk::DescriptorSetLayout,
    desc_pool: vk::DescriptorPool,
    desc_set: vk::DescriptorSet,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,

    vbo: vk::Buffer,
    vbo_mem: vk::DeviceMemory,
    ubo: vk::Buffer,
    ubo_mem: vk::DeviceMemory,
    ubo_ptr: *mut u8,

    texture: vk::Image,
    texture_mem: vk::DeviceMemory,
    texture_view: vk::ImageView,
    sampler: vk::Sampler,

    cmd_pool: vk::CommandPool,
    cmd: vk::CommandBuffer,
    acquire_fence: vk::Fence,

    bg_clear: [f32; 4],

    _entry: Entry,
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            self.device.destroy_fence(self.acquire_fence, None);
            self.device.destroy_command_pool(self.cmd_pool, None);
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_image_view(self.texture_view, None);
            self.device.destroy_image(self.texture, None);
            self.device.free_memory(self.texture_mem, None);
            self.device.unmap_memory(self.ubo_mem);
            self.device.destroy_buffer(self.ubo, None);
            self.device.free_memory(self.ubo_mem, None);
            self.device.destroy_buffer(self.vbo, None);
            self.device.free_memory(self.vbo_mem, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_descriptor_pool(self.desc_pool, None);
            self.device.destroy_descriptor_set_layout(self.dsl, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.destroy_swapchain();
            self.device.destroy_device(None);
            self.surface_ext.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

const DEV_EXT: &CStr = c"VK_KHR_swapchain";

impl Vulkan {
    pub fn new(
        display_ptr: *mut c_void,
        surface_ptr: *mut c_void,
        w: u32,
        h: u32,
        rgba: &[u8],
        img_w: u32,
        img_h: u32,
        bg_clear: [f32; 4],
    ) -> Self {
        let entry = unsafe { Entry::load().expect("vulkan loader") };

        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_3)
            .application_name(CStr::from_bytes_with_nul(b"zen-image\0").unwrap())
            .engine_version(1);
        let ext_names = [
            CStr::from_bytes_with_nul(b"VK_KHR_surface\0").unwrap(),
            CStr::from_bytes_with_nul(b"VK_KHR_wayland_surface\0").unwrap(),
        ];
        let names: Vec<*const std::os::raw::c_char> =
            ext_names.iter().map(|c| c.as_ptr()).collect();
        let inst_ci = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&names);
        let instance = unsafe { entry.create_instance(&inst_ci, None).expect("instance") };

        let surface_ext = ash::khr::surface::Instance::new(&entry, &instance);
        let wayland_ext = ash::khr::wayland_surface::Instance::new(&entry, &instance);

        let surface_ci = vk::WaylandSurfaceCreateInfoKHR::default()
            .display(display_ptr as *mut vk::wl_display)
            .surface(surface_ptr as *mut vk::wl_surface);
        let surface = unsafe {
            wayland_ext
                .create_wayland_surface(&surface_ci, None)
                .expect("wayland surface")
        };

        let phys = Self::pick_physical_device(&instance, &surface_ext, surface);
        let queue_family = Self::pick_queue_family(&instance, &surface_ext, phys, surface);

        let queue_priorities = [1.0];
        let queue_ci = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&queue_priorities);
        let dev_exts = [DEV_EXT.as_ptr()];
        let qcis = [queue_ci];
        let dev_ci = vk::DeviceCreateInfo::default()
            .queue_create_infos(&qcis)
            .enabled_extension_names(&dev_exts);
        let device = unsafe { instance.create_device(phys, &dev_ci, None).expect("device") };
        let queue = unsafe { device.get_device_queue(queue_family, 0) };
        let swap_ext = ash::khr::swapchain::Device::new(&instance, &device);

        let format = Self::pick_format(&surface_ext, phys, surface);
        let present_mode = Self::pick_present_mode(&surface_ext, phys, surface);

        let mut v = Vulkan {
            instance,
            _entry: entry,
            phys,
            device,
            queue_family,
            queue,
            surface,
            surface_ext,
            swap_ext,
            format,
            present_mode,
            extent: vk::Extent2D { width: w, height: h },
            swapchain: vk::SwapchainKHR::null(),
            images: Vec::new(),
            views: Vec::new(),
            framebuffers: Vec::new(),
            render_pass: vk::RenderPass::null(),
            dsl: vk::DescriptorSetLayout::null(),
            desc_pool: vk::DescriptorPool::null(),
            desc_set: vk::DescriptorSet::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            vbo: vk::Buffer::null(),
            vbo_mem: vk::DeviceMemory::null(),
            ubo: vk::Buffer::null(),
            ubo_mem: vk::DeviceMemory::null(),
            ubo_ptr: std::ptr::null_mut(),
            texture: vk::Image::null(),
            texture_mem: vk::DeviceMemory::null(),
            texture_view: vk::ImageView::null(),
            sampler: vk::Sampler::null(),
            cmd_pool: vk::CommandPool::null(),
            cmd: vk::CommandBuffer::null(),
            acquire_fence: vk::Fence::null(),
            bg_clear,
        };

        v.create_swapchain();
        v.create_render_pass();
        v.create_commands();
        v.create_texture(rgba, img_w, img_h);
        v.create_vertex_ubo();
        let vs_code = crate::wgsl::compile(crate::wgsl::VERTEX_WGSL);
        let fs_code = crate::wgsl::compile(crate::wgsl::FRAGMENT_WGSL);
        let vs_mod = unsafe { v.make_shader(&vs_code) };
        let fs_mod = unsafe { v.make_shader(&fs_code) };
        v.create_pipeline(&vs_mod, &fs_mod);
        unsafe {
            v.device.destroy_shader_module(fs_mod, None);
            v.device.destroy_shader_module(vs_mod, None);
        }
        v
    }

    unsafe fn make_shader(&self, code: &[u32]) -> vk::ShaderModule {
        let ci = vk::ShaderModuleCreateInfo::default().code(code);
        unsafe { self.device.create_shader_module(&ci, None).expect("shader") }
    }

    fn pick_physical_device(
        instance: &Instance,
        surface_ext: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> vk::PhysicalDevice {
        let devs = unsafe { instance.enumerate_physical_devices().expect("physdevs") };
        for phys in devs {
            let props = unsafe { instance.get_physical_device_properties(phys) };
            let families = unsafe { instance.get_physical_device_queue_family_properties(phys) };
            let good = families.iter().enumerate().any(|(i, f)| {
                f.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    && unsafe {
                        surface_ext
                            .get_physical_device_surface_support(phys, i as u32, surface)
                            .unwrap_or(false)
                    }
            });
            if good {
                eprintln!(
                    "GPU: {}",
                    unsafe { CStr::from_ptr(props.device_name.as_ptr()) }.to_string_lossy()
                );
                return phys;
            }
        }
        panic!("no suitable physical device");
    }

    fn pick_queue_family(
        instance: &Instance,
        surface_ext: &ash::khr::surface::Instance,
        phys: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> u32 {
        let families = unsafe { instance.get_physical_device_queue_family_properties(phys) };
        for (i, f) in families.iter().enumerate() {
            if f.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && unsafe {
                    surface_ext
                        .get_physical_device_surface_support(phys, i as u32, surface)
                        .unwrap_or(false)
                }
            {
                return i as u32;
            }
        }
        panic!("no graphics+present queue family");
    }

    fn pick_format(
        surface_ext: &ash::khr::surface::Instance,
        phys: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> vk::Format {
        let formats = unsafe {
            surface_ext
                .get_physical_device_surface_formats(phys, surface)
                .expect("formats")
        };
        for wf in [vk::Format::B8G8R8A8_SRGB, vk::Format::R8G8B8A8_SRGB] {
            if formats.iter().any(|f| f.format == wf) {
                return wf;
            }
        }
        formats[0].format
    }

    fn pick_present_mode(
        surface_ext: &ash::khr::surface::Instance,
        phys: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> vk::PresentModeKHR {
        let modes = unsafe {
            surface_ext
                .get_physical_device_surface_present_modes(phys, surface)
                .expect("present modes")
        };
        if modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else {
            vk::PresentModeKHR::FIFO
        }
    }

    fn pick_extent(caps: &vk::SurfaceCapabilitiesKHR, w: u32, h: u32) -> vk::Extent2D {
        if caps.current_extent.width != u32::MAX {
            return caps.current_extent;
        }
        vk::Extent2D {
            width: w.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: h.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        }
    }

    fn memory_type(&self, flags: vk::MemoryPropertyFlags) -> u32 {
        let props = unsafe { self.instance.get_physical_device_memory_properties(self.phys) };
        for (i, mt) in props.memory_types.iter().enumerate() {
            if mt.property_flags.contains(flags) {
                return i as u32;
            }
        }
        panic!("no suitable memory type");
    }

    fn create_swapchain(&mut self) {
        let caps = unsafe {
            self.surface_ext
                .get_physical_device_surface_capabilities(self.phys, self.surface)
                .expect("caps")
        };
        let extent = Self::pick_extent(&caps, self.extent.width, self.extent.height);
        self.extent = extent;

        let mut min_images = caps.min_image_count.saturating_add(1);
        if caps.max_image_count > 0 && min_images > caps.max_image_count {
            min_images = caps.max_image_count;
        }

        let composite = if caps
            .supported_composite_alpha
            .contains(vk::CompositeAlphaFlagsKHR::OPAQUE)
        {
            vk::CompositeAlphaFlagsKHR::OPAQUE
        } else {
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
        };

        let ci = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(min_images)
            .image_format(self.format)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(composite)
            .present_mode(self.present_mode)
            .clipped(true)
            .old_swapchain(self.swapchain);
        self.swapchain =
            unsafe { self.swap_ext.create_swapchain(&ci, None).expect("swapchain") };
        self.rebuild_views();
    }

    fn destroy_swapchain(&mut self) {
        unsafe {
            for f in self.framebuffers.drain(..) {
                self.device.destroy_framebuffer(f, None);
            }
            for v in self.views.drain(..) {
                self.device.destroy_image_view(v, None);
            }
            if self.swapchain != vk::SwapchainKHR::null() {
                self.swap_ext.destroy_swapchain(self.swapchain, None);
                self.swapchain = vk::SwapchainKHR::null();
            }
        }
    }

    fn rebuild_views(&mut self) {
        unsafe {
            for f in self.framebuffers.drain(..) {
                self.device.destroy_framebuffer(f, None);
            }
            for v in self.views.drain(..) {
                self.device.destroy_image_view(v, None);
            }
            self.images = self
                .swap_ext
                .get_swapchain_images(self.swapchain)
                .expect("swapchain images");
            for img in &self.images {
                let ci = vk::ImageViewCreateInfo::default()
                    .image(*img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(self.format)
                    .components(vk::ComponentMapping::default())
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .level_count(1)
                            .layer_count(1),
                    );
                let view = self.device.create_image_view(&ci, None).expect("image view");
                self.views.push(view);
            }
            if self.render_pass != vk::RenderPass::null() {
                for v in &self.views {
                    let atts = [*v];
                    let ci = vk::FramebufferCreateInfo::default()
                        .render_pass(self.render_pass)
                        .attachments(&atts)
                        .width(self.extent.width)
                        .height(self.extent.height)
                        .layers(1);
                    let fb = self.device.create_framebuffer(&ci, None).expect("framebuffer");
                    self.framebuffers.push(fb);
                }
            }
        }
    }

    pub fn rebuild(&mut self) {
        unsafe { self.device.device_wait_idle().ok() };
        self.destroy_swapchain();
        self.create_swapchain();
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        unsafe { self.device.device_wait_idle().ok() };
        self.destroy_swapchain();
        self.extent.width = w.max(1);
        self.extent.height = h.max(1);
        self.create_swapchain();
    }

    pub fn extents(&self) -> (u32, u32) {
        (self.extent.width, self.extent.height)
    }

    fn create_render_pass(&mut self) {
        let attach = vk::AttachmentDescription::default()
            .format(self.format)
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
        let atts = [attach];
        let subs = [subpass];
        let rp = vk::RenderPassCreateInfo::default()
            .attachments(&atts)
            .subpasses(&subs);
        self.render_pass =
            unsafe { self.device.create_render_pass(&rp, None).expect("render pass") };
        self.rebuild_views();
    }

    fn create_texture(&mut self, rgba: &[u8], img_w: u32, img_h: u32) {
        // device-local texture image
        let image_ci = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_SRGB)
            .extent(vk::Extent3D { width: img_w, height: img_h, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        self.texture =
            unsafe { self.device.create_image(&image_ci, None).expect("texture image") };
        let reqs = unsafe { self.device.get_image_memory_requirements(self.texture) };
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(reqs.size)
            .memory_type_index(self.memory_type(vk::MemoryPropertyFlags::DEVICE_LOCAL));
        self.texture_mem =
            unsafe { self.device.allocate_memory(&alloc, None).expect("tex mem") };
        unsafe {
            self.device
                .bind_image_memory(self.texture, self.texture_mem, 0)
                .expect("bind tex")
        };

        // staging buffer
        let staging = unsafe {
            self.device
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(rgba.len() as u64)
                        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE),
                    None,
                )
                .expect("staging")
        };
        let sreq = unsafe { self.device.get_buffer_memory_requirements(staging) };
        let salloc = vk::MemoryAllocateInfo::default()
            .allocation_size(sreq.size)
            .memory_type_index(self.memory_type(
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ));
        let smem = unsafe { self.device.allocate_memory(&salloc, None).expect("stag mem") };
        unsafe { self.device.bind_buffer_memory(staging, smem, 0).expect("bind stag") };
        let ptr = unsafe {
            self.device
                .map_memory(smem, 0, sreq.size, vk::MemoryMapFlags::empty())
                .expect("map stag")
        };
        unsafe {
            std::ptr::copy_nonoverlapping(rgba.as_ptr(), ptr as *mut u8, rgba.len());
            self.device.unmap_memory(smem);
        }

        // one-shot upload
        let ai = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmd = unsafe { self.device.allocate_command_buffers(&ai).expect("cmd alloc")[0] };
        unsafe {
            self.device
                .begin_command_buffer(
                    cmd,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .expect("cmd begin");
        }

        let to_transfer = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
            .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.texture)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );
        let barriers = [to_transfer];
        let dep = vk::DependencyInfo::default().image_memory_barriers(&barriers);
        unsafe { self.device.cmd_pipeline_barrier2(cmd, &dep) };

        let region = vk::BufferImageCopy::default()
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1),
            )
            .image_extent(vk::Extent3D { width: img_w, height: img_h, depth: 1 });
        unsafe {
            self.device
                .cmd_copy_buffer_to_image(
                    cmd,
                    staging,
                    self.texture,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[region],
                );
        }

        let to_read = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.texture)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );
        let barriers2 = [to_read];
        let dep2 = vk::DependencyInfo::default().image_memory_barriers(&barriers2);
        unsafe { self.device.cmd_pipeline_barrier2(cmd, &dep2) };

        unsafe {
            self.device.end_command_buffer(cmd).expect("cmd end");
            self.device
                .queue_submit(
                    self.queue,
                    &[vk::SubmitInfo::default().command_buffers(&[cmd])],
                    vk::Fence::null(),
                )
                .expect("upload submit");
            self.device.queue_wait_idle(self.queue).expect("upload wait");
            self.device.free_command_buffers(self.cmd_pool, &[cmd]);
            self.device.destroy_buffer(staging, None);
            self.device.free_memory(smem, None);
        }

        let view_ci = vk::ImageViewCreateInfo::default()
            .image(self.texture)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_SRGB)
            .components(vk::ComponentMapping::default())
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );
        self.texture_view =
            unsafe { self.device.create_image_view(&view_ci, None).expect("tex view") };

        if self.sampler == vk::Sampler::null() {
            let samp = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .max_lod(0.0);
            self.sampler =
                unsafe { self.device.create_sampler(&samp, None).expect("sampler") };
        }
    }

    pub fn set_texture(&mut self, rgba: &[u8], w: u32, h: u32) {
        unsafe { self.device.device_wait_idle().ok() };
        unsafe {
            if self.texture_view != vk::ImageView::null() {
                self.device.destroy_image_view(self.texture_view, None);
            }
            if self.texture_mem != vk::DeviceMemory::null() {
                self.device.free_memory(self.texture_mem, None);
            }
            if self.texture != vk::Image::null() {
                self.device.destroy_image(self.texture, None);
            }
        }
        self.texture = vk::Image::null();
        self.texture_mem = vk::DeviceMemory::null();
        self.texture_view = vk::ImageView::null();
        self.create_texture(rgba, w, h);

        let img_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.texture_view);
        let writes = [vk::WriteDescriptorSet::default()
            .dst_set(self.desc_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .image_info(std::slice::from_ref(&img_info))];
        unsafe { self.device.update_descriptor_sets(&writes, &[]) };
    }

    fn create_vertex_ubo(&mut self) {
        self.dsl = unsafe {
            self.device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                        vk::DescriptorSetLayoutBinding::default()
                            .binding(0)
                            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                            .descriptor_count(1)
                            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                        vk::DescriptorSetLayoutBinding::default()
                            .binding(1)
                            .descriptor_type(vk::DescriptorType::SAMPLER)
                            .descriptor_count(1)
                            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                        vk::DescriptorSetLayoutBinding::default()
                            .binding(2)
                            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                            .descriptor_count(1)
                            .stage_flags(vk::ShaderStageFlags::VERTEX),
                    ]),
                    None,
                )
                .expect("dsl")
        };

        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: 1,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            },
        ];
        self.desc_pool = unsafe {
            self.device
                .create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default()
                        .max_sets(1)
                        .pool_sizes(&pool_sizes),
                    None,
                )
                .expect("desc pool")
        };

        // vertex buffer: fullscreen quad (unit coords)
        const QUAD: [[f32; 4]; 6] = [
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 0.0, 1.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
        ];
        let vbo_ci = vk::BufferCreateInfo::default()
            .size(bytemuck::bytes_of(&QUAD).len() as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        self.vbo = unsafe {
            self.device.create_buffer(&vbo_ci, None).expect("vbo")
        };
        let vreq = unsafe { self.device.get_buffer_memory_requirements(self.vbo) };
        self.vbo_mem = unsafe {
            self.device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(vreq.size)
                    .memory_type_index(self.memory_type(
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    )),
                None,
            )
            .expect("vbo mem")
        };
        unsafe { self.device.bind_buffer_memory(self.vbo, self.vbo_mem, 0).expect("vbo bind") };
        let vptr = unsafe {
            self.device
                .map_memory(self.vbo_mem, 0, vreq.size, vk::MemoryMapFlags::empty())
                .expect("vbo map")
        };
        unsafe {
            std::ptr::copy_nonoverlapping(QUAD.as_ptr().cast::<f32>(), vptr as *mut f32, 24);
            self.device.unmap_memory(self.vbo_mem);
        }

        // uniform buffer
        self.ubo = unsafe {
            self.device
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(UBO_SIZE)
                        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE),
                    None,
                )
                .expect("ubo")
        };
        let ureq = unsafe { self.device.get_buffer_memory_requirements(self.ubo) };
        self.ubo_mem = unsafe {
            self.device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(ureq.size)
                    .memory_type_index(self.memory_type(
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    )),
                None,
            )
            .expect("ubo mem")
        };
        unsafe { self.device.bind_buffer_memory(self.ubo, self.ubo_mem, 0).expect("ubo bind") };
        self.ubo_ptr = unsafe {
            self.device
                .map_memory(self.ubo_mem, 0, UBO_SIZE, vk::MemoryMapFlags::empty())
                .expect("ubo map")
        } as *mut u8;

        // descriptor set
        let dsls = [self.dsl];
        let ai = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.desc_pool)
            .set_layouts(&dsls);
        self.desc_set =
            unsafe { self.device.allocate_descriptor_sets(&ai).expect("desc set")[0] };

        let img_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.texture_view);
        let sam_info = vk::DescriptorImageInfo::default().sampler(self.sampler);
        let buf_info = vk::DescriptorBufferInfo::default()
            .buffer(self.ubo)
            .offset(0)
            .range(UBO_SIZE);
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(self.desc_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(std::slice::from_ref(&img_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(self.desc_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(std::slice::from_ref(&sam_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(self.desc_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buf_info)),
        ];
        unsafe { self.device.update_descriptor_sets(&writes, &[]) };

        self.pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::default().set_layouts(&[self.dsl]),
                    None,
                )
                .expect("pipeline layout")
        };
    }

    fn create_pipeline(&mut self, vs: &vk::ShaderModule, fs: &vk::ShaderModule) {
        let vs_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(*vs)
            .name(CStr::from_bytes_with_nul(b"vs_main\0").unwrap());
        let fs_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(*fs)
            .name(CStr::from_bytes_with_nul(b"fs_main\0").unwrap());

        let binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(16)
            .input_rate(vk::VertexInputRate::VERTEX);
        let attrs = [
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(8),
        ];
        let bindings = [binding];
        let vis = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attrs);
        let ias = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let ras = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE);
        let ms = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let cb = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            );
        let cbs1 = [cb];
        let cbs = vk::PipelineColorBlendStateCreateInfo::default().attachments(&cbs1);
        let dyns2 = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dyns = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dyns2);

        let stages = [vs_stage, fs_stage];
        let ci = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vis)
            .input_assembly_state(&ias)
            .rasterization_state(&ras)
            .multisample_state(&ms)
            .color_blend_state(&cbs)
            .dynamic_state(&dyns)
            .layout(self.pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0);
        self.pipeline = unsafe {
            self.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[ci], None)
                .expect("pipelines")[0]
        };
    }

    fn create_commands(&mut self) {
        let pool_ci = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(self.queue_family);
        self.cmd_pool =
            unsafe { self.device.create_command_pool(&pool_ci, None).expect("cmd pool") };

        let ai = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        self.cmd = unsafe { self.device.allocate_command_buffers(&ai).expect("cmdbuf")[0] };

        self.acquire_fence = unsafe {
            self.device
                .create_fence(&vk::FenceCreateInfo::default(), None)
                .expect("fence")
        };
    }

    fn update_ubo(&self, view: &View) {
        let data: [f32; 4] = [view.scale[0], view.scale[1], view.offset[0], view.offset[1]];
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ubo_ptr as *mut f32, 4);
        }
    }

    pub fn render(&mut self, view: &View) -> RenderStatus {
        unsafe {
            self.device.reset_fences(&[self.acquire_fence]).expect("reset fence");
        }
        let acquired = unsafe {
            self.swap_ext.acquire_next_image(
                self.swapchain,
                100_000_000,
                vk::Semaphore::null(),
                self.acquire_fence,
            )
        };
        let (idx, _suboptimal) = match acquired {
            Ok(r) => r,
            Err(vk::Result::NOT_READY) | Err(vk::Result::TIMEOUT) => return RenderStatus::Busy,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::ERROR_SURFACE_LOST_KHR) => {
                return RenderStatus::OutOfDate
            }
            Err(e) => panic!("acquire failed: {e:?}"),
        };

        unsafe {
            self.device
                .wait_for_fences(&[self.acquire_fence], true, u64::MAX)
                .expect("wait acquire");
        }

        self.update_ubo(view);

        unsafe {
            self.device
                .reset_command_buffer(self.cmd, vk::CommandBufferResetFlags::empty())
                .expect("reset cmd");
            let cbi = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.cmd, &cbi).expect("begin");

            let clear = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: self.bg_clear,
                },
            };
            let clears = [clear];
            let rpbi = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[idx as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.extent,
                })
                .clear_values(&clears);
            self.device
                .cmd_begin_render_pass(self.cmd, &rpbi, vk::SubpassContents::INLINE);

            let viewport = vk::Viewport::default()
                .width(self.extent.width as f32)
                .height(self.extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0);
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent,
            };
            self.device.cmd_set_viewport(self.cmd, 0, &[viewport]);
            self.device.cmd_set_scissor(self.cmd, 0, &[scissor]);
            self.device
                .cmd_bind_pipeline(self.cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            self.device.cmd_bind_descriptor_sets(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.desc_set],
                &[],
            );
            self.device.cmd_bind_vertex_buffers(self.cmd, 0, &[self.vbo], &[0]);
            self.device.cmd_draw(self.cmd, 6, 1, 0, 0);
            self.device.cmd_end_render_pass(self.cmd);
            self.device.end_command_buffer(self.cmd).expect("end");
        }

        unsafe {
            let cbs = [self.cmd];
            let si = vk::SubmitInfo::default().command_buffers(&cbs);
            self.device
                .queue_submit(self.queue, &[si], vk::Fence::null())
                .expect("submit");
            self.device.queue_wait_idle(self.queue).expect("wait idle");
        }

        unsafe {
            let scs = [self.swapchain];
            let idxs = [idx];
            let pi = vk::PresentInfoKHR::default()
                .swapchains(&scs)
                .image_indices(&idxs);
            let pres = self.swap_ext.queue_present(self.queue, &pi);
            if let Err(vk::Result::ERROR_OUT_OF_DATE_KHR) = pres {
                return RenderStatus::OutOfDate;
            }
        }
        RenderStatus::Rendered
    }
}