use std::{rc::Rc, collections::HashSet, hash::Hash};
use ash::vk;

use super::vulkan_experimental::VulkanInstance;

pub struct VulkanDebugUtils {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl Drop for VulkanDebugUtils {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_debug_utils_messenger(self.messenger, None)
        };
    }
}

type VulkanDebugUtilsMessengerCallbackType = unsafe extern "system" fn(vk::DebugUtilsMessageSeverityFlagsEXT, vk::DebugUtilsMessageTypeFlagsEXT, *const vk::DebugUtilsMessengerCallbackDataEXT, *mut std::ffi::c_void) -> u32;
pub(super) struct VulkanDebugUtilsBuilder<'a> {
    entry: &'a ash::Entry,
    instance: &'a VulkanInstance,
    debug_message_types: HashSet<DebugUtilsMessageType>,
    debug_message_severities: HashSet<DebugUtilsMessageSeverity>,
    messenger_callback: Option<VulkanDebugUtilsMessengerCallbackType>,
}

impl<'a> VulkanDebugUtilsBuilder<'a> {
    pub(super) fn new(entry: &'a ash::Entry, instance: &'a VulkanInstance) -> Self {
        VulkanDebugUtilsBuilder {
            entry,
            instance,
            debug_message_types: HashSet::new(),
            debug_message_severities: HashSet::new(),
            messenger_callback: None,
        }
    }

    pub(super) fn with_debug_message_types(mut self, flags: &[DebugUtilsMessageType]) -> Self {
        flags.iter().for_each(|f| debug_assert!(self.debug_message_types.insert(*f)));
        self
    }
    
    pub(super) fn with_debug_message_severities(mut self, flags: &[DebugUtilsMessageSeverity]) -> Self {
        flags.iter().for_each(|f| debug_assert!(self.debug_message_severities.insert(*f)));
        self
    }

    pub(super) fn with_messenger_callback(mut self, callback: VulkanDebugUtilsMessengerCallbackType) -> Self {
        self.messenger_callback = Some(callback);
        self
    }

    pub(super) fn build(self) -> Result<VulkanDebugUtils, vk::Result> {
        let mut create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder();

        if !self.debug_message_types.is_empty() {
            let mut message_type_flags: vk::DebugUtilsMessageTypeFlagsEXT = vk::DebugUtilsMessageTypeFlagsEXT::empty();
            for flag in self.debug_message_types {
                match flag {
                    DebugUtilsMessageType::General => message_type_flags |= vk::DebugUtilsMessageTypeFlagsEXT::GENERAL,
                    DebugUtilsMessageType::Validation => message_type_flags |= vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                    DebugUtilsMessageType::Performance => message_type_flags |= vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                }
            }
            create_info = create_info.message_type(message_type_flags);

            if !self.debug_message_severities.is_empty() {
                let mut severity_flags: vk::DebugUtilsMessageSeverityFlagsEXT = vk::DebugUtilsMessageSeverityFlagsEXT::empty();
                for flag in self.debug_message_severities {
                    match flag {
                        DebugUtilsMessageSeverity::Verbose => severity_flags |= vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
                        DebugUtilsMessageSeverity::Info => severity_flags |= vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                        DebugUtilsMessageSeverity::Warning => severity_flags |= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
                        DebugUtilsMessageSeverity::Error => severity_flags |= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                    }
                }
                create_info = create_info.message_severity(severity_flags);
            }
        }
        create_info = create_info.pfn_user_callback(self.messenger_callback);
        
        let loader = ash::extensions::ext::DebugUtils::new(self.entry, self.instance);
        let messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None)? };
        
        Ok(VulkanDebugUtils {
            loader,
            messenger
        })

    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum DebugUtilsMessageType {
    General,
    Validation,
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum DebugUtilsMessageSeverity {
    Verbose,
    Info,
    Warning,
    Error,
}

/// Debug Callback
pub unsafe extern "system" fn vulkan_debug_callback_println(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);
    vk::FALSE
}

#[derive(Debug)]
pub struct ValidationLayersDescriptor {
    layer_names: Rc<Vec<std::ffi::CString>>,
    layer_name_pointers: Rc<Vec<*const i8>>,
}

impl ValidationLayersDescriptor {
    pub fn new() -> Self {
        let layer_names = Rc::new(vec![
                std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
                std::ffi::CString::new("VK_LAYER_LUNARG_api_dump").unwrap(),    
            ]);

        let layer_name_pointers = Rc::new(layer_names.iter().map(|layer_name| layer_name.as_ptr()).collect());
        
        Self {
            layer_names,
            layer_name_pointers,
        }
    }

    pub fn layer_name_pointers(&self) -> &Vec<*const i8> {
        &self.layer_name_pointers
    }
}
