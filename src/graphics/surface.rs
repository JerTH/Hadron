use ash::vk;

use crate::graphics::vulkangfx::{GraphicsDevice, QueueFamilies};

pub(crate) struct GraphicsSurface {
    _wayland_surface_loader: ash::extensions::khr::WaylandSurface,
    surface_loader: ash::extensions::khr::Surface,
    surface: ash::vk::SurfaceKHR,
}

impl GraphicsSurface {
    pub(crate) fn init(window: &winit::window::Window, entry: &ash::Entry, instance: &ash::Instance) -> Result<Self, vk::Result> {
        use winit::platform::unix::WindowExtUnix;
        let wayland_display = window.wayland_display().unwrap();
        let wayland_surface = window.wayland_surface().unwrap();
        let wayland_create_info = vk::WaylandSurfaceCreateInfoKHR::builder()
            .display(wayland_display)
            .surface(wayland_surface);
        let wayland_surface_loader = ash::extensions::khr::WaylandSurface::new(&entry, &instance);
        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface = unsafe { wayland_surface_loader.create_wayland_surface(&wayland_create_info, None) }?;

        Ok( Self {
            _wayland_surface_loader: wayland_surface_loader,
            surface_loader,
            surface,
        })
    }

    pub(crate) fn get_capabilities(&self, physical_device: vk::PhysicalDevice) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_capabilities(physical_device, self.surface)
        }
    }

    pub(crate) fn get_present_modes(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::PresentModeKHR>, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_present_modes(physical_device, self.surface)
        }
    }

    pub(crate) fn get_formats(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_formats(physical_device, self.surface)
        }
    }

    /// Determines whether a queue family of a physical device supports presentation to a given surface
    pub(crate) fn get_physical_device_surface_support(&self, physical_device: vk::PhysicalDevice, queue_family_index: usize) -> Result<bool, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_support(physical_device, queue_family_index as u32, self.surface)
        }
    }
}

impl Drop for GraphicsSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

pub(crate) struct Swapchain {
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    imageviews: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    _surface_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    image_available: Vec<vk::Semaphore>,
    rendering_finished: Vec<vk::Semaphore>,
    draw_fences: Vec<vk::Fence>,
    current_image: usize,
}

impl Swapchain {
    pub fn init(instance: &ash::Instance, physical_device: vk::PhysicalDevice, graphics_device: &GraphicsDevice, surfaces: &GraphicsSurface, queue_families: &QueueFamilies) -> Result<Self, vk::Result> {
        let surface_capabilities = surfaces.get_capabilities(physical_device)?;

        let _surface_present_modes = surfaces.get_present_modes(physical_device)?;
        let surface_format = *surfaces.get_formats(physical_device)?.first().unwrap();
        let vec_queue_families = vec![queue_families.graphics_queue_index().unwrap()];
        let vk_surface = surfaces.surface;
        let logical_device = graphics_device.logical_device();

        let (image_width, image_height) = (800,600);
        let extent = vk::Extent2D::builder().width(image_width).height(image_height).build();

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(vk_surface)
            .min_image_count(3.max(surface_capabilities.min_image_count))
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)             //  <--- Change this to a real extent
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&vec_queue_families)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);    
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &logical_device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        
        // Create image views from the swapchain images
        let mut imageviews = Vec::with_capacity(images.len());
        for image in &images {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let imageview_create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_SRGB)
                .subresource_range(*subresource_range);

            let imageview = unsafe { logical_device.create_image_view(&imageview_create_info, None) }?;
            imageviews.push(imageview);
        };

        let mut image_available = vec![];
        let mut rendering_finished = vec![];
        let mut draw_fences = vec![];
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        let fence_create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        for _ in 0..images.len() {
            let semaphore_available = unsafe { logical_device.create_semaphore(&semaphore_create_info, None) }?;
            let semaphore_finished = unsafe { logical_device.create_semaphore(&semaphore_create_info, None) }?;
            let drawing_fence = unsafe { logical_device.create_fence(&fence_create_info, None) }?; 
            image_available.push(semaphore_available);
            rendering_finished.push(semaphore_finished);
            draw_fences.push(drawing_fence)
        }

        Ok( Swapchain {
            swapchain_loader,
            swapchain,
            images,
            imageviews,
            framebuffers: Vec::new(),
            _surface_format: surface_format,
            extent,
            image_available,
            rendering_finished,
            draw_fences,
            current_image: 0usize,
        })
    }
    
    pub fn present(&self, image_index: usize, queue: vk::Queue) {
        let semaphores_finished = [self.rendering_finished[self.current_image()]];
        let swapchains = [self.swapchain];
        let indices = [image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&semaphores_finished)
            .swapchains(&swapchains)
            .image_indices(&indices);

        unsafe {
            self.swapchain_loader.queue_present(queue, &present_info).expect("queue presentation error");
        }
    }

    pub fn create_framebuffers(&mut self, graphics_device: &GraphicsDevice, renderpass: vk::RenderPass) -> Result<(), vk::Result> {
        for imageview in &self.imageviews {
            let iview = [*imageview];
            let framebuffer_info = vk::FramebufferCreateInfo::builder()
                .render_pass(renderpass)
                .attachments(&iview)
                .width(self.extent.width)
                .height(self.extent.height)
                .layers(1);
            let fb = graphics_device.create_framebuffer(&framebuffer_info)?;
            self.framebuffers.push(fb);
        }
        Ok(())
    }

    pub fn framebuffer_count(&self) -> usize {
        self.framebuffers.len()
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn next_image(&mut self) -> usize {
        self.current_image = (self.current_image + 1) % self.images.len();

        let (_image_index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                10_000_000u64,
                self.image_available[self.current_image],
                vk::Fence::null()
            ).expect("next_image unable to acquire")
        };

        self.current_image
    }
    
    pub unsafe fn cleanup(&mut self, graphics_device: &GraphicsDevice) {
        graphics_device.logical_device().device_wait_idle().expect("Error during device_wait_idle in swapchain cleanup");
        
        for fence in &self.draw_fences {
            graphics_device.destroy_fence(*fence);
        }

        for semaphore in &self.image_available {
            graphics_device.destroy_semaphore(*semaphore);
        }

        for semaphore in &self.rendering_finished {
            graphics_device.destroy_semaphore(*semaphore);
        }

        for framebuffer in &self.framebuffers {
            unsafe { graphics_device.destroy_framebuffer(*framebuffer) }
        }
        for imageview in &self.imageviews {
            unsafe { graphics_device.destroy_image_view(*imageview) };
        }
        self.swapchain_loader.destroy_swapchain(self.swapchain, None);
    }

    pub(crate) fn framebuffer(&self, i: usize) -> vk::Framebuffer {
        self.framebuffers[i]
    }

    pub(crate) fn current_image(&self) -> usize {
        self.current_image
    }

    pub(crate) fn draw_fences(&self) -> &Vec<vk::Fence> {
        &self.draw_fences
    }

    //pub(crate) fn submit_commandbuffer(&self, image_index: usize, command_buffers: &[vk::CommandBuffer]) {
    //    todo!()
    //}

    pub(crate) fn image_finished_semaphore(&self) -> vk::Semaphore {
        self.rendering_finished[self.current_image()]
    }

    pub(crate) fn image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available[self.current_image()]
    }
}
