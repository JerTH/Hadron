use std::rc::Rc;

use ash::vk;
use crate::graphics::{ debug, surface, render };

/**
 * Setup
 * - Initialize Vulkan
 *  - Setup debugging/logs
 *  - Setup validation layers
 * - Choose physical device
 *  - Query queue families
 *  - Create a set of queues, possibly only one
 * - Create a window
 * - Create a surface
 *  - Link the surface to the window
 * - Create a swapchain
 *  - Create associated images, image views, and framebuffers
 * 
 * 
 * 
 * Rendering
 * Commands are recorded into command buffers
 * 
**/

#[deprecated]
pub(crate) struct TVulkanGraphics {
    window: Rc<winit::window::Window>,
    entry: ash::Entry,
    instance: ash::Instance,
    debug: std::mem::ManuallyDrop<debug::VulkanDebugWidget>,
    surfaces: std::mem::ManuallyDrop<surface::GraphicsSurface>,
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    queue_families: QueueFamilies,
    graphics_device: GraphicsDevice,
    swapchain: surface::Swapchain,
    renderpass: vk::RenderPass,
    pipeline: render::Pipeline,
    command_pools: CommandPools,
    command_buffers: Vec<vk::CommandBuffer>,
}

impl TVulkanGraphics {
    pub(crate) fn init(window: Rc<winit::window::Window>) -> Result<Self, vk::Result> {
        let entry = unsafe { ash::Entry::load().expect("couldn't load Vulkan entry point") };
        
        let layers = debug::ValidationLayers::init()?;
        let instance = init_vulkan_instance(&entry, &layers)?;
        let debug = debug::VulkanDebugWidget::init(&entry, &instance)?;
        let surfaces = surface::GraphicsSurface::init(&window, &entry, &instance)?;
        let (physical_device, physical_device_properties) = choose_physical_device(&instance)?;
        let queue_families = QueueFamilies::init(&instance, physical_device, &surfaces)?;
        let graphics_device = GraphicsDevice::init(&instance, physical_device, &queue_families, layers)?;
        let mut swapchain = surface::Swapchain::init(&instance, physical_device, &graphics_device, &surfaces, &queue_families)?;
        let renderpass = render::init_renderpass(&graphics_device, physical_device, &surfaces)?;
        swapchain.create_framebuffers(&graphics_device, renderpass)?;
        let pipeline = render::Pipeline::init(&graphics_device, &swapchain, &renderpass)?;
        let command_pools = CommandPools::init(&graphics_device, &queue_families)?;
        let command_buffers = create_commandbuffers(&graphics_device, &command_pools, swapchain.framebuffer_count())?;
        
        fill_command_buffers(&command_buffers, renderpass, &swapchain, &pipeline, &graphics_device)?;

        Ok(TVulkanGraphics {
            window,
            entry,
            instance,
            debug: std::mem::ManuallyDrop::new(debug),
            surfaces: std::mem::ManuallyDrop::new(surfaces),
            physical_device,
            physical_device_properties,
            queue_families,
            graphics_device,
            swapchain,
            renderpass,
            pipeline,
            command_pools,
            command_buffers,
        })
    }

    pub(crate) fn graphics_device(&self) -> &GraphicsDevice {
        &self.graphics_device
    }
    
    pub(crate) fn swapchain(&self) -> &surface::Swapchain {
        &self.swapchain
    }

    pub(crate) fn command_buffers(&self) -> &[vk::CommandBuffer] {
        &self.command_buffers
    }

    pub(crate) fn wait_for_fences(&self) {
        self.graphics_device.wait_for_fences(&self.swapchain)
    }

    pub(crate) fn reset_fences(&self) {
        self.graphics_device.reset_fences(&self.swapchain)
    }
    
    pub(crate) fn submit_commandbuffer(&self, image_index: usize) {
        self.graphics_device.submit_commandbuffer(image_index, &self.command_buffers, &self.swapchain)
    }

    pub(crate) fn next_image(&mut self) -> usize {
        self.swapchain.next_image()
    }
}

impl Drop for TVulkanGraphics {
    fn drop(&mut self) {
        unsafe {
            // Extracts the logical device and drops the queues
            let logical_device = self.graphics_device.logical_device();

            self.pipeline.cleanup(&logical_device);
            
            logical_device.destroy_render_pass(self.renderpass, None);

            self.swapchain.cleanup(&self.graphics_device);
            
            logical_device.destroy_device(None);
            
            std::mem::ManuallyDrop::drop(&mut self.surfaces);
            std::mem::ManuallyDrop::drop(&mut self.debug);
            
            self.instance.destroy_instance(None);
        }
    }
}

/// Instance Creation
pub(crate) fn init_vulkan_instance(entry: &ash::Entry, layers: &debug::ValidationLayers) -> Result<ash::Instance, vk::Result> {
    let enginename = std::ffi::CString::new("Hadron").unwrap();
    let appname = std::ffi::CString::new("Infinity").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&appname)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&enginename)
        .engine_version(vk::make_api_version(0, 0, 0, 0))
        .api_version(vk::make_api_version(0, 1, 2, 0));

    let extension_name_pointers: Vec<*const i8> =
        vec![
            ash::extensions::ext::DebugUtils::name().as_ptr(),
            ash::extensions::khr::Surface::name().as_ptr(),
            ash::extensions::khr::WaylandSurface::name().as_ptr(),
        ];

    let mut debug_create_info = debug::init_debug_info()?;

    // Instance creation
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_create_info)
        .application_info(&app_info)
        .enabled_layer_names(layers.layer_name_pointers())
        .enabled_extension_names(&extension_name_pointers);

    Ok(unsafe { entry.create_instance(&instance_create_info, None)? })
}

pub(crate) struct CommandPools {
    commandpool_graphics: vk::CommandPool,
    commandpool_transfer: Option<vk::CommandPool>,
}

impl CommandPools {
    pub(crate) fn init(graphics_device: &GraphicsDevice, queue_families: &QueueFamilies) -> Result<CommandPools, vk::Result> {
        let graphics_commandpool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.graphics_queue_index().unwrap())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let commandpool_graphics = graphics_device.create_command_pool(&graphics_commandpool_info)?;
        
        let commandpool_transfer = if let Some(transfer_queue_index) = queue_families.transfer_queue_index() {
            let transfer_commandpool_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(transfer_queue_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

                Some( graphics_device.create_command_pool(&transfer_commandpool_info)? )
        } else {
            None
        };

        Ok(CommandPools {
            commandpool_graphics,
            commandpool_transfer
        })
    }

    pub(crate) fn cleanup(&self, graphics_device: &GraphicsDevice) {
        unsafe {
            graphics_device.destroy_command_pool(self.commandpool_graphics);
            
            if let Some(commandpool_transfer) = self.commandpool_transfer {
                graphics_device.destroy_command_pool(commandpool_transfer);
            }
        }
    }
}

pub(crate) fn create_commandbuffers(graphics_device: &GraphicsDevice, pools: &CommandPools, count: usize) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    
    let command_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(pools.commandpool_graphics)
        .command_buffer_count(count as u32);

    
    graphics_device.allocate_command_buffers(&command_allocate_info)
}

pub(crate) fn fill_command_buffers(
    command_buffers: &Vec<vk::CommandBuffer>,
    renderpass: vk::RenderPass,
    swapchain: &surface::Swapchain,
    pipeline: &render::Pipeline,
    graphics_device: &GraphicsDevice,
) -> Result<(), vk::Result> {
    for (i, &command_buffer) in command_buffers.iter().enumerate() {
        unsafe {
            let commandbuffer_begin_info = vk::CommandBufferBeginInfo::builder();
            graphics_device.begin_command_buffer(command_buffer, &commandbuffer_begin_info)?;

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            let renderpass_begininfo = vk::RenderPassBeginInfo::builder()
                .render_pass(renderpass)
                .framebuffer(swapchain.framebuffer(i))
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain.extent()
                })
                .clear_values(&clear_values);
            
            let logical_device = graphics_device.logical_device();
            logical_device.cmd_begin_render_pass(command_buffer, &renderpass_begininfo, vk::SubpassContents::INLINE);
            logical_device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline());
            logical_device.cmd_draw(command_buffer, 1, 1, 0, 0);
            logical_device.cmd_end_render_pass(command_buffer);
            logical_device.end_command_buffer(command_buffer)?;

        }
    }

    Ok(())
}

pub struct GraphicsDevice {
    graphics_queue: vk::Queue,
    transfer_queue: Option<vk::Queue>,
    logical_device: ash::Device,
}

impl GraphicsDevice {
    pub(crate) fn init(instance: &ash::Instance, physical_device: vk::PhysicalDevice, queue_families: &QueueFamilies, layers: debug::ValidationLayers) -> Result<Self, vk::Result> {
        println!("GraphicsDevice::init()");
        
        let layer_name_pointers = layers.layer_name_pointers();
        let queue_priorities = [1.0f32];

        let mut graphics_queue_info = None;
        let mut transfer_queue_info = None;

        let graphics_queue_index = queue_families.graphics_queue_index.unwrap();
        if let Some(transfer_queue_index) = queue_families.transfer_queue_index {
            println!("Has transfer queue index");
            
            // We have both a graphics and transfer queue
            transfer_queue_info = Some(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(transfer_queue_index)
                    .queue_priorities(&queue_priorities)
                    .build()
            );
        };

        graphics_queue_info = Some(
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(graphics_queue_index)
                .queue_priorities(&queue_priorities)
                .build()
        );

        let mut queue_create_infos = Vec::new();
        if let Some(graphics_queue_info) = graphics_queue_info { queue_create_infos.push(graphics_queue_info); }
        if let Some(transfer_queue_info) = transfer_queue_info { queue_create_infos.push(transfer_queue_info); }

        let device_extension_name_pointers = vec![ash::extensions::khr::Swapchain::name().as_ptr()];
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extension_name_pointers)
            .enabled_layer_names(&layer_name_pointers);
        
        let logical_device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };
        let graphics_queue = 
            if let Some(graphics_queue_index) = queue_families.graphics_queue_index {
                Some(unsafe { logical_device.get_device_queue(graphics_queue_index, 0) })
            } else { 
                None
            }.unwrap(); // unwrap/expect for now
        
        let transfer_queue = 
            if let Some(transfer_queue_index) = queue_families.graphics_queue_index {
                Some(unsafe{ logical_device.get_device_queue(transfer_queue_index, 0)})
            } else {
                None
            };
        
        Ok(GraphicsDevice {
            graphics_queue,
            transfer_queue,
            logical_device,
        })
    }

    pub(crate) fn logical_device(&self) -> &ash::Device {
        &self.logical_device
    }

    pub(crate) fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }

    pub fn allocate_command_buffers(&self, command_allocate_info: &vk::CommandBufferAllocateInfoBuilder) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
        unsafe {
            self.logical_device.allocate_command_buffers(command_allocate_info)
        }
    }
    
    pub fn create_pipeline_layout(&self, create_info: &vk::PipelineLayoutCreateInfoBuilder) -> Result<vk::PipelineLayout, vk::Result> {
        unsafe {
            self.logical_device.create_pipeline_layout(create_info, None)
        }        
    }

    pub fn create_graphics_pipelines(&self, create_infos: &[vk::GraphicsPipelineCreateInfo]) -> Vec<vk::Pipeline> {
        unsafe { 
            self.logical_device.create_graphics_pipelines(vk::PipelineCache::null(), create_infos, None).unwrap()
        }
    }

    pub fn create_shader_module(&self, create_info: &vk::ShaderModuleCreateInfoBuilder) -> Result<vk::ShaderModule, vk::Result> {
        unsafe{
            self.logical_device.create_shader_module(create_info, None)
        }
    }

    pub unsafe fn destroy_shader_module(&self, shader: vk::ShaderModule) {
        self.logical_device.destroy_shader_module(shader, None);
    }
    
    pub fn create_render_pass(&self, create_info: &vk::RenderPassCreateInfoBuilder ) -> Result<vk::RenderPass, vk::Result> {
        unsafe {
            self.logical_device.create_render_pass(create_info, None)
        }
    }
    
    pub fn create_framebuffer(&self, create_info: &vk::FramebufferCreateInfoBuilder) -> Result<vk::Framebuffer, vk::Result>{
        unsafe {
            self.logical_device.create_framebuffer(create_info, None)
        }
    }
    
    pub unsafe fn destroy_framebuffer(&self, framebuffer: vk::Framebuffer) {
        self.logical_device.destroy_framebuffer(framebuffer, None)
    }
    
    pub unsafe fn destroy_image_view(&self, image_view: vk::ImageView) {
        self.logical_device.destroy_image_view(image_view, None)
    }

    pub fn create_command_pool(&self, create_info: &vk::CommandPoolCreateInfo) -> Result<vk::CommandPool, vk::Result> {
        unsafe {
            self.logical_device.create_command_pool(create_info, None)
        }
    }

    pub unsafe fn destroy_command_pool(&self, pool: vk::CommandPool) {
        self.logical_device.destroy_command_pool(pool, None);
    }

    pub(crate) fn begin_command_buffer(&self, command_buffer: vk::CommandBuffer, begin_info: &vk::CommandBufferBeginInfoBuilder) -> Result<(), vk::Result> {
        unsafe {
            self.logical_device.begin_command_buffer(command_buffer, begin_info)
        }
    }
    
    pub unsafe fn destroy_semaphore(&self, semaphore: vk::Semaphore) {
        self.logical_device.destroy_semaphore(semaphore, None);
    }
    
    pub unsafe fn destroy_fence(&self, fence: vk::Fence) {
        self.logical_device.destroy_fence(fence, None);
    }

    #[deprecated]
    pub fn cleanup(self) -> ash::Device {
        self.logical_device
    }
    
    pub(crate) fn wait_for_fences(&self, swapchain: &surface::Swapchain) {
        unsafe {
            self.logical_device.wait_for_fences(
                &[swapchain.draw_fences()[swapchain.current_image()]],
                true,
                100_000_000u64,
            )
            .expect("wait_for_fences error during wait");
        }
    }
    
    pub(crate) fn reset_fences(&self, swapchain: &surface::Swapchain) {
        unsafe {
            self.logical_device.reset_fences(&[
                swapchain.draw_fences()[swapchain.current_image()]
            ]).expect("reset_fences error resetting swapchain fences");
        }
    }
    
    pub(crate) fn submit_commandbuffer(&self, image_index: usize, command_buffers: &[vk::CommandBuffer], swapchain: &surface::Swapchain) {
        let semaphores_available = [swapchain.image_available_semaphore()];
        let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let semaphores_finished = [swapchain.image_finished_semaphore()];
        let command_buffers = [command_buffers[image_index]];
        let submit_info = [vk::SubmitInfo::builder()
            .wait_semaphores(&semaphores_available)
            .wait_dst_stage_mask(&waiting_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&semaphores_finished)
            .build()
        ];

        unsafe {
            self.logical_device.queue_submit(
                self.graphics_queue,
                &submit_info,
                swapchain.draw_fences()[swapchain.current_image()]
            ).expect("submit_commandbuffer queue_submit failed");
        }
    }
}

pub(crate) struct QueueFamilies {
    graphics_queue_index: Option<u32>,
    transfer_queue_index: Option<u32>,
}

impl QueueFamilies {
    pub(crate) fn init(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface: &surface::GraphicsSurface) -> Result<Self, vk::Result> {
        println!("Initializing queue families");

        let queuefamilyproperties = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        
        let mut graphics_queue_index: Option<u32> = None;
        let mut transfer_queue_index: Option<u32> = None;
        for (index, qfam) in queuefamilyproperties.iter().enumerate() {
            if qfam.queue_count > 0 
            && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            && surface.get_physical_device_surface_support(physical_device, index)?
            {
                graphics_queue_index = Some(index as u32);
            }
            if qfam.queue_count > 0 
            && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER)
            && !graphics_queue_index.contains(&(index as u32))
            {
                println!("Found unique transfer queue");
                if transfer_queue_index.is_none() || !qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                {
                    transfer_queue_index = Some(index as u32);
                }
            }
        }

        assert!(graphics_queue_index.is_some());
        Ok(Self {
            graphics_queue_index,
            transfer_queue_index,
        })
    }

    pub(crate) fn graphics_queue_index(&self) -> Option<u32> {
        self.graphics_queue_index
    }

    pub(crate) fn transfer_queue_index(&self) -> Option<u32> {
        self.transfer_queue_index
    }
}

pub(crate) fn choose_physical_device(instance: &ash::Instance) -> Result<(vk::PhysicalDevice, vk::PhysicalDeviceProperties), vk::Result> {
    let phys_devs = unsafe { instance.enumerate_physical_devices()? };
    let chosen = {
        let mut chosen = None;
        let mut found_discrete = false;
        for p in phys_devs {
            let properties = unsafe { instance.get_physical_device_properties(p) };
            if !found_discrete {
                if properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU {
                    chosen = Some((p, properties));
                }
            }
            if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                chosen = Some((p, properties));
                found_discrete = true;
            }
        }
        chosen
    };
    Ok(chosen.unwrap())
}
