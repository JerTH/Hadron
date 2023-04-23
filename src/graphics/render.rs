use ash::vk;
use crate::graphics::{ surface, vulkangfx::GraphicsDevice };

pub(crate) fn init_renderpass(graphics_device: &GraphicsDevice, physical_device: vk::PhysicalDevice, surfaces: &surface::GraphicsSurface) -> Result<vk::RenderPass, vk::Result> {
    let attachments = [vk::AttachmentDescription::builder()
        .format(
            surfaces
                .get_formats(physical_device)?
                .first()
                .unwrap()
                .format,
        )
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .samples(vk::SampleCountFlags::TYPE_1)
        .build()];
    
    let color_attachment_references = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    
    let subpasses = [vk::SubpassDescription::builder()
        .color_attachments(&color_attachment_references)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).build()];
    
    let subpass_dependencies = [vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_subpass(0)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        )
        .build()];
    
    let renderpass_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&subpass_dependencies);
    
    let renderpass = unsafe { graphics_device.create_render_pass(&renderpass_info)? };
    Ok(renderpass)
}

pub(crate) struct Pipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Pipeline {
    pub(crate) fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_pipeline(self.pipeline, None);
            logical_device.destroy_pipeline_layout(self.layout, None);
        }
    }

    pub(crate) fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
    
    pub(crate) fn init(graphics_device: &GraphicsDevice, swapchain: &surface::Swapchain, renderpass: &vk::RenderPass) -> Result<Self, vk::Result> {
        let vertexshader_createinfo = vk::ShaderModuleCreateInfo::builder()
            .code(vk_shader_macros::include_glsl!("src/graphics/shader.vert", kind: vert));
        let vertexshader_module = graphics_device.create_shader_module(&vertexshader_createinfo)?;
        
        let fragmentshader_createinfo = vk::ShaderModuleCreateInfo::builder()
            .code(vk_shader_macros::include_glsl!("src/graphics/shader.frag"));
        let fragmentshader_module = graphics_device.create_shader_module(&fragmentshader_createinfo)?;

        let mainfunctionname = std::ffi::CString::new("main").unwrap();
        let vertexshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertexshader_module)
            .name(&mainfunctionname);
        
        let fragmentshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragmentshader_module)
            .name(&mainfunctionname);
        
        let shader_stages = vec![vertexshader_stage.build(), fragmentshader_stage.build()];

// *** cont allocators
        let vertex_attrib_descs = [vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            offset: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
        }];

        let vertex_binding_descs = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: 16,
            input_rate: vk::VertexInputRate::VERTEX,
        }];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_attrib_descs)
            .vertex_binding_descriptions(&vertex_binding_descs);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::POINT_LIST);

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: swapchain.extent().width as f32,
            height: swapchain.extent().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent(),
        }];
        
        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE)
            .polygon_mode(vk::PolygonMode::FILL);

        let multisampler_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let colourblend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .build()];

        let colourblend_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&colourblend_attachments);

        let pipelinelayout_info = vk::PipelineLayoutCreateInfo::builder();
        let pipelinelayout = graphics_device.create_pipeline_layout(&pipelinelayout_info)?;
            
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampler_info)
            .color_blend_state(&colourblend_info)
            .layout(pipelinelayout)
            .render_pass(*renderpass)
            .subpass(0);
        
        let graphicspipeline = graphics_device.create_graphics_pipelines(&[pipeline_info.build()])[0];
        
        unsafe {
            graphics_device.destroy_shader_module(fragmentshader_module);
            graphics_device.destroy_shader_module(vertexshader_module);
        }

        Ok(Pipeline {
            pipeline: graphicspipeline,
            layout: pipelinelayout,
        })
    }
}
