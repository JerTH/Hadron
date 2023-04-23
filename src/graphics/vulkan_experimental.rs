use std::{rc::Rc, mem::ManuallyDrop, collections::{HashMap, BTreeMap, HashSet}};
use ash::{vk::{self, QueueFlags, QueueFamilyProperties}, extensions::khr};
use serde::{Serialize, Deserialize};
use winit::window::Window;

use crate::{graphics::{vulkan_debug, vulkan_experimental::builders::{InstanceValidationLayer, VulkanLogicalDeviceBuilder}}, debug};
use super::vulkan_debug::{VulkanDebugUtils, ValidationLayersDescriptor, DebugUtilsMessageType, DebugUtilsMessageSeverity};

pub(crate) struct VulkanInstance {
    instance: ash::Instance,
    validation_layers: HashSet<InstanceValidationLayer>,
}

impl std::ops::Deref for VulkanInstance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl std::ops::DerefMut for VulkanInstance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.instance
    }
}

pub(crate) struct VulkanGraphics {
    window: Rc<winit::window::Window>,

    entry: ash::Entry,
    instance: VulkanInstance,
    debug: ManuallyDrop<VulkanDebugUtils>,
    physical: PhysicalDevice,
    logical: Option<LogicalDevice>,

    surface: Option<SurfaceImpl>,
    swapchain: Option<Swapchain>,

    scene: Option<RenderStyle>,
    ui: Option<RenderStyle>,
}

enum DebugImpl {
    None,
    Std(ManuallyDrop<VulkanDebugUtils>),
}

/// Facilitates platform dependent surface implementations
enum SurfaceImpl {
    None,
    Wayland(WaylandSurface),
}

struct WaylandSurface {
    wayland_surface_loader: khr::WaylandSurface,
    surface_loader: khr::Surface,
    surface_khr: vk::SurfaceKHR,
}

#[derive(Debug)]
pub(crate) struct PhysicalDevice {
    device: vk::PhysicalDevice,
    properties: vk::PhysicalDeviceProperties,
    queue_families: BTreeMap<QueueFamilyGroup, Vec<QueueFamilyInfo>>,
}

struct LogicalDevice {
    queues: Vec<vk::Queue>,
    family_indices: Vec<u32>,
    device: Option<ash::Device>,
    command_pools: Vec<vk::CommandPool>,
}

struct Swapchain {
    loader: khr::Swapchain,
    swapchain: vk::SwapchainKHR,

    extent: vk::Extent2D,

    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    current: usize,

    available: Vec<vk::Semaphore>,
    finished: Vec<vk::Semaphore>,
    fences: Vec<vk::Fence>,
}

/// Encapsulates a renderpass and its associated pipelines
struct RenderStyle {
    renderpass: vk::RenderPass,
    pipelines: Vec<vk::Pipeline>,
    layouts: Vec<vk::PipelineLayout>,
}

#[derive(Debug)]
pub(crate) enum VulkanResult {
    Success,
    NotReady,
    Timeout,
    EventSet,
    EventReset,
    Incomplete,
    Error(VulkanError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum VulkanError {
    /* Vulkan spec error types */
    OutOfHostMemory,
    OutOfDeviceMemory,
    InitializationFailed,
    DeviceLost,
    MemoryMapFailed,
    LayerNotPresent,
    ExtensionNotPresent,
    FeatureNotPresent,
    IncompatibleDriver,
    TooManyObjects,
    FormatNotSupported,
    FragmentedPool,
    Unknown,

    /* Implementation error types */
    NoSupportedDevice,
    MissingSurfaceImplementation,
    NoGtcSurfaceQueue,
    NotWaylandWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum QueueFamilyGroup {
    GraphicsTransferCompute,
    GraphicsTransfer,
    Graphics,
    Transfer,
    GraphicsCompute,
    TransferCompute,
    Compute,
}

#[derive(Debug, Clone)]
struct QueueFamilyInfo {
    index: usize,
    family_properties: QueueFamilyProperties,
    surface_support: bool,
}

// Impls

impl VulkanGraphics {
    pub(crate) fn new(window: Rc<winit::window::Window>) -> Result<Self, VulkanResult> {
        let entry = load_entry();

        use builders::InstanceExtension;
        let instance = builders::VulkanInstanceBuilder::new(&entry)
            .with_app_name("Test App Name")
            .with_engine_name("Test Engine Name")
            .with_extensions(&[
                InstanceExtension::ExtDebugUtils,
                InstanceExtension::KhrSurface,
                InstanceExtension::KhrWaylandSurface,
            ])
            .with_validation_layers(&[
                InstanceValidationLayer::LunarGApiDump,
                InstanceValidationLayer::KhronosValidation,
            ])
            .build()?;
            
        let debug = vulkan_debug::VulkanDebugUtilsBuilder::new(&entry, &instance)
            .with_debug_message_types(&[
                DebugUtilsMessageType::General,
                DebugUtilsMessageType::Performance,
                DebugUtilsMessageType::Validation,
            ])
            .with_debug_message_severities(&[
                DebugUtilsMessageSeverity::Info,
                DebugUtilsMessageSeverity::Warning,
                DebugUtilsMessageSeverity::Error,
                DebugUtilsMessageSeverity::Verbose,
            ])
            .with_messenger_callback(vulkan_debug::vulkan_debug_callback_println)
            .build()?;

        let surface = SurfaceImpl::Wayland(WaylandSurface::new(&entry, &instance, &window)?);
        let physical = PhysicalDevice::new(&instance, &surface)?;
        let logical = VulkanLogicalDeviceBuilder::new(&instance, &physical, &surface, instance.validation_layers.clone())
            .build()?;
        
        Ok(VulkanGraphics {
            window: window,
            entry: entry,
            instance: instance,
            debug: ManuallyDrop::new(debug),
            physical: physical,
            logical: Some(logical),
            surface: None,
            swapchain: None,
            scene: None,
            ui: None
        })
    }
}

impl Drop for VulkanGraphics {
    fn drop(&mut self) {
        todo!()
    }
}

impl Default for DebugImpl {
    fn default() -> Self {
        DebugImpl::None
    }
}

impl Default for SurfaceImpl {
    fn default() -> Self {
        SurfaceImpl::None
    }
}

impl WaylandSurface {
    fn new(entry: &ash::Entry, instance: &ash::Instance, window: &winit::window::Window) -> Result<Self, VulkanResult> {
        use winit::platform::unix::WindowExtUnix;

        let wayland_display = window.wayland_display().ok_or(VulkanResult::Error(VulkanError::NotWaylandWindow))?;
        let wayland_surface = window.wayland_surface().ok_or(VulkanResult::Error(VulkanError::NotWaylandWindow))?;
        let wayland_create_info = vk::WaylandSurfaceCreateInfoKHR::builder()
            .display(wayland_display)
            .surface(wayland_surface);
        let wayland_surface_loader = ash::extensions::khr::WaylandSurface::new(entry, instance);
        let surface_loader = ash::extensions::khr::Surface::new(entry, instance);
        let surface = unsafe { wayland_surface_loader.create_wayland_surface(&wayland_create_info, None)? };

        Ok( WaylandSurface {
            wayland_surface_loader: wayland_surface_loader,
            surface_loader: surface_loader,
            surface_khr: surface,
        })
    }
}

impl PhysicalDevice {
    fn new(instance: &ash::Instance, surface: &SurfaceImpl) -> Result<Self, VulkanResult> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        let mut integrated = Vec::new();
        let mut discrete = Vec::new();
        let mut cpu = Vec::new();
        let mut virtual_gpu = Vec::new();
        let mut other = Vec::new();
        
        for device in physical_devices {
            let device_properties = unsafe { instance.get_physical_device_properties(device) };
            match device_properties.device_type {
                vk::PhysicalDeviceType::INTEGRATED_GPU => integrated.push((device, device_properties)),
                vk::PhysicalDeviceType::DISCRETE_GPU => discrete.push((device, device_properties)),
                vk::PhysicalDeviceType::CPU => cpu.push((device, device_properties)),
                vk::PhysicalDeviceType::VIRTUAL_GPU => virtual_gpu.push((device, device_properties)),
                vk::PhysicalDeviceType::OTHER => other.push((device, device_properties)),
                _ => unreachable!()
            }
        }
        
        let (physical_device, physical_device_properties) = if let Some(discrete) = discrete.first() {
            (discrete.0, discrete.1)
        } else if let Some(integrated) = integrated.first() {
            (integrated.0, integrated.1)
        } else {
            let unsupported_devices: Vec<PhysicalDevice> = integrated.iter()
                .chain(discrete.iter())
                .chain(cpu.iter())
                .chain(virtual_gpu.iter())
                .chain(other.iter())
                .map(|d| PhysicalDevice { device: d.0, properties: d.1, queue_families: BTreeMap::new() })
                .collect();
            return Err(VulkanResult::Error(VulkanError::NoSupportedDevice));
        };

        // We've chosen a device, get some info about its available queue families
        let queue_family_properties = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
        };

        let mut queue_family_map: BTreeMap<QueueFamilyGroup, Vec<QueueFamilyInfo>> = BTreeMap::new();
        for (index, family) in queue_family_properties.iter().enumerate() {
            let surface_support = match surface {
                SurfaceImpl::None => return Err(VulkanResult::Error(VulkanError::MissingSurfaceImplementation)),
                SurfaceImpl::Wayland(wayland_surface) => unsafe {
                    wayland_surface.surface_loader.get_physical_device_surface_support(physical_device, index as u32, wayland_surface.surface_khr)?
                },
            };
            
            let queue_family_group = QueueFamilyGroup::from(family);
            let queue_family_info = QueueFamilyInfo {
                index: index,
                family_properties: *family,
                surface_support: surface_support,
            };

            queue_family_map
                .entry(queue_family_group)
                .and_modify(|v| v.push(queue_family_info.clone()))
                .or_insert(vec![queue_family_info.clone()]);
        }

        debug_assert!(!queue_family_map.is_empty(), "empty queue family map");

        Ok(PhysicalDevice {
            device: physical_device,
            properties: physical_device_properties,
            queue_families: queue_family_map,
        })
    }
}

impl LogicalDevice {
    fn new() -> Self {
        LogicalDevice {
            queues: Vec::new(),
            family_indices: Vec::new(),
            device: None,
            command_pools: Vec::new(),
        }
    }
}

impl Swapchain {

}

impl RenderStyle {
    
}

impl From<vk::Result> for VulkanResult {
    fn from(result: vk::Result) -> Self {
        match result {
            vk::Result::SUCCESS => VulkanResult::Success,
            vk::Result::NOT_READY => VulkanResult::NotReady,
            vk::Result::TIMEOUT => VulkanResult::Timeout,
            vk::Result::EVENT_SET => VulkanResult::EventSet,
            vk::Result::EVENT_RESET => VulkanResult::EventReset,
            vk::Result::INCOMPLETE => VulkanResult::Incomplete,
            vk::Result::ERROR_OUT_OF_HOST_MEMORY => VulkanResult::Error(VulkanError::OutOfHostMemory),
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => VulkanResult::Error(VulkanError::OutOfDeviceMemory),
            vk::Result::ERROR_INITIALIZATION_FAILED => VulkanResult::Error(VulkanError::InitializationFailed),
            vk::Result::ERROR_DEVICE_LOST => VulkanResult::Error(VulkanError::DeviceLost),
            vk::Result::ERROR_MEMORY_MAP_FAILED => VulkanResult::Error(VulkanError::MemoryMapFailed),
            vk::Result::ERROR_LAYER_NOT_PRESENT => VulkanResult::Error(VulkanError::LayerNotPresent),
            vk::Result::ERROR_EXTENSION_NOT_PRESENT => VulkanResult::Error(VulkanError::ExtensionNotPresent),
            vk::Result::ERROR_FEATURE_NOT_PRESENT => VulkanResult::Error(VulkanError::FeatureNotPresent),
            vk::Result::ERROR_INCOMPATIBLE_DRIVER => VulkanResult::Error(VulkanError::IncompatibleDriver),
            vk::Result::ERROR_TOO_MANY_OBJECTS => VulkanResult::Error(VulkanError::TooManyObjects),
            vk::Result::ERROR_FORMAT_NOT_SUPPORTED => VulkanResult::Error(VulkanError::FormatNotSupported),
            vk::Result::ERROR_FRAGMENTED_POOL => VulkanResult::Error(VulkanError::FragmentedPool),
            vk::Result::ERROR_UNKNOWN => VulkanResult::Error(VulkanError::Unknown),
            _ => todo!()
        }
    }
}

impl std::error::Error for VulkanError {}

impl std::fmt::Display for VulkanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VulkanError::OutOfHostMemory => write!(f, "out of host memory"),
            VulkanError::OutOfDeviceMemory => write!(f, "out of device memory"),
            VulkanError::InitializationFailed => write!(f, "initialization failed"),
            VulkanError::DeviceLost => write!(f, "device lost"),
            VulkanError::MemoryMapFailed => write!(f, "memory map failed"),
            VulkanError::LayerNotPresent => write!(f, "layer not present"),
            VulkanError::ExtensionNotPresent => write!(f, "extension not present"),
            VulkanError::FeatureNotPresent => write!(f, "feature not present"),
            VulkanError::IncompatibleDriver => write!(f, "incompatible driver"),
            VulkanError::TooManyObjects => write!(f, "too many objects"),
            VulkanError::FormatNotSupported => write!(f, "format not supported"),
            VulkanError::FragmentedPool => write!(f, "fragmented pool"),
            VulkanError::Unknown => write!(f, "unknown"),
            VulkanError::NoSupportedDevice => write!(f, "no supported device"),
            VulkanError::MissingSurfaceImplementation => write!(f, "missing surface implementation"),
            VulkanError::NoGtcSurfaceQueue => write!(f, "no surface supporting gtc queue"),
            VulkanError::NotWaylandWindow => write!(f, "expected a wayland window"),
        }
    }
}

impl From<&vk::QueueFamilyProperties> for QueueFamilyGroup {
    fn from(props: &vk::QueueFamilyProperties) -> Self {
        match props.queue_flags {
            f if f == (QueueFlags::GRAPHICS | QueueFlags::TRANSFER | QueueFlags::COMPUTE) => QueueFamilyGroup::GraphicsTransferCompute,
            f if f == (QueueFlags::GRAPHICS | QueueFlags::TRANSFER) => QueueFamilyGroup::GraphicsTransfer,
            f if f == (QueueFlags::GRAPHICS | QueueFlags::COMPUTE) => QueueFamilyGroup::GraphicsCompute,
            f if f == (QueueFlags::TRANSFER | QueueFlags::COMPUTE) => QueueFamilyGroup::TransferCompute,
            f if f == (QueueFlags::GRAPHICS) => QueueFamilyGroup::Graphics,
            f if f == (QueueFlags::TRANSFER) => QueueFamilyGroup::Transfer,
            f if f == (QueueFlags::COMPUTE) => QueueFamilyGroup::Compute,
            _ => { unreachable!() }
        }
    }
}

/// Builders
mod builders {
    use std::{collections::{HashSet, VecDeque}, ffi::CString, hash::Hash};
    use ash::vk;
    use serde::{Serialize, Deserialize};
    use crate::debug::log;

    use super::{VulkanResult, LogicalDevice, PhysicalDevice, QueueFamilyGroup, SurfaceImpl, VulkanError, QueueFamilyInfo, VulkanInstance};

    #[derive(Default)]
    pub(super) struct VulkanInstanceBuilder<'a> {
        entry: Option<&'a ash::Entry>,
        app_name: Option<CString>,
        app_version: Option<u32>,
        engine_name: Option<CString>,
        engine_version: Option<u32>,
        api_version: Option<u32>,
        validation_layers: HashSet<InstanceValidationLayer>,
        extensions: HashSet<InstanceExtension>,
        log: log::Logger,
    }
    
    pub(super) struct VulkanLogicalDeviceBuilder<'a> {
        instance: &'a ash::Instance,
        physical: &'a PhysicalDevice,
        surface: &'a SurfaceImpl,
        validation_layers: HashSet<InstanceValidationLayer>,
        log: log::Logger,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(super) enum InstanceExtension {
        ExtDebugUtils,
        KhrSurface,
        KhrWaylandSurface,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(super) enum InstanceValidationLayer {
        KhronosValidation,
        LunarGApiDump,
    }

    impl InstanceValidationLayer {
        fn layer_name_pointer(&self) -> *const i8 {
            const KHRONOS_VALIDATION_STRING: *const i8 = "VK_LAYER_KHRONOS_validation\0".as_ptr() as *const i8;
            const LUNARG_API_DUMP_STRING: *const i8 = "VK_LAYER_LUNARG_api_dump\0".as_ptr() as *const i8;
            match self {
                InstanceValidationLayer::KhronosValidation => KHRONOS_VALIDATION_STRING,
                InstanceValidationLayer::LunarGApiDump => LUNARG_API_DUMP_STRING,
            }
        }
    }

    impl<'a> VulkanInstanceBuilder<'a> {
        pub(super) fn new(entry: &'a ash::Entry) -> Self {
            VulkanInstanceBuilder {
                entry: Some(entry),
                ..Default::default()
            }
        }

        pub(super) fn with_app_name(mut self, name: &str) -> Self {
            self.log.info(format!("setting vulkan instance app name: {}", name));
            self.app_name = Some(CString::new(name).unwrap()); self
        }

        pub(super) fn with_engine_name(mut self, name: &str) -> Self {
            self.log.info(format!("setting vulkan instance engine name: {}", name));
            self.engine_name = Some(CString::new(name).unwrap()); self
        }

        pub(super) fn with_extensions(mut self, extensions: &[InstanceExtension]) -> Self {
            self.log.info(format!("enabling vulkan instance extensions: {:?}", extensions));
            for extension in extensions {
                debug_assert!(self.extensions.insert(*extension), "duplicate extension");
            }
            self
        }

        pub(super) fn with_validation_layers(mut self, validation_layers: &[InstanceValidationLayer]) -> Self {
            self.log.info(format!("enabling vulkan instance validation layers: {:?}", validation_layers));
            for layer in validation_layers {
                debug_assert!(self.validation_layers.insert(*layer), "duplicate validation layer");
            }
            self
        }

        pub(super) fn build(self) -> Result<VulkanInstance, VulkanResult> {
            self.log.info(format!("building vulkan instance"));
            
            let mut instance_create_info = vk::InstanceCreateInfo::builder();

            let app_name = self.app_name.unwrap_or(CString::new("Default App").unwrap());
            let engine_name = self.engine_name.unwrap_or(CString::new("Default Engine").unwrap());

            let app_info = vk::ApplicationInfo::builder()
                .application_name(&app_name)
                .application_version(self.app_version.unwrap_or(vk::make_api_version(0, 0, 0, 0)))
                .engine_name(&engine_name)
                .engine_version(self.engine_version.unwrap_or(vk::make_api_version(0, 0, 0, 0)))
                .api_version(self.api_version.unwrap_or(vk::make_api_version(0, 1, 2, 0)));

            instance_create_info = instance_create_info.application_info(&app_info);

            let mut extension_name_pointers = Vec::new();
            if !self.extensions.is_empty() {
                for extension in self.extensions {
                    let pointer = match extension {
                        InstanceExtension::ExtDebugUtils => ash::extensions::ext::DebugUtils::name().as_ptr(),
                        InstanceExtension::KhrSurface => ash::extensions::khr::Surface::name().as_ptr(),
                        InstanceExtension::KhrWaylandSurface => ash::extensions::khr::WaylandSurface::name().as_ptr(),
                    };
                    extension_name_pointers.push(pointer);
                }
                self.log.info(format!("enabled instance extensions: {:?}", &extension_name_pointers));
                instance_create_info = instance_create_info.enabled_extension_names(&extension_name_pointers);
            }
            
            let validation_layer_name_pointers: Vec<*const i8> = self.validation_layers.iter().map(|l| l.layer_name_pointer()).collect();

            instance_create_info = instance_create_info.enabled_layer_names(&validation_layer_name_pointers);

            println!("creating instance now");
            let instance = unsafe { self.entry.unwrap().create_instance(&instance_create_info, None)? }; 
            println!("created");

            let instance = Ok(VulkanInstance {
                instance: instance,
                validation_layers: self.validation_layers.clone()
            });

            instance
        }
    }

    impl<'a> VulkanLogicalDeviceBuilder<'a> {
        pub(super) fn new(instance: &'a ash::Instance, physical: &'a PhysicalDevice, surface: &'a SurfaceImpl, validation: HashSet<InstanceValidationLayer>) -> Self {
            VulkanLogicalDeviceBuilder {
                instance: instance,
                physical: physical,
                surface: surface,
                validation_layers: validation,
                log: crate::debug::log::get(),
            }
        }
        
        pub(super) fn build(mut self) -> Result<LogicalDevice, VulkanResult> {
            self.log.info(format!("building vulkan logical device"));

            let mut gtc_surface_support_queues = VecDeque::new();
            let mut transfer_only_queues = VecDeque::new();
            let mut compute_only_queues = VecDeque::new();

            dbg!(&self.physical.queue_families);

            // Queues which support graphics transfer and compute and also support our surface, primary queue candidates
            if let Some(queue_fams) = self.physical.queue_families.get(&QueueFamilyGroup::GraphicsTransferCompute) {
                dbg!(&queue_fams);
                for queue_fam_info in queue_fams {
                    dbg!(queue_fam_info.surface_support);
                    if queue_fam_info.surface_support {
                        gtc_surface_support_queues.push_back(queue_fam_info);
                    }
                }
            };
            
            self.log.info(&format!("found {} gtc surface support queues", gtc_surface_support_queues.len()));

            // Transfer only queues
            if let Some(queue_fams) = self.physical.queue_families.get(&QueueFamilyGroup::Transfer) {
                for queue_fam_info in queue_fams {
                    transfer_only_queues.push_back(queue_fam_info);
                }
            }

            self.log.info(&format!("found {} transfer only queues", transfer_only_queues.len()));

            // Compute only queues
            if let Some(queue_fams) = self.physical.queue_families.get(&QueueFamilyGroup::Compute) {
                for queue_fam_info in queue_fams {
                    compute_only_queues.push_back(queue_fam_info);
                }
            }
            
            self.log.info(&format!("found {} compute only queues", compute_only_queues.len()));

            let queue_priorities = [1.0f32];
            let mut primary_queue_info: Option<&QueueFamilyInfo> = None;
            let mut transfer_queue_info: Option<&QueueFamilyInfo> = None;
            let mut queue_create_infos = Vec::new();

            if let Some(queue_family_info) = gtc_surface_support_queues.pop_front() {
                let queue_create_info = Self::make_queue_create_info(queue_family_info, &queue_priorities);
                primary_queue_info = Some(queue_family_info);
                queue_create_infos.push(queue_create_info);
            } else {
                return Err(VulkanResult::Error(VulkanError::NoGtcSurfaceQueue));
            }

            if let Some(queue_family_info) = transfer_only_queues.pop_front() {
                let queue_create_info = Self::make_queue_create_info(queue_family_info, &queue_priorities);
                transfer_queue_info = Some(queue_family_info);
                queue_create_infos.push(queue_create_info);
            } else {
                self.log.warn("no available transfer only queues");
            }
            
            let device_extension_name_pointers: Vec<*const i8> = vec![ash::extensions::khr::Swapchain::name().as_ptr()];
            let validation_layer_name_pointers: Vec<*const i8> = self.validation_layers.iter().map(|l| l.layer_name_pointer()).collect();
            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&device_extension_name_pointers)
                .enabled_layer_names(&validation_layer_name_pointers);

            let logical_device = unsafe {
                self.instance.create_device(self.physical.device, &device_create_info, None)?
            };
            
            // queues

            Ok(LogicalDevice {
                queues: todo!(),
                family_indices: todo!(),
                device: Some(logical_device),
                command_pools: todo!(),
            })
        } 

        fn make_queue_create_info(info: &QueueFamilyInfo, priorities: &[f32]) -> vk::DeviceQueueCreateInfo {
            let queue_family_index = info.index as u32;
            let queue_create_info = vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(priorities)
                    .build();
            queue_create_info
        }
    }

}



// Fn
fn load_entry() -> ash::Entry {
    unsafe {
        ash::Entry::load().expect("unable to load vulkan entry point")
    }
}

#[deprecated]
#[allow(unused)]
fn make_validation_layer_descriptor() -> ValidationLayersDescriptor {
    ValidationLayersDescriptor::new()
}
