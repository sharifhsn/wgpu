use alloc::vec::Vec;
use core::mem;
use core::ops::Range;

#[cfg(target_os = "horizon")]
use core::ptr;
#[cfg(target_os = "horizon")]
use deko3d_sys as dk;

#[cfg(target_os = "horizon")]
use super::ShaderBindingKind;

use super::{
    Api, BindGroupInner, Buffer, DeviceResult, Queue, RawImage, RawQueueHandle,
    RenderPipelineInner, Resource, TextureInner,
};

const DEKO_INVALIDATE_IMAGE: u32 = 1 << 0;
const DEKO_INVALIDATE_SHADER: u32 = 1 << 1;
const DEKO_INVALIDATE_DESCRIPTORS: u32 = 1 << 2;
const DEKO_INVALIDATE_L2_CACHE: u32 = 1 << 4;
const DEKO_RESOURCE_TRANSITION_INVALIDATE_FLAGS: u32 = DEKO_INVALIDATE_IMAGE
    | DEKO_INVALIDATE_SHADER
    | DEKO_INVALIDATE_DESCRIPTORS
    | DEKO_INVALIDATE_L2_CACHE;

/// Command buffer type, which performs double duty as the command encoder type too.
#[derive(Debug)]
pub struct CommandBuffer {
    commands: Vec<Command>,
    error: Option<crate::DeviceError>,
}

#[derive(Debug)]
enum Command {
    ClearBuffer {
        buffer: Buffer,
        range: crate::MemoryRange,
    },
    CopyBufferToBuffer {
        src: Buffer,
        dst: Buffer,
        regions: Vec<crate::BufferCopy>,
    },
    CopyBufferToTexture {
        src: Buffer,
        dst: Option<alloc::sync::Arc<TextureInner>>,
        regions: Vec<crate::BufferTextureCopy>,
    },
    CopyTextureToTexture {
        src: Option<alloc::sync::Arc<TextureInner>>,
        dst: Option<alloc::sync::Arc<TextureInner>>,
        regions: Vec<crate::TextureCopy>,
    },
    CopyTextureToBuffer {
        src: Option<alloc::sync::Arc<TextureInner>>,
        dst: Buffer,
        regions: Vec<crate::BufferTextureCopy>,
    },
    ResourceBarrier {
        invalidate_flags: u32,
    },
    BeginRenderPass {
        images: Vec<Option<RawImage>>,
        extent: wgt::Extent3d,
        clear_values: Vec<Option<wgt::Color>>,
        depth: Option<RawImage>,
        depth_clear_value: Option<f32>,
    },
    SetRenderPipeline {
        pipeline: alloc::sync::Arc<RenderPipelineInner>,
    },
    SetVertexBuffer {
        index: u32,
        buffer: Buffer,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
    },
    SetIndexBuffer {
        buffer: Buffer,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
        format: wgt::IndexFormat,
    },
    SetBindGroup {
        index: u32,
        group: alloc::sync::Arc<BindGroupInner>,
        dynamic_offsets: Vec<wgt::DynamicOffset>,
    },
    SetViewport {
        rect: crate::Rect<f32>,
        depth_range: Range<f32>,
    },
    SetScissor {
        rect: crate::Rect<u32>,
    },
    SetStencilReference {
        reference: u8,
    },
    SetBlendConstants {
        color: [f32; 4],
    },
    Draw {
        first_vertex: u32,
        vertex_count: u32,
        first_instance: u32,
        instance_count: u32,
    },
    DrawIndexed {
        first_index: u32,
        index_count: u32,
        base_vertex: i32,
        first_instance: u32,
        instance_count: u32,
    },
}

#[derive(Default)]
struct ExecutionState {
    target: Option<RenderTarget>,
    pipeline: Option<alloc::sync::Arc<RenderPipelineInner>>,
    vertex_buffers: Vec<Option<VertexBinding>>,
    index_buffer: Option<IndexBinding>,
    bind_groups: Vec<Option<BoundBindGroup>>,
    viewport: Option<(crate::Rect<f32>, Range<f32>)>,
    scissor: Option<crate::Rect<u32>>,
    stencil_reference: u8,
    blend_constants: [f32; 4],
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct RenderTarget {
    images: Vec<Option<RawImage>>,
    extent: wgt::Extent3d,
    depth: Option<RawImage>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct VertexBinding {
    buffer: Buffer,
    offset: wgt::BufferAddress,
    size: Option<wgt::BufferSize>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct IndexBinding {
    buffer: Buffer,
    offset: wgt::BufferAddress,
    size: Option<wgt::BufferSize>,
    format: wgt::IndexFormat,
}

#[derive(Clone, Debug)]
struct BoundBindGroup {
    group: alloc::sync::Arc<BindGroupInner>,
    dynamic_offsets: Vec<wgt::DynamicOffset>,
}

impl CommandBuffer {
    /// # Safety
    ///
    /// Must be called with appropriate synchronization for the resources affected by the command,
    /// such as ensuring that buffers are not accessed by a command while aliasing references exist.
    pub(crate) unsafe fn execute(
        &self,
        queue: &Queue,
        surface_queue: Option<RawQueueHandle>,
    ) -> DeviceResult<()> {
        if let Some(error) = &self.error {
            #[cfg(target_os = "horizon")]
            super::trace::dump("command_buffer_error");
            return Err(error.clone());
        }
        let mut state = ExecutionState::default();
        for command in &self.commands {
            if let Err(error) = unsafe { command.execute(queue, surface_queue, &mut state) } {
                eprintln!(
                    "[wgpu-deko3d] command_execute_failed command={} error={error:?}",
                    match command {
                        Command::ClearBuffer { .. } => "clear_buffer",
                        Command::CopyBufferToBuffer { .. } => "copy_buffer_to_buffer",
                        Command::CopyBufferToTexture { .. } => "copy_buffer_to_texture",
                        Command::CopyTextureToTexture { .. } => "copy_texture_to_texture",
                        Command::CopyTextureToBuffer { .. } => "copy_texture_to_buffer",
                        Command::ResourceBarrier { .. } => "resource_barrier",
                        Command::BeginRenderPass { .. } => "begin_render_pass",
                        Command::SetRenderPipeline { .. } => "set_render_pipeline",
                        Command::SetVertexBuffer { .. } => "set_vertex_buffer",
                        Command::SetIndexBuffer { .. } => "set_index_buffer",
                        Command::SetBindGroup { .. } => "set_bind_group",
                        Command::SetViewport { .. } => "set_viewport",
                        Command::SetScissor { .. } => "set_scissor",
                        Command::SetStencilReference { .. } => "set_stencil_reference",
                        Command::SetBlendConstants { .. } => "set_blend_constants",
                        Command::Draw { .. } => "draw",
                        Command::DrawIndexed { .. } => "draw_indexed",
                    }
                );
                #[cfg(target_os = "horizon")]
                super::trace::dump("command_execute_failed");
                return Err(error);
            }
        }
        Ok(())
    }

    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
            error: None,
        }
    }

    #[track_caller]
    fn record_unsupported(&mut self) {
        #[cfg(target_os = "horizon")]
        {
            super::trace::record(format_args!(
                "failure kind=unsupported_command caller={}",
                core::panic::Location::caller()
            ));
            super::trace::dump("unsupported_command");
        }
        self.error.get_or_insert(crate::DeviceError::Lost);
    }
}

impl crate::CommandEncoder for CommandBuffer {
    type A = Api;

    unsafe fn begin_encoding(&mut self, label: crate::Label) -> DeviceResult<()> {
        assert!(self.commands.is_empty());
        assert!(self.error.is_none());
        Ok(())
    }
    unsafe fn discard_encoding(&mut self) {
        self.commands.clear();
        self.error = None;
    }
    unsafe fn end_encoding(&mut self) -> DeviceResult<CommandBuffer> {
        if let Some(error) = self.error.take() {
            self.commands.clear();
            return Err(error);
        }
        Ok(CommandBuffer {
            commands: mem::take(&mut self.commands),
            error: None,
        })
    }
    unsafe fn reset_all<I>(&mut self, command_buffers: I) {}

    unsafe fn transition_buffers<'a, T>(&mut self, barriers: T)
    where
        T: Iterator<Item = crate::BufferBarrier<'a, Buffer>>,
    {
        if barriers
            .into_iter()
            .any(|barrier| barrier.usage.from != barrier.usage.to)
        {
            self.commands.push(Command::ResourceBarrier {
                invalidate_flags: DEKO_RESOURCE_TRANSITION_INVALIDATE_FLAGS,
            });
        }
    }

    unsafe fn transition_textures<'a, T>(&mut self, barriers: T)
    where
        T: Iterator<Item = crate::TextureBarrier<'a, Resource>>,
    {
        if barriers
            .into_iter()
            .any(|barrier| barrier.usage.from != barrier.usage.to)
        {
            self.commands.push(Command::ResourceBarrier {
                invalidate_flags: DEKO_RESOURCE_TRANSITION_INVALIDATE_FLAGS,
            });
        }
    }

    unsafe fn clear_buffer(&mut self, buffer: &Buffer, range: crate::MemoryRange) {
        self.commands.push(Command::ClearBuffer {
            buffer: buffer.clone(),
            range,
        })
    }

    unsafe fn copy_buffer_to_buffer<T>(&mut self, src: &Buffer, dst: &Buffer, regions: T)
    where
        T: Iterator<Item = crate::BufferCopy>,
    {
        self.commands.push(Command::CopyBufferToBuffer {
            src: src.clone(),
            dst: dst.clone(),
            regions: regions.collect(),
        });
    }

    #[cfg(webgl)]
    unsafe fn copy_external_image_to_texture<T>(
        &mut self,
        src: &wgt::CopyExternalImageSourceInfo,
        dst: &Resource,
        dst_premultiplication: bool,
        regions: T,
    ) where
        T: Iterator<Item = crate::TextureCopy>,
    {
        self.record_unsupported();
    }

    unsafe fn copy_texture_to_texture<T>(
        &mut self,
        src: &Resource,
        _src_usage: wgt::TextureUses,
        dst: &Resource,
        regions: T,
    ) where
        T: Iterator<Item = crate::TextureCopy>,
    {
        let src = match src {
            Resource::Texture(texture) => Some(texture.clone()),
            _ => None,
        };
        let dst = match dst {
            Resource::Texture(texture) => Some(texture.clone()),
            _ => None,
        };
        self.commands.push(Command::CopyTextureToTexture {
            src,
            dst,
            regions: regions.collect(),
        });
    }

    unsafe fn copy_buffer_to_texture<T>(&mut self, src: &Buffer, dst: &Resource, regions: T)
    where
        T: Iterator<Item = crate::BufferTextureCopy>,
    {
        let dst = match dst {
            Resource::Texture(texture) => Some(texture.clone()),
            _ => None,
        };
        self.commands.push(Command::CopyBufferToTexture {
            src: src.clone(),
            dst,
            regions: regions.collect(),
        });
    }

    unsafe fn copy_texture_to_buffer<T>(
        &mut self,
        src: &Resource,
        src_usage: wgt::TextureUses,
        dst: &Buffer,
        regions: T,
    ) where
        T: Iterator<Item = crate::BufferTextureCopy>,
    {
        let src = match src {
            Resource::Texture(texture) => Some(texture.clone()),
            _ => None,
        };
        self.commands.push(Command::CopyTextureToBuffer {
            src,
            dst: dst.clone(),
            regions: regions.collect(),
        });
    }

    unsafe fn begin_query(&mut self, set: &Resource, index: u32) {
        self.record_unsupported();
    }
    unsafe fn end_query(&mut self, set: &Resource, index: u32) {
        self.record_unsupported();
    }
    unsafe fn write_timestamp(&mut self, set: &Resource, index: u32) {
        self.record_unsupported();
    }
    unsafe fn read_acceleration_structure_compact_size(
        &mut self,
        acceleration_structure: &Resource,
        buf: &Buffer,
    ) {
        self.record_unsupported();
    }
    unsafe fn reset_queries(&mut self, set: &Resource, range: Range<u32>) {
        self.record_unsupported();
    }
    unsafe fn copy_query_results(
        &mut self,
        set: &Resource,
        range: Range<u32>,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        stride: wgt::BufferSize,
    ) {
        self.record_unsupported();
    }

    // render

    unsafe fn begin_render_pass(
        &mut self,
        desc: &crate::RenderPassDescriptor<Resource, Resource>,
    ) -> DeviceResult<()> {
        if desc.multiview_mask.is_some()
            || desc.timestamp_writes.is_some()
            || desc.occlusion_query_set.is_some()
            || desc.sample_count != 1
        {
            return Err(crate::DeviceError::Lost);
        }

        if desc.color_attachments.len() > 8 {
            return Err(crate::DeviceError::Lost);
        }
        let mut images = Vec::with_capacity(desc.color_attachments.len());
        let mut clear_values = Vec::with_capacity(desc.color_attachments.len());
        for attachment in desc.color_attachments {
            let Some(attachment) = attachment else {
                images.push(None);
                clear_values.push(None);
                continue;
            };
            if attachment.resolve_target.is_some() || attachment.depth_slice.is_some() {
                return Err(crate::DeviceError::Lost);
            }
            let Resource::TextureView {
                image,
                extent,
                format,
                aspect,
                ..
            } = attachment.target.view
            else {
                return Err(crate::DeviceError::Lost);
            };
            if !matches!(
                *format,
                wgt::TextureFormat::Rgba8Unorm
                    | wgt::TextureFormat::Rgba8UnormSrgb
                    | wgt::TextureFormat::Rg16Float
                    | wgt::TextureFormat::Rgba16Float
            ) || *aspect != wgt::TextureAspect::All
            {
                return Err(crate::DeviceError::Lost);
            }
            let clear_value = if attachment.ops.contains(crate::AttachmentOps::LOAD_CLEAR) {
                Some(attachment.clear_value)
            } else if attachment.ops.contains(crate::AttachmentOps::LOAD)
                || attachment
                    .ops
                    .contains(crate::AttachmentOps::LOAD_DONT_CARE)
            {
                None
            } else {
                return Err(crate::DeviceError::Lost);
            };
            if extent.width != desc.extent.width || extent.height != desc.extent.height {
                return Err(crate::DeviceError::Lost);
            }
            images.push(Some(*image));
            clear_values.push(clear_value);
        }
        let (depth, depth_clear_value) = match desc.depth_stencil_attachment.as_ref() {
            None => (None, None),
            Some(depth) => {
                if !depth
                    .target
                    .usage
                    .contains(wgt::TextureUses::DEPTH_STENCIL_WRITE)
                {
                    return Err(crate::DeviceError::Lost);
                }
                let Resource::TextureView {
                    image,
                    extent,
                    format,
                    aspect,
                    ..
                } = depth.target.view
                else {
                    return Err(crate::DeviceError::Lost);
                };
                if extent.width != desc.extent.width
                    || extent.height != desc.extent.height
                    || !matches!(
                        *format,
                        wgt::TextureFormat::Depth16Unorm | wgt::TextureFormat::Depth32Float
                    )
                    || !matches!(
                        *aspect,
                        wgt::TextureAspect::All | wgt::TextureAspect::DepthOnly
                    )
                {
                    return Err(crate::DeviceError::Lost);
                }
                let clear = if depth.depth_ops.contains(crate::AttachmentOps::LOAD_CLEAR) {
                    Some(depth.clear_value.0)
                } else if depth.depth_ops.contains(crate::AttachmentOps::LOAD)
                    || depth
                        .depth_ops
                        .contains(crate::AttachmentOps::LOAD_DONT_CARE)
                {
                    None
                } else {
                    return Err(crate::DeviceError::Lost);
                };
                (Some(*image), clear)
            }
        };
        if images.iter().all(Option::is_none) && depth.is_none() {
            return Err(crate::DeviceError::Lost);
        }
        self.commands.push(Command::BeginRenderPass {
            images,
            extent: desc.extent,
            clear_values,
            depth,
            depth_clear_value,
        });
        Ok(())
    }
    unsafe fn end_render_pass(&mut self) {}

    unsafe fn set_bind_group(
        &mut self,
        layout: &Resource,
        index: u32,
        group: &Resource,
        dynamic_offsets: &[wgt::DynamicOffset],
    ) {
        if let Resource::BindGroup(group) = group {
            self.commands.push(Command::SetBindGroup {
                index,
                group: group.clone(),
                dynamic_offsets: dynamic_offsets.to_vec(),
            });
        }
    }
    unsafe fn set_immediates(&mut self, layout: &Resource, offset_bytes: u32, data: &[u32]) {
        self.record_unsupported();
    }

    unsafe fn insert_debug_marker(&mut self, label: &str) {}
    unsafe fn begin_debug_marker(&mut self, group_label: &str) {}
    unsafe fn end_debug_marker(&mut self) {}

    unsafe fn set_render_pipeline(&mut self, pipeline: &Resource) {
        if let Resource::RenderPipeline(pipeline) = pipeline {
            self.commands.push(Command::SetRenderPipeline {
                pipeline: pipeline.clone(),
            });
        }
    }

    unsafe fn set_index_buffer<'a>(
        &mut self,
        binding: crate::BufferBinding<'a, Buffer>,
        format: wgt::IndexFormat,
    ) {
        self.commands.push(Command::SetIndexBuffer {
            buffer: binding.buffer.clone(),
            offset: binding.offset,
            size: binding.size,
            format,
        });
    }
    unsafe fn set_vertex_buffer<'a>(
        &mut self,
        index: u32,
        binding: crate::BufferBinding<'a, Buffer>,
    ) {
        self.commands.push(Command::SetVertexBuffer {
            index,
            buffer: binding.buffer.clone(),
            offset: binding.offset,
            size: binding.size,
        });
    }
    unsafe fn set_viewport(&mut self, rect: &crate::Rect<f32>, depth_range: Range<f32>) {
        if !rect.x.is_finite()
            || !rect.y.is_finite()
            || !rect.w.is_finite()
            || !rect.h.is_finite()
            || rect.w < 0.0
            || rect.h < 0.0
            || !depth_range.start.is_finite()
            || !depth_range.end.is_finite()
            || depth_range.start < 0.0
            || depth_range.end > 1.0
            || depth_range.start > depth_range.end
        {
            self.record_unsupported();
            return;
        }
        self.commands.push(Command::SetViewport {
            rect: rect.clone(),
            depth_range,
        });
    }
    unsafe fn set_scissor_rect(&mut self, rect: &crate::Rect<u32>) {
        self.commands
            .push(Command::SetScissor { rect: rect.clone() });
    }
    unsafe fn set_stencil_reference(&mut self, value: u32) {
        self.commands.push(Command::SetStencilReference {
            reference: value as u8,
        });
    }
    unsafe fn set_blend_constants(&mut self, color: &[f32; 4]) {
        self.commands
            .push(Command::SetBlendConstants { color: *color });
    }

    unsafe fn draw(
        &mut self,
        first_vertex: u32,
        vertex_count: u32,
        first_instance: u32,
        instance_count: u32,
    ) {
        self.commands.push(Command::Draw {
            first_vertex,
            vertex_count,
            first_instance,
            instance_count,
        });
    }
    unsafe fn draw_indexed(
        &mut self,
        first_index: u32,
        index_count: u32,
        base_vertex: i32,
        first_instance: u32,
        instance_count: u32,
    ) {
        self.commands.push(Command::DrawIndexed {
            first_index,
            index_count,
            base_vertex,
            first_instance,
            instance_count,
        });
    }
    unsafe fn draw_mesh_tasks(
        &mut self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) {
        self.record_unsupported();
    }
    unsafe fn draw_indirect(
        &mut self,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) {
        self.record_unsupported();
    }
    unsafe fn draw_indexed_indirect(
        &mut self,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) {
        self.record_unsupported();
    }
    unsafe fn draw_mesh_tasks_indirect(
        &mut self,
        buffer: &<Self::A as crate::Api>::Buffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) {
        self.record_unsupported();
    }
    unsafe fn draw_indirect_count(
        &mut self,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        count_buffer: &Buffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    ) {
        self.record_unsupported();
    }
    unsafe fn draw_indexed_indirect_count(
        &mut self,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        count_buffer: &Buffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    ) {
        self.record_unsupported();
    }
    unsafe fn draw_mesh_tasks_indirect_count(
        &mut self,
        buffer: &<Self::A as crate::Api>::Buffer,
        offset: wgt::BufferAddress,
        count_buffer: &<Self::A as crate::Api>::Buffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    ) {
        self.record_unsupported();
    }

    // compute

    unsafe fn begin_compute_pass(&mut self, desc: &crate::ComputePassDescriptor<Resource>) {
        self.record_unsupported();
    }
    unsafe fn end_compute_pass(&mut self) {
        self.record_unsupported();
    }

    unsafe fn set_compute_pipeline(&mut self, pipeline: &Resource) {
        self.record_unsupported();
    }

    unsafe fn dispatch(&mut self, count: [u32; 3]) {
        self.record_unsupported();
    }
    unsafe fn dispatch_indirect(&mut self, buffer: &Buffer, offset: wgt::BufferAddress) {
        self.record_unsupported();
    }

    unsafe fn build_acceleration_structures<'a, T>(
        &mut self,
        _descriptor_count: u32,
        descriptors: T,
    ) where
        Api: 'a,
        T: IntoIterator<Item = crate::BuildAccelerationStructureDescriptor<'a, Buffer, Resource>>,
    {
        self.record_unsupported();
    }

    unsafe fn place_acceleration_structure_barrier(
        &mut self,
        _barriers: crate::AccelerationStructureBarrier,
    ) {
        self.record_unsupported();
    }

    unsafe fn copy_acceleration_structure_to_acceleration_structure(
        &mut self,
        src: &Resource,
        dst: &Resource,
        copy: wgt::AccelerationStructureCopy,
    ) {
        self.record_unsupported();
    }

    unsafe fn set_acceleration_structure_dependencies(
        command_buffers: &[&CommandBuffer],
        dependencies: &[&Resource],
    ) {
    }
}

impl Command {
    /// # Safety
    ///
    /// Must be called with appropriate synchronization for the resources affected by the command,
    /// such as ensuring that buffers are not accessed by a command while aliasing references exist.
    unsafe fn execute(
        &self,
        queue: &Queue,
        surface_queue: Option<RawQueueHandle>,
        state: &mut ExecutionState,
    ) -> DeviceResult<()> {
        match self {
            Command::ClearBuffer { ref buffer, range } => {
                // SAFETY:
                // Caller is responsible for ensuring this does not alias.
                let buffer_slice: &mut [u8] = unsafe { &mut *buffer.get_slice_ptr(range.clone()) };
                buffer_slice.fill(0);
                Ok(())
            }

            Command::CopyBufferToBuffer { src, dst, regions } => {
                for &crate::BufferCopy {
                    src_offset,
                    dst_offset,
                    size,
                } in regions
                {
                    // SAFETY:
                    // Caller is responsible for ensuring this does not alias.
                    let src_region: &[u8] =
                        unsafe { &*src.get_slice_ptr(src_offset..src_offset + size.get()) };
                    let dst_region: &mut [u8] =
                        unsafe { &mut *dst.get_slice_ptr(dst_offset..dst_offset + size.get()) };
                    dst_region.copy_from_slice(src_region);
                }
                #[cfg(target_os = "horizon")]
                unsafe {
                    submit_copy_buffer_to_buffer(queue, surface_queue, src, dst, regions)?;
                }
                Ok(())
            }
            Command::CopyBufferToTexture { src, dst, regions } => {
                let dst = dst.as_ref().ok_or(crate::DeviceError::Lost)?;
                unsafe { submit_copy_buffer_to_texture(queue, surface_queue, src, dst, regions) }
            }
            Command::CopyTextureToTexture { src, dst, regions } => {
                let src = src.as_ref().ok_or(crate::DeviceError::Lost)?;
                let dst = dst.as_ref().ok_or(crate::DeviceError::Lost)?;
                unsafe { submit_copy_texture_to_texture(queue, surface_queue, src, dst, regions) }
            }
            Command::CopyTextureToBuffer { src, dst, regions } => {
                let src = src.as_ref().ok_or(crate::DeviceError::Lost)?;
                unsafe { submit_copy_texture_to_buffer(queue, surface_queue, src, dst, regions) }
            }
            Command::ResourceBarrier { invalidate_flags } => unsafe {
                submit_resource_barrier(queue, surface_queue, *invalidate_flags)
            },
            Command::BeginRenderPass {
                images,
                extent,
                clear_values,
                depth,
                depth_clear_value,
            } => {
                state.target = Some(RenderTarget {
                    images: images.clone(),
                    extent: *extent,
                    depth: *depth,
                });
                state.viewport = None;
                state.scissor = None;
                unsafe {
                    submit_begin_render_pass(
                        queue,
                        surface_queue,
                        images,
                        *extent,
                        clear_values,
                        *depth,
                        *depth_clear_value,
                    )
                }
            }
            Command::SetRenderPipeline { pipeline } => {
                state.pipeline = Some(pipeline.clone());
                Ok(())
            }
            Command::SetVertexBuffer {
                index,
                buffer,
                offset,
                size,
            } => {
                let index = usize::try_from(*index).map_err(|_| crate::DeviceError::Lost)?;
                if state.vertex_buffers.len() <= index {
                    state.vertex_buffers.resize(index + 1, None);
                }
                state.vertex_buffers[index] = Some(VertexBinding {
                    buffer: buffer.clone(),
                    offset: *offset,
                    size: *size,
                });
                Ok(())
            }
            Command::SetIndexBuffer {
                buffer,
                offset,
                size,
                format,
            } => {
                state.index_buffer = Some(IndexBinding {
                    buffer: buffer.clone(),
                    offset: *offset,
                    size: *size,
                    format: *format,
                });
                Ok(())
            }
            Command::SetBindGroup {
                index,
                group,
                dynamic_offsets,
            } => {
                let index = usize::try_from(*index).map_err(|_| crate::DeviceError::Lost)?;
                if state.bind_groups.len() <= index {
                    state.bind_groups.resize(index + 1, None);
                }
                state.bind_groups[index] = Some(BoundBindGroup {
                    group: group.clone(),
                    dynamic_offsets: dynamic_offsets.clone(),
                });
                Ok(())
            }
            Command::SetViewport { rect, depth_range } => {
                state.viewport = Some((rect.clone(), depth_range.clone()));
                Ok(())
            }
            Command::SetScissor { rect } => {
                state.scissor = Some(rect.clone());
                Ok(())
            }
            Command::SetStencilReference { reference } => {
                state.stencil_reference = *reference;
                Ok(())
            }
            Command::SetBlendConstants { color } => {
                state.blend_constants = *color;
                Ok(())
            }
            Command::Draw {
                first_vertex,
                vertex_count,
                first_instance,
                instance_count,
            } => unsafe {
                submit_draw(
                    queue,
                    surface_queue,
                    state,
                    *first_vertex,
                    *vertex_count,
                    *first_instance,
                    *instance_count,
                )
            },
            Command::DrawIndexed {
                first_index,
                index_count,
                base_vertex,
                first_instance,
                instance_count,
            } => unsafe {
                submit_draw_indexed(
                    queue,
                    surface_queue,
                    state,
                    *first_index,
                    *index_count,
                    *base_vertex,
                    *first_instance,
                    *instance_count,
                )
            },
        }
    }
}

#[cfg(all(test, deko3d, not(target_os = "horizon")))]
mod tests {
    use super::*;
    use crate::{CommandEncoder as _, Queue as _};
    use std::sync::Mutex;

    fn queue() -> Queue {
        Queue {
            device: alloc::sync::Arc::new(super::super::DeviceInner {
                raw: super::super::RawDevice,
            }),
            state: Mutex::new(super::super::QueueState {
                raw: super::super::RawQueue,
            }),
        }
    }

    fn assert_end_fails(record: impl FnOnce(&mut CommandBuffer)) {
        let mut encoder = CommandBuffer::new();
        record(&mut encoder);
        let error = unsafe { encoder.end_encoding() }.unwrap_err();
        assert_eq!(error, crate::DeviceError::Lost);
    }

    #[test]
    fn unsupported_void_commands_fail_encoding_on_forced_host() {
        assert_end_fails(|encoder| unsafe {
            encoder.draw_mesh_tasks(1, 1, 1);
        });
        assert_end_fails(|encoder| unsafe {
            encoder.dispatch([1, 1, 1]);
        });
    }

    #[test]
    fn copy_dynamic_state_and_resource_transitions_encode_on_forced_host() {
        let mut encoder = CommandBuffer::new();
        let placeholder = Resource::Placeholder;
        unsafe {
            encoder.copy_texture_to_texture(
                &placeholder,
                wgt::TextureUses::COPY_SRC,
                &placeholder,
                core::iter::empty(),
            );
            encoder.set_stencil_reference(0xAB);
            encoder.set_blend_constants(&[0.25, 0.5, 0.75, 1.0]);
            encoder.transition_textures(core::iter::once(crate::TextureBarrier {
                texture: &placeholder,
                range: wgt::ImageSubresourceRange::default(),
                usage: crate::StateTransition {
                    from: wgt::TextureUses::COPY_SRC,
                    to: wgt::TextureUses::COPY_DST,
                },
            }));
        }
        let encoded = unsafe { encoder.end_encoding() }.unwrap();
        assert!(matches!(
            encoded.commands.as_slice(),
            [
                Command::CopyTextureToTexture { .. },
                Command::SetStencilReference { reference: 0xAB },
                Command::SetBlendConstants { .. },
                Command::ResourceBarrier {
                    invalidate_flags: DEKO_RESOURCE_TRANSITION_INVALIDATE_FLAGS
                }
            ]
        ));
    }

    #[test]
    fn viewport_and_scissor_encode_on_forced_host() {
        let mut encoder = CommandBuffer::new();
        unsafe {
            encoder.set_viewport(
                &crate::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 1.0,
                    h: 1.0,
                },
                0.0..1.0,
            );
            encoder.set_scissor_rect(&crate::Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            });
        }
        assert!(unsafe { encoder.end_encoding() }.is_ok());
    }

    #[test]
    fn failed_command_buffer_fails_execute_and_submit_on_forced_host() {
        let command_buffer = CommandBuffer {
            commands: Vec::new(),
            error: Some(crate::DeviceError::Lost),
        };
        let queue = queue();
        assert_eq!(
            unsafe { command_buffer.execute(&queue, None) },
            Err(crate::DeviceError::Lost)
        );

        let mut fence = super::super::Fence {
            value: super::super::AtomicU64::new(0),
            queue: Mutex::new(None),
        };
        assert_eq!(
            unsafe { queue.submit(&[&command_buffer], &[], (&mut fence, 1)) },
            Err(crate::DeviceError::Lost)
        );
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_buffer_to_buffer(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &Buffer,
    dst: &Buffer,
    regions: &[crate::BufferCopy],
) -> DeviceResult<()> {
    unsafe {
        submit_deko_commands(queue, surface_queue, "copy_buffer_to_buffer", |cmdbuf| {
            dk::dkCmdBufBarrier(
                cmdbuf,
                dk::DkBarrier::DkBarrier_Full,
                dk::DkInvalidateFlags_L2Cache,
            );
            for region in regions {
                let (src_addr, src_size) = src.gpu_binding(region.src_offset, Some(region.size))?;
                let (dst_addr, dst_size) = dst.gpu_binding(region.dst_offset, Some(region.size))?;
                if src_size != dst_size {
                    return Err(crate::DeviceError::Lost);
                }
                dk::dkCmdBufCopyBuffer(cmdbuf, src_addr, dst_addr, src_size);
            }
            dk::dkCmdBufBarrier(
                cmdbuf,
                dk::DkBarrier::DkBarrier_Full,
                dk::DkInvalidateFlags_Shader | dk::DkInvalidateFlags_L2Cache,
            );
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_buffer_to_texture(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &Buffer,
    dst: &TextureInner,
    regions: &[crate::BufferTextureCopy],
) -> DeviceResult<()> {
    let bytes_per_texel = match dst.format() {
        wgt::TextureFormat::R8Unorm => 1,
        wgt::TextureFormat::Rg8Unorm => 2,
        wgt::TextureFormat::Rgba8Unorm
        | wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Rgb9e5Ufloat
        | wgt::TextureFormat::R32Float => 4,
        wgt::TextureFormat::Rg16Float => 4,
        wgt::TextureFormat::Rgba16Float => 8,
        wgt::TextureFormat::Rgba32Float => 16,
        _ => return Err(crate::DeviceError::Lost),
    };
    unsafe {
        submit_deko_commands(queue, surface_queue, "copy_buffer_to_texture", |cmdbuf| {
            dk::dkCmdBufBarrier(
                cmdbuf,
                dk::DkBarrier::DkBarrier_Full,
                dk::DkInvalidateFlags_L2Cache,
            );
            for region in regions {
                if region.texture_base.aspect != crate::FormatAspects::COLOR {
                    return Err(crate::DeviceError::Lost);
                }
                let extent = dst.extent();
                if region.texture_base.origin.x + region.size.width > extent.width
                    || region.texture_base.origin.y + region.size.height > extent.height
                {
                    return Err(crate::DeviceError::Lost);
                }
                let z = if dst.dimension() == wgt::TextureDimension::D3 {
                    if region.texture_base.array_layer != 0
                        || region.texture_base.origin.z + region.size.depth
                            > extent.depth_or_array_layers
                    {
                        return Err(crate::DeviceError::Lost);
                    }
                    region.texture_base.origin.z
                } else {
                    if region.texture_base.origin.z != 0
                        || region.texture_base.array_layer + region.size.depth
                            > extent.depth_or_array_layers
                    {
                        return Err(crate::DeviceError::Lost);
                    }
                    0
                };
                let mut dst_view = dst.raw_image().1;
                if dst.dimension() == wgt::TextureDimension::D3 {
                    dst_view.type_ = dk::DkImageType::DkImageType_3D;
                } else {
                    dst_view.layerOffset = u16::try_from(region.texture_base.array_layer)
                        .map_err(|_| crate::DeviceError::Lost)?;
                    dst_view.layerCount =
                        u16::try_from(region.size.depth).map_err(|_| crate::DeviceError::Lost)?;
                }
                dst_view.mipLevelOffset = u8::try_from(region.texture_base.mip_level)
                    .map_err(|_| crate::DeviceError::Lost)?;
                dst_view.mipLevelCount = 1;

                let row_bytes = region
                    .size
                    .width
                    .checked_mul(bytes_per_texel)
                    .ok_or(crate::DeviceError::Lost)?;
                let bytes_per_row = region.buffer_layout.bytes_per_row.unwrap_or(row_bytes);
                if bytes_per_row < row_bytes || bytes_per_row % bytes_per_texel != 0 {
                    return Err(crate::DeviceError::Lost);
                }
                let rows_per_image = region
                    .buffer_layout
                    .rows_per_image
                    .unwrap_or(region.size.height);
                if rows_per_image < region.size.height {
                    return Err(crate::DeviceError::Lost);
                }
                let required_bytes = if region.size.height == 0 || region.size.depth == 0 {
                    0
                } else {
                    u64::from(bytes_per_row)
                        .checked_mul(u64::from(rows_per_image))
                        .and_then(|bytes_per_image| {
                            bytes_per_image.checked_mul(u64::from(region.size.depth - 1))
                        })
                        .and_then(|bytes| {
                            u64::from(bytes_per_row)
                                .checked_mul(u64::from(region.size.height - 1))
                                .and_then(|last_rows| bytes.checked_add(last_rows))
                        })
                        .and_then(|bytes| bytes.checked_add(u64::from(row_bytes)))
                        .ok_or(crate::DeviceError::Lost)?
                };
                let required_bytes =
                    wgt::BufferSize::new(required_bytes).ok_or(crate::DeviceError::Lost)?;
                let (src_addr, _) =
                    src.gpu_binding(region.buffer_layout.offset, Some(required_bytes))?;
                let trace_len = required_bytes.get().min(256);
                let trace_end = region
                    .buffer_layout
                    .offset
                    .checked_add(trace_len)
                    .ok_or(crate::DeviceError::Lost)?;
                let host_bytes = &*src.get_slice_ptr(region.buffer_layout.offset..trace_end);
                let gpu_bytes = src.gpu_slice(region.buffer_layout.offset..trace_end)?;
                super::trace::record_resource(format_args!(
                    "upload buffer_id={} buffer_label={:?} texture_id={} texture_label={:?} format={:?} dimension={:?} mip={} origin={:?} size={:?} offset={} bytes_per_row={} rows_per_image={} deko_row_length={} deko_image_height={} host_fingerprint={:016x} gpu_staging_fingerprint={:016x}",
                    src.id(),
                    src.label(),
                    dst.id(),
                    dst.label(),
                    dst.format(),
                    dst.dimension(),
                    region.texture_base.mip_level,
                    region.texture_base.origin,
                    region.size,
                    region.buffer_layout.offset,
                    bytes_per_row,
                    rows_per_image,
                    if bytes_per_row == row_bytes { 0 } else { bytes_per_row },
                    if rows_per_image == region.size.height { 0 } else { bytes_per_row * rows_per_image },
                    super::trace::fingerprint(host_bytes),
                    super::trace::fingerprint(gpu_bytes)
                ));
                let copy_src = dk::DkCopyBuf {
                    addr: src_addr,
                    rowLength: if bytes_per_row == row_bytes {
                        0
                    } else {
                        bytes_per_row
                    },
                    imageHeight: if rows_per_image == region.size.height {
                        0
                    } else {
                        bytes_per_row
                            .checked_mul(rows_per_image)
                            .ok_or(crate::DeviceError::Lost)?
                    },
                };
                let copy_rect = dk::DkImageRect {
                    x: region.texture_base.origin.x,
                    y: region.texture_base.origin.y,
                    z,
                    width: region.size.width,
                    height: region.size.height,
                    depth: region.size.depth,
                };
                if dst.dimension() == wgt::TextureDimension::D3 && region.size.depth > 1 {
                    let bytes_per_image = bytes_per_row
                        .checked_mul(rows_per_image)
                        .ok_or(crate::DeviceError::Lost)?;
                    for slice in 0..region.size.depth {
                        let slice_src = dk::DkCopyBuf {
                            addr: copy_src.addr + u64::from(bytes_per_image) * u64::from(slice),
                            ..copy_src
                        };
                        let slice_rect = dk::DkImageRect {
                            z: copy_rect.z + slice,
                            depth: 1,
                            ..copy_rect
                        };
                        dk::dkCmdBufCopyBufferToImage(
                            cmdbuf,
                            &slice_src,
                            &dst_view,
                            &slice_rect,
                            0,
                        );
                    }
                } else {
                    dk::dkCmdBufCopyBufferToImage(cmdbuf, &copy_src, &dst_view, &copy_rect, 0);
                }
            }
            dk::dkCmdBufBarrier(
                cmdbuf,
                dk::DkBarrier::DkBarrier_Full,
                dk::DkInvalidateFlags_Shader | dk::DkInvalidateFlags_L2Cache,
            );
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
fn texture_copy_view_and_rect(
    texture: &TextureInner,
    base: &crate::TextureCopyBase,
    size: crate::CopyExtent,
) -> DeviceResult<(dk::DkImageView, dk::DkImageRect)> {
    let expected_aspect = match texture.format() {
        wgt::TextureFormat::Depth16Unorm | wgt::TextureFormat::Depth32Float => {
            crate::FormatAspects::DEPTH
        }
        _ => crate::FormatAspects::COLOR,
    };
    if base.aspect != expected_aspect || size.depth == 0 {
        return Err(crate::DeviceError::Lost);
    }
    let extent = texture.extent();
    let mip_width = extent.width.checked_shr(base.mip_level).unwrap_or(0).max(1);
    let mip_height = extent
        .height
        .checked_shr(base.mip_level)
        .unwrap_or(0)
        .max(1);
    let end_x = base
        .origin
        .x
        .checked_add(size.width)
        .ok_or(crate::DeviceError::Lost)?;
    let end_y = base
        .origin
        .y
        .checked_add(size.height)
        .ok_or(crate::DeviceError::Lost)?;
    if end_x > mip_width || end_y > mip_height {
        return Err(crate::DeviceError::Lost);
    }

    let mut view = texture.raw_image().1;
    view.mipLevelOffset = u8::try_from(base.mip_level).map_err(|_| crate::DeviceError::Lost)?;
    view.mipLevelCount = 1;
    let z = if texture.dimension() == wgt::TextureDimension::D3 {
        if base.array_layer != 0
            || base
                .origin
                .z
                .checked_add(size.depth)
                .ok_or(crate::DeviceError::Lost)?
                > extent.depth_or_array_layers
        {
            return Err(crate::DeviceError::Lost);
        }
        view.type_ = dk::DkImageType::DkImageType_3D;
        base.origin.z
    } else {
        if base.origin.z != 0
            || base
                .array_layer
                .checked_add(size.depth)
                .ok_or(crate::DeviceError::Lost)?
                > extent.depth_or_array_layers
        {
            return Err(crate::DeviceError::Lost);
        }
        view.layerOffset = u16::try_from(base.array_layer).map_err(|_| crate::DeviceError::Lost)?;
        view.layerCount = u16::try_from(size.depth).map_err(|_| crate::DeviceError::Lost)?;
        0
    };
    Ok((
        view,
        dk::DkImageRect {
            x: base.origin.x,
            y: base.origin.y,
            z,
            width: size.width,
            height: size.height,
            depth: size.depth,
        },
    ))
}

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_texture_to_texture(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &TextureInner,
    dst: &TextureInner,
    regions: &[crate::TextureCopy],
) -> DeviceResult<()> {
    if src.format().remove_srgb_suffix() != dst.format().remove_srgb_suffix() {
        return Err(crate::DeviceError::Lost);
    }
    unsafe {
        submit_deko_commands(queue, surface_queue, "copy_texture_to_texture", |cmdbuf| {
            for region in regions {
                let (src_view, src_rect) =
                    texture_copy_view_and_rect(src, &region.src_base, region.size)?;
                let (dst_view, dst_rect) =
                    texture_copy_view_and_rect(dst, &region.dst_base, region.size)?;
                dk::dkCmdBufCopyImage(cmdbuf, &src_view, &src_rect, &dst_view, &dst_rect, 0);
            }
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_resource_barrier(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    invalidate_flags: u32,
) -> DeviceResult<()> {
    unsafe {
        submit_deko_commands(queue, surface_queue, "resource_barrier", |cmdbuf| {
            dk::dkCmdBufBarrier(cmdbuf, dk::DkBarrier::DkBarrier_Full, invalidate_flags);
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_texture_to_buffer(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &TextureInner,
    dst: &Buffer,
    regions: &[crate::BufferTextureCopy],
) -> DeviceResult<()> {
    let bytes_per_texel = match src.format() {
        wgt::TextureFormat::R8Unorm => 1,
        wgt::TextureFormat::Rg8Unorm => 2,
        wgt::TextureFormat::Rgba8Unorm
        | wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Rgb9e5Ufloat
        | wgt::TextureFormat::R32Float => 4,
        wgt::TextureFormat::Rg16Float => 4,
        wgt::TextureFormat::Rgba16Float => 8,
        wgt::TextureFormat::Rgba32Float => 16,
        _ => return Err(crate::DeviceError::Lost),
    };
    let mut downloaded_ranges = Vec::with_capacity(regions.len());
    unsafe {
        submit_deko_commands(queue, surface_queue, "copy_texture_to_buffer", |cmdbuf| {
            dk::dkCmdBufBarrier(
                cmdbuf,
                dk::DkBarrier::DkBarrier_Full,
                dk::DkInvalidateFlags_L2Cache,
            );
            for region in regions {
                if region.texture_base.aspect != crate::FormatAspects::COLOR {
                    return Err(crate::DeviceError::Lost);
                }
                let extent = src.extent();
                if region.texture_base.origin.x + region.size.width > extent.width
                    || region.texture_base.origin.y + region.size.height > extent.height
                {
                    return Err(crate::DeviceError::Lost);
                }
                let z = if src.dimension() == wgt::TextureDimension::D3 {
                    if region.texture_base.array_layer != 0
                        || region.texture_base.origin.z + region.size.depth
                            > extent.depth_or_array_layers
                    {
                        return Err(crate::DeviceError::Lost);
                    }
                    region.texture_base.origin.z
                } else {
                    if region.texture_base.origin.z != 0
                        || region.texture_base.array_layer + region.size.depth
                            > extent.depth_or_array_layers
                    {
                        return Err(crate::DeviceError::Lost);
                    }
                    0
                };
                let mut src_view = src.raw_image().1;
                if src.dimension() == wgt::TextureDimension::D3 {
                    src_view.type_ = dk::DkImageType::DkImageType_3D;
                } else {
                    src_view.layerOffset = u16::try_from(region.texture_base.array_layer)
                        .map_err(|_| crate::DeviceError::Lost)?;
                    src_view.layerCount =
                        u16::try_from(region.size.depth).map_err(|_| crate::DeviceError::Lost)?;
                }
                src_view.mipLevelOffset = u8::try_from(region.texture_base.mip_level)
                    .map_err(|_| crate::DeviceError::Lost)?;
                src_view.mipLevelCount = 1;

                let row_bytes = region
                    .size
                    .width
                    .checked_mul(bytes_per_texel)
                    .ok_or(crate::DeviceError::Lost)?;
                let bytes_per_row = region.buffer_layout.bytes_per_row.unwrap_or(row_bytes);
                let rows_per_image = region
                    .buffer_layout
                    .rows_per_image
                    .unwrap_or(region.size.height);
                if bytes_per_row < row_bytes
                    || bytes_per_row % bytes_per_texel != 0
                    || rows_per_image < region.size.height
                {
                    return Err(crate::DeviceError::Lost);
                }
                let required_bytes = if region.size.height == 0 || region.size.depth == 0 {
                    0
                } else {
                    u64::from(bytes_per_row)
                        .checked_mul(u64::from(rows_per_image))
                        .and_then(|bytes_per_image| {
                            bytes_per_image.checked_mul(u64::from(region.size.depth - 1))
                        })
                        .and_then(|bytes| {
                            u64::from(bytes_per_row)
                                .checked_mul(u64::from(region.size.height - 1))
                                .and_then(|last_rows| bytes.checked_add(last_rows))
                        })
                        .and_then(|bytes| bytes.checked_add(u64::from(row_bytes)))
                        .ok_or(crate::DeviceError::Lost)?
                };
                let required_size =
                    wgt::BufferSize::new(required_bytes).ok_or(crate::DeviceError::Lost)?;
                let (dst_addr, _) =
                    dst.gpu_binding(region.buffer_layout.offset, Some(required_size))?;
                let copy_dst = dk::DkCopyBuf {
                    addr: dst_addr,
                    rowLength: if bytes_per_row == row_bytes {
                        0
                    } else {
                        bytes_per_row
                    },
                    imageHeight: if rows_per_image == region.size.height {
                        0
                    } else {
                        bytes_per_row
                            .checked_mul(rows_per_image)
                            .ok_or(crate::DeviceError::Lost)?
                    },
                };
                let copy_rect = dk::DkImageRect {
                    x: region.texture_base.origin.x,
                    y: region.texture_base.origin.y,
                    z,
                    width: region.size.width,
                    height: region.size.height,
                    depth: region.size.depth,
                };
                if src.dimension() == wgt::TextureDimension::D3 && region.size.depth > 1 {
                    let bytes_per_image = bytes_per_row
                        .checked_mul(rows_per_image)
                        .ok_or(crate::DeviceError::Lost)?;
                    for slice in 0..region.size.depth {
                        let slice_dst = dk::DkCopyBuf {
                            addr: copy_dst.addr + u64::from(bytes_per_image) * u64::from(slice),
                            ..copy_dst
                        };
                        let slice_rect = dk::DkImageRect {
                            z: copy_rect.z + slice,
                            depth: 1,
                            ..copy_rect
                        };
                        dk::dkCmdBufCopyImageToBuffer(
                            cmdbuf,
                            &src_view,
                            &slice_rect,
                            &slice_dst,
                            0,
                        );
                    }
                } else {
                    dk::dkCmdBufCopyImageToBuffer(cmdbuf, &src_view, &copy_rect, &copy_dst, 0);
                }
                downloaded_ranges.push(
                    region.buffer_layout.offset..region.buffer_layout.offset + required_bytes,
                );
            }
            dk::dkCmdBufBarrier(
                cmdbuf,
                dk::DkBarrier::DkBarrier_Full,
                dk::DkInvalidateFlags_L2Cache,
            );
            Ok(())
        })?;
        for range in downloaded_ranges {
            dst.download_from_gpu(range.clone())?;
            let fingerprint_end = range.start + (range.end - range.start).min(256);
            let bytes = &*dst.get_slice_ptr(range.start..fingerprint_end);
            super::trace::record(format_args!(
                "readback texture_id={} texture_label={:?} buffer_id={} buffer_label={:?} offset={} bytes={} fingerprint={:016x}",
                src.id(),
                src.label(),
                dst.id(),
                dst.label(),
                range.start,
                range.end - range.start,
                super::trace::fingerprint(bytes)
            ));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_copy_texture_to_buffer(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _src: &TextureInner,
    _dst: &Buffer,
    _regions: &[crate::BufferTextureCopy],
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_copy_texture_to_texture(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _src: &TextureInner,
    _dst: &TextureInner,
    _regions: &[crate::TextureCopy],
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_resource_barrier(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _invalidate_flags: u32,
) -> DeviceResult<()> {
    Ok(())
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_copy_buffer_to_texture(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _src: &Buffer,
    _dst: &TextureInner,
    _regions: &[crate::BufferTextureCopy],
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn bind_render_targets(
    cmdbuf: dk::DkCmdBuf,
    images: &[Option<RawImage>],
    depth: Option<&dk::DkImageView>,
) {
    let views = images
        .iter()
        .map(|image| image.map(|image| image.1))
        .collect::<Vec<_>>();
    let view_ptrs = views
        .iter()
        .map(|view| view.as_ref().map_or(ptr::null(), ptr::from_ref))
        .collect::<Vec<_>>();
    unsafe {
        dk::dkCmdBufBindRenderTargets(
            cmdbuf,
            view_ptrs.as_ptr(),
            view_ptrs.len() as u32,
            depth.map_or(ptr::null(), ptr::from_ref),
        );
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_begin_render_pass(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    images: &[Option<RawImage>],
    extent: wgt::Extent3d,
    clear_values: &[Option<wgt::Color>],
    depth: Option<RawImage>,
    depth_clear_value: Option<f32>,
) -> DeviceResult<()> {
    unsafe {
        submit_deko_commands(queue, surface_queue, "begin_render_pass", |cmdbuf| {
            let depth_view = depth.map(|depth| depth.1);
            bind_render_targets(cmdbuf, images, depth_view.as_ref());
            let viewport = dk::DkViewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                near: 0.0,
                far: 1.0,
            };
            let scissor = dk::DkScissor {
                x: 0,
                y: 0,
                width: extent.width,
                height: extent.height,
            };
            dk::dkCmdBufSetViewports(cmdbuf, 0, &viewport, 1);
            dk::dkCmdBufSetScissors(cmdbuf, 0, &scissor, 1);
            for (index, clear_value) in clear_values.iter().enumerate() {
                if let Some(clear_value) = clear_value {
                    dk::dkCmdBufClearColorFloat(
                        cmdbuf,
                        index as u32,
                        dk::DkColorMask_RGBA,
                        clear_value.r as f32,
                        clear_value.g as f32,
                        clear_value.b as f32,
                        clear_value.a as f32,
                    );
                }
            }
            if let Some(depth_clear_value) = depth_clear_value {
                dk::dkCmdBufClearDepthStencil(cmdbuf, true, depth_clear_value, 0, 0);
            }
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_begin_render_pass(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _images: &[Option<RawImage>],
    _extent: wgt::Extent3d,
    _clear_values: &[Option<wgt::Color>],
    _depth: Option<RawImage>,
    _depth_clear_value: Option<f32>,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_draw(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    first_vertex: u32,
    vertex_count: u32,
    first_instance: u32,
    instance_count: u32,
) -> DeviceResult<()> {
    trace_draw(
        "non_indexed",
        state,
        first_vertex,
        vertex_count,
        0,
        first_instance,
        instance_count,
    );
    unsafe {
        submit_deko_draw(queue, surface_queue, state, |cmdbuf, pipeline| {
            dk::dkCmdBufDraw(
                cmdbuf,
                pipeline.primitive,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_draw_indexed(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    first_index: u32,
    index_count: u32,
    base_vertex: i32,
    first_instance: u32,
    instance_count: u32,
) -> DeviceResult<()> {
    let index_binding = state
        .index_buffer
        .as_ref()
        .ok_or(crate::DeviceError::Lost)?;
    trace_draw(
        "indexed",
        state,
        first_index,
        index_count,
        base_vertex,
        first_instance,
        instance_count,
    );
    unsafe {
        submit_deko_draw(queue, surface_queue, state, |cmdbuf, pipeline| {
            let (index_addr, _) = index_binding
                .buffer
                .gpu_binding(index_binding.offset, index_binding.size)?;
            dk::dkCmdBufBindIdxBuffer(cmdbuf, map_index_format(index_binding.format), index_addr);
            dk::dkCmdBufDrawIndexed(
                cmdbuf,
                pipeline.primitive,
                index_count,
                instance_count,
                first_index,
                base_vertex,
                first_instance,
            );
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
fn trace_draw(
    kind: &str,
    state: &ExecutionState,
    first: u32,
    count: u32,
    base_vertex: i32,
    first_instance: u32,
    instance_count: u32,
) {
    let _ = (
        kind,
        state,
        first,
        count,
        base_vertex,
        first_instance,
        instance_count,
    );
}

#[cfg(target_os = "horizon")]
unsafe fn submit_deko_draw(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    draw: impl FnOnce(dk::DkCmdBuf, &super::RenderPipelineInnerRaw) -> DeviceResult<()>,
) -> DeviceResult<()> {
    let target = state.target.as_ref().ok_or(crate::DeviceError::Lost)?;
    let pipeline = state.pipeline.as_ref().ok_or(crate::DeviceError::Lost)?;
    let pipeline = pipeline.raw();
    unsafe {
        submit_deko_commands(queue, surface_queue, "draw", |cmdbuf| {
            let depth_view = target.depth.map(|depth| depth.1);
            bind_render_targets(cmdbuf, &target.images, depth_view.as_ref());
            let (viewport_rect, depth_range) = state.viewport.as_ref().cloned().unwrap_or((
                crate::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: target.extent.width as f32,
                    h: target.extent.height as f32,
                },
                0.0..1.0,
            ));
            let viewport = dk::DkViewport {
                x: viewport_rect.x,
                y: viewport_rect.y,
                width: viewport_rect.w,
                height: viewport_rect.h,
                near: depth_range.start,
                far: depth_range.end,
            };
            let scissor_rect = state.scissor.as_ref().cloned().unwrap_or(crate::Rect {
                x: 0,
                y: 0,
                w: target.extent.width,
                h: target.extent.height,
            });
            let scissor = dk::DkScissor {
                x: scissor_rect.x,
                y: scissor_rect.y,
                width: scissor_rect.w,
                height: scissor_rect.h,
            };
            let shaders = [
                pipeline.vertex_shader.raw_shader(),
                pipeline.fragment_shader.raw_shader(),
            ];
            dk::dkCmdBufSetViewports(cmdbuf, 0, &viewport, 1);
            dk::dkCmdBufSetScissors(cmdbuf, 0, &scissor, 1);
            dk::dkCmdBufBindShaders(
                cmdbuf,
                dk::DkStageFlag_GraphicsMask,
                shaders.as_ptr(),
                shaders.len() as u32,
            );
            let bound_group = |index: u32| {
                state
                    .bind_groups
                    .get(index as usize)
                    .and_then(Option::as_ref)
                    .filter(|_| (index as usize) < pipeline.bind_group_count)
                    .ok_or(crate::DeviceError::Lost)
            };
            let fragment_textures = pipeline
                .fragment_bindings
                .iter()
                .filter(|binding| binding.kind == ShaderBindingKind::Texture)
                .collect::<Vec<_>>();
            if !fragment_textures.is_empty() {
                let (image_addr, sampler_addr, image_stride, sampler_stride) = fragment_textures
                    .iter()
                    .find_map(|binding| {
                        bound_group(binding.group)
                            .ok()
                            .and_then(|group| group.group.texture_descriptor_set())
                    })
                    .ok_or(crate::DeviceError::Lost)?;
                let descriptor_count = fragment_textures
                    .iter()
                    .map(|binding| binding.target)
                    .max()
                    .and_then(|target| target.checked_add(1))
                    .ok_or(crate::DeviceError::Lost)?;
                if descriptor_count > super::DEKO_TEXTURE_SAMPLER_COUNT as u32 {
                    return Err(crate::DeviceError::Lost);
                }
                for binding in fragment_textures {
                    let group = bound_group(binding.group)?;
                    group.group.push_texture_binding(
                        cmdbuf,
                        image_addr,
                        sampler_addr,
                        image_stride,
                        sampler_stride,
                        binding.group,
                        binding.binding,
                        binding.target,
                    )?;
                }
                dk::dkCmdBufBindImageDescriptorSet(cmdbuf, image_addr, descriptor_count);
                dk::dkCmdBufBindSamplerDescriptorSet(cmdbuf, sampler_addr, descriptor_count);
            }
            for (bindings, stage) in [
                (&pipeline.vertex_bindings, dk::DkStage::DkStage_Vertex),
                (&pipeline.fragment_bindings, dk::DkStage::DkStage_Fragment),
            ] {
                for binding in bindings
                    .iter()
                    .filter(|binding| binding.kind == ShaderBindingKind::Uniform)
                {
                    let group = bound_group(binding.group)?;
                    group.group.bind_uniform_binding(
                        cmdbuf,
                        &group.dynamic_offsets,
                        binding.binding,
                        binding.target,
                        stage,
                    )?;
                }
            }
            dk::dkCmdBufBindRasterizerState(cmdbuf, &pipeline.rasterizer_state);
            dk::dkCmdBufBindColorState(cmdbuf, &pipeline.color_state);
            dk::dkCmdBufBindColorWriteState(cmdbuf, &pipeline.color_write_state);
            if !pipeline.blend_states.is_empty() {
                dk::dkCmdBufBindBlendStates(
                    cmdbuf,
                    0,
                    pipeline.blend_states.as_ptr(),
                    pipeline.blend_states.len() as u32,
                );
            }
            if pipeline.uses_depth_stencil && target.depth.is_none() {
                return Err(crate::DeviceError::Lost);
            }
            dk::dkCmdBufBindDepthStencilState(cmdbuf, &pipeline.depth_stencil_state);
            dk::dkCmdBufSetStencil(
                cmdbuf,
                dk::DkFace_FrontAndBack,
                pipeline.stencil_write_mask,
                state.stencil_reference,
                pipeline.stencil_read_mask,
            );
            dk::dkCmdBufSetBlendConst(
                cmdbuf,
                state.blend_constants[0],
                state.blend_constants[1],
                state.blend_constants[2],
                state.blend_constants[3],
            );
            for (index, binding) in state.vertex_buffers.iter().enumerate() {
                let Some(binding) = binding else {
                    continue;
                };
                let (gpu_addr, gpu_size) =
                    binding.buffer.gpu_binding(binding.offset, binding.size)?;
                dk::dkCmdBufBindVtxBuffer(
                    cmdbuf,
                    u32::try_from(index).map_err(|_| crate::DeviceError::Lost)?,
                    gpu_addr,
                    gpu_size,
                );
            }
            dk::dkCmdBufBindVtxAttribState(
                cmdbuf,
                pipeline.vertex_attributes.as_ptr(),
                pipeline.vertex_attributes.len() as u32,
            );
            dk::dkCmdBufBindVtxBufferState(
                cmdbuf,
                pipeline.vertex_buffers.as_ptr(),
                pipeline.vertex_buffers.len() as u32,
            );
            draw(cmdbuf, pipeline)
        })
    }
}

#[cfg(target_os = "horizon")]
fn map_index_format(format: wgt::IndexFormat) -> dk::DkIdxFormat {
    match format {
        wgt::IndexFormat::Uint16 => dk::DkIdxFormat::DkIdxFormat_Uint16,
        wgt::IndexFormat::Uint32 => dk::DkIdxFormat::DkIdxFormat_Uint32,
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_draw(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _first_vertex: u32,
    _vertex_count: u32,
    _first_instance: u32,
    _instance_count: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
#[allow(clippy::too_many_arguments)]
unsafe fn submit_draw_indexed(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _first_index: u32,
    _index_count: u32,
    _base_vertex: i32,
    _first_instance: u32,
    _instance_count: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_deko_commands(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    label: &'static str,
    record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
) -> DeviceResult<()> {
    let raw_queue = match surface_queue {
        Some(surface_queue) => surface_queue,
        None => queue.raw_queue()?,
    }
    .0;
    unsafe { queue.record_and_submit(raw_queue, label, record) }
}
