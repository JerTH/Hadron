use std::rc::Rc;
use ash::vk::{ self, DebugUtilsMessengerCreateInfoEXTBuilder };

/// Debug Callback
pub unsafe extern "system" fn vulkan_debug_utils_callback(
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

/// Debug Utils
pub fn init_debug_info<'a>() -> Result<DebugUtilsMessengerCreateInfoEXTBuilder<'a>, vk::Result> {
    let dci = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                //| vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback));
    Ok(dci)
}

pub struct VulkanDebugWidget {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanDebugWidget {
    pub fn init(entry: &ash::Entry, instance: &ash::Instance) -> Result<VulkanDebugWidget, vk::Result> {
        let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(vulkan_debug_utils_callback));
            
        let loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let messenger = unsafe { loader.create_debug_utils_messenger(&debugcreateinfo, None)? };
            
        Ok(VulkanDebugWidget { loader, messenger })
    }
}

impl Drop for VulkanDebugWidget {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_debug_utils_messenger(self.messenger, None)
        };
    }
}

pub struct ValidationLayers {
    layer_names: Rc<Vec<std::ffi::CString>>,
    layer_name_pointers: Rc<Vec<*const i8>>,
}

impl ValidationLayers {
    pub fn init() -> Result<Self, vk::Result> {
        let layer_names = Rc::new(vec![
                std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
                //std::ffi::CString::new("VK_LAYER_LUNARG_api_dump").unwrap(),    
            ]);

        let layer_name_pointers = Rc::new(layer_names.iter().map(|layer_name| layer_name.as_ptr()).collect());
        
        Ok(Self {
            layer_names,
            layer_name_pointers,
        })
    }

    pub fn layer_name_pointers(&self) -> &Vec<*const i8> {
        &self.layer_name_pointers
    }
}
