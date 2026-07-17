use alloc::vec::Vec;
use core::mem;
use core::ops::Range;

#[cfg(target_os = "horizon")]
use core::mem::size_of;

#[cfg(target_os = "horizon")]
use core::ptr;
#[cfg(target_os = "horizon")]
use deko3d_sys as dk;

#[cfg(target_os = "horizon")]
use super::ShaderBindingKind;
#[cfg(target_os = "horizon")]
use super::DEKO_QUERY_RESULT_SIZE;

use super::{
    Api, BindGroupInner, Buffer, ComputePipelineInner, DeviceResult, QuerySetInner, Queue,
    RawImage, RawQueueHandle, RenderPipelineInner, Resource, TextureInner,
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
    CopyQueryResults {
        query_set: alloc::sync::Arc<QuerySetInner>,
        range: Range<u32>,
        buffer: Buffer,
        offset: wgt::BufferAddress,
        stride: wgt::BufferSize,
    },
    ResetQueries {
        query_set: alloc::sync::Arc<QuerySetInner>,
        range: Range<u32>,
    },
    BeginOcclusionQuery {
        query_set: alloc::sync::Arc<QuerySetInner>,
        index: u32,
    },
    EndOcclusionQuery {
        query_set: alloc::sync::Arc<QuerySetInner>,
        index: u32,
    },
    WriteTimestamp(TimestampWrite),
    ResourceBarrier {
        invalidate_flags: u32,
    },
    BeginRenderPass {
        images: Vec<Option<RawImage>>,
        resolve_images: Vec<Option<RawImage>>,
        extent: wgt::Extent3d,
        clear_values: Vec<Option<wgt::Color>>,
        depth: Option<RawImage>,
        depth_clear_value: Option<f32>,
        stencil_clear_value: Option<u32>,
        beginning_timestamp: Option<TimestampWrite>,
        end_timestamp: Option<TimestampWrite>,
        multiview_mask: Option<core::num::NonZeroU32>,
    },
    EndRenderPass,
    SetRenderPipeline {
        pipeline: alloc::sync::Arc<RenderPipelineInner>,
    },
    BeginComputePass {
        beginning_timestamp: Option<TimestampWrite>,
        end_timestamp: Option<TimestampWrite>,
    },
    EndComputePass,
    SetComputePipeline {
        pipeline: alloc::sync::Arc<ComputePipelineInner>,
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
    SetImmediates {
        layout: alloc::sync::Arc<super::PipelineLayoutInner>,
        offset_bytes: u32,
        data: Vec<u32>,
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
    DrawIndirect {
        buffer: Buffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    },
    DrawIndexedIndirect {
        buffer: Buffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    },
    DrawIndirectCount {
        buffer: Buffer,
        offset: wgt::BufferAddress,
        count_buffer: Buffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    },
    DrawIndexedIndirectCount {
        buffer: Buffer,
        offset: wgt::BufferAddress,
        count_buffer: Buffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    },
    DispatchWorkgroups {
        count: [u32; 3],
    },
    DispatchWorkgroupsIndirect {
        buffer: Buffer,
        offset: wgt::BufferAddress,
    },
}

#[derive(Clone, Debug)]
struct TimestampWrite {
    query_set: alloc::sync::Arc<QuerySetInner>,
    index: u32,
}

#[derive(Default)]
struct ExecutionState {
    target: Option<RenderTarget>,
    pipeline: Option<alloc::sync::Arc<RenderPipelineInner>>,
    compute_pipeline: Option<alloc::sync::Arc<ComputePipelineInner>>,
    in_compute_pass: bool,
    vertex_buffers: Vec<Option<VertexBinding>>,
    index_buffer: Option<IndexBinding>,
    bind_groups: Vec<Option<BoundBindGroup>>,
    viewport: Option<(crate::Rect<f32>, Range<f32>)>,
    scissor: Option<crate::Rect<u32>>,
    stencil_reference: u8,
    blend_constants: [f32; 4],
    render_pass_end_timestamp: Option<TimestampWrite>,
    compute_pass_end_timestamp: Option<TimestampWrite>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct RenderTarget {
    images: Vec<Option<RawImage>>,
    resolve_images: Vec<Option<RawImage>>,
    extent: wgt::Extent3d,
    depth: Option<RawImage>,
    multiview_mask: Option<core::num::NonZeroU32>,
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
                    command.name()
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
        if self.commands.is_empty() && self.error.is_none() {
            Ok(())
        } else {
            Err(crate::DeviceError::Lost)
        }
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
        let Resource::QuerySet(query_set) = set else {
            self.record_unsupported();
            return;
        };
        let Some(end) = index.checked_add(1) else {
            self.record_unsupported();
            return;
        };
        if !query_set.is_occlusion() || query_set.validate_range(index..end).is_err() {
            self.record_unsupported();
            return;
        }
        self.commands.push(Command::BeginOcclusionQuery {
            query_set: query_set.clone(),
            index,
        });
    }
    unsafe fn end_query(&mut self, set: &Resource, index: u32) {
        let Resource::QuerySet(query_set) = set else {
            self.record_unsupported();
            return;
        };
        let Some(end) = index.checked_add(1) else {
            self.record_unsupported();
            return;
        };
        if !query_set.is_occlusion() || query_set.validate_range(index..end).is_err() {
            self.record_unsupported();
            return;
        }
        self.commands.push(Command::EndOcclusionQuery {
            query_set: query_set.clone(),
            index,
        });
    }
    unsafe fn write_timestamp(&mut self, set: &Resource, index: u32) {
        let Resource::QuerySet(query_set) = set else {
            self.record_unsupported();
            return;
        };
        let Some(end) = index.checked_add(1) else {
            self.record_unsupported();
            return;
        };
        if !query_set.is_timestamp() || query_set.validate_range(index..end).is_err() {
            self.record_unsupported();
            return;
        }
        self.commands.push(Command::WriteTimestamp(TimestampWrite {
            query_set: query_set.clone(),
            index,
        }));
    }
    unsafe fn read_acceleration_structure_compact_size(
        &mut self,
        acceleration_structure: &Resource,
        buf: &Buffer,
    ) {
        self.record_unsupported();
    }
    unsafe fn reset_queries(&mut self, set: &Resource, range: Range<u32>) {
        let Resource::QuerySet(query_set) = set else {
            self.record_unsupported();
            return;
        };
        if query_set.validate_range(range.clone()).is_err() {
            self.record_unsupported();
            return;
        }
        self.commands.push(Command::ResetQueries {
            query_set: query_set.clone(),
            range,
        });
    }
    unsafe fn copy_query_results(
        &mut self,
        set: &Resource,
        range: Range<u32>,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        stride: wgt::BufferSize,
    ) {
        let Resource::QuerySet(query_set) = set else {
            self.record_unsupported();
            return;
        };
        if query_set.validate_range(range.clone()).is_err() {
            self.record_unsupported();
            return;
        }
        self.commands.push(Command::CopyQueryResults {
            query_set: query_set.clone(),
            range,
            buffer: buffer.clone(),
            offset,
            stride,
        });
    }

    // render

    unsafe fn begin_render_pass(
        &mut self,
        desc: &crate::RenderPassDescriptor<Resource, Resource>,
    ) -> DeviceResult<()> {
        if !supports_sample_count(desc.sample_count) {
            return Err(crate::DeviceError::Lost);
        }
        if let Some(query_set) = desc.occlusion_query_set {
            let Resource::QuerySet(query_set) = query_set else {
                return Err(crate::DeviceError::Lost);
            };
            if !query_set.is_occlusion() {
                return Err(crate::DeviceError::Lost);
            }
        }

        if desc.color_attachments.len() > 8 {
            return Err(crate::DeviceError::Lost);
        }
        let mut images = Vec::with_capacity(desc.color_attachments.len());
        let mut resolve_images = Vec::with_capacity(desc.color_attachments.len());
        let mut clear_values = Vec::with_capacity(desc.color_attachments.len());
        for attachment in desc.color_attachments {
            let Some(attachment) = attachment else {
                images.push(None);
                resolve_images.push(None);
                clear_values.push(None);
                continue;
            };
            if attachment.depth_slice.is_some() {
                return Err(crate::DeviceError::Lost);
            }
            let Resource::TextureView {
                image,
                extent,
                sample_count,
                format,
                aspect,
                ..
            } = attachment.target.view
            else {
                return Err(crate::DeviceError::Lost);
            };
            if !super::is_color_attachment_format(*format) || *aspect != wgt::TextureAspect::All {
                return Err(crate::DeviceError::Lost);
            }
            if *sample_count != desc.sample_count {
                return Err(crate::DeviceError::Lost);
            }
            let resolve_image = match attachment.resolve_target.as_ref() {
                None => None,
                Some(resolve) => {
                    if desc.sample_count == 1
                        || !resolve.usage.contains(wgt::TextureUses::COLOR_TARGET)
                    {
                        return Err(crate::DeviceError::Lost);
                    }
                    let Resource::TextureView {
                        image,
                        extent: resolve_extent,
                        sample_count,
                        format: resolve_format,
                        aspect: resolve_aspect,
                        ..
                    } = resolve.view
                    else {
                        return Err(crate::DeviceError::Lost);
                    };
                    if *sample_count != 1
                        || *resolve_extent != *extent
                        || *resolve_format != *format
                        || *resolve_aspect != wgt::TextureAspect::All
                    {
                        return Err(crate::DeviceError::Lost);
                    }
                    Some(*image)
                }
            };
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
            resolve_images.push(resolve_image);
            clear_values.push(clear_value);
        }
        let (depth, depth_clear_value, stencil_clear_value) =
            match desc.depth_stencil_attachment.as_ref() {
                None => (None, None, None),
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
                        sample_count,
                        format,
                        aspect,
                        ..
                    } = depth.target.view
                    else {
                        return Err(crate::DeviceError::Lost);
                    };
                    if extent.width != desc.extent.width
                        || extent.height != desc.extent.height
                        || !super::deko3d_texture_format_capabilities(*format)
                            .contains(crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT)
                        || !matches!(
                            *aspect,
                            wgt::TextureAspect::All | wgt::TextureAspect::DepthOnly
                        )
                    {
                        return Err(crate::DeviceError::Lost);
                    }
                    if *sample_count != desc.sample_count {
                        return Err(crate::DeviceError::Lost);
                    }
                    let has_depth = *format != wgt::TextureFormat::Stencil8;
                    let depth_clear = if !has_depth {
                        None
                    } else if depth.depth_ops.contains(crate::AttachmentOps::LOAD_CLEAR) {
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
                    let has_stencil = matches!(
                        *format,
                        wgt::TextureFormat::Stencil8
                            | wgt::TextureFormat::Depth24PlusStencil8
                            | wgt::TextureFormat::Depth32FloatStencil8
                    );
                    let stencil_clear = if !has_stencil {
                        None
                    } else if depth.stencil_ops.contains(crate::AttachmentOps::LOAD_CLEAR) {
                        Some(depth.clear_value.1)
                    } else if depth.stencil_ops.contains(crate::AttachmentOps::LOAD)
                        || depth
                            .stencil_ops
                            .contains(crate::AttachmentOps::LOAD_DONT_CARE)
                    {
                        None
                    } else {
                        return Err(crate::DeviceError::Lost);
                    };
                    (Some(*image), depth_clear, stencil_clear)
                }
            };
        if images.iter().all(Option::is_none) && depth.is_none() {
            return Err(crate::DeviceError::Lost);
        }
        let (beginning_timestamp, end_timestamp) =
            timestamp_writes(desc.timestamp_writes.as_ref())?;
        self.commands.push(Command::BeginRenderPass {
            images,
            resolve_images,
            extent: desc.extent,
            clear_values,
            depth,
            depth_clear_value,
            stencil_clear_value,
            beginning_timestamp,
            end_timestamp,
            multiview_mask: desc.multiview_mask,
        });
        Ok(())
    }
    unsafe fn end_render_pass(&mut self) {
        self.commands.push(Command::EndRenderPass);
    }

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
        let Resource::PipelineLayout(layout) = layout else {
            self.record_unsupported();
            return;
        };
        if !layout.contains_immediate_range(offset_bytes, data) {
            self.record_unsupported();
            return;
        }
        self.commands.push(Command::SetImmediates {
            layout: layout.clone(),
            offset_bytes,
            data: data.to_vec(),
        });
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
        self.commands.push(Command::DrawIndirect {
            buffer: buffer.clone(),
            offset,
            draw_count,
        });
    }
    unsafe fn draw_indexed_indirect(
        &mut self,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) {
        self.commands.push(Command::DrawIndexedIndirect {
            buffer: buffer.clone(),
            offset,
            draw_count,
        });
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
        self.commands.push(Command::DrawIndirectCount {
            buffer: buffer.clone(),
            offset,
            count_buffer: count_buffer.clone(),
            count_offset,
            max_count,
        });
    }
    unsafe fn draw_indexed_indirect_count(
        &mut self,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        count_buffer: &Buffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    ) {
        self.commands.push(Command::DrawIndexedIndirectCount {
            buffer: buffer.clone(),
            offset,
            count_buffer: count_buffer.clone(),
            count_offset,
            max_count,
        });
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
        match timestamp_writes(desc.timestamp_writes.as_ref()) {
            Ok((beginning_timestamp, end_timestamp)) => {
                self.commands.push(Command::BeginComputePass {
                    beginning_timestamp,
                    end_timestamp,
                });
            }
            Err(_) => self.record_unsupported(),
        }
    }
    unsafe fn end_compute_pass(&mut self) {
        self.commands.push(Command::EndComputePass);
    }

    unsafe fn set_compute_pipeline(&mut self, pipeline: &Resource) {
        if let Resource::ComputePipeline(pipeline) = pipeline {
            self.commands.push(Command::SetComputePipeline {
                pipeline: pipeline.clone(),
            });
        }
    }

    unsafe fn dispatch(&mut self, count: [u32; 3]) {
        self.commands.push(Command::DispatchWorkgroups { count });
    }
    unsafe fn dispatch_indirect(&mut self, buffer: &Buffer, offset: wgt::BufferAddress) {
        self.commands.push(Command::DispatchWorkgroupsIndirect {
            buffer: buffer.clone(),
            offset,
        });
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
    fn name(&self) -> &'static str {
        match self {
            Command::ClearBuffer { .. } => "clear_buffer",
            Command::CopyBufferToBuffer { .. } => "copy_buffer_to_buffer",
            Command::CopyBufferToTexture { .. } => "copy_buffer_to_texture",
            Command::CopyTextureToTexture { .. } => "copy_texture_to_texture",
            Command::CopyTextureToBuffer { .. } => "copy_texture_to_buffer",
            Command::CopyQueryResults { .. } => "copy_query_results",
            Command::ResetQueries { .. } => "reset_queries",
            Command::BeginOcclusionQuery { .. } => "begin_occlusion_query",
            Command::EndOcclusionQuery { .. } => "end_occlusion_query",
            Command::WriteTimestamp(_) => "write_timestamp",
            Command::ResourceBarrier { .. } => "resource_barrier",
            Command::BeginRenderPass { .. } => "begin_render_pass",
            Command::EndRenderPass => "end_render_pass",
            Command::BeginComputePass { .. } => "begin_compute_pass",
            Command::EndComputePass => "end_compute_pass",
            Command::SetRenderPipeline { .. } => "set_render_pipeline",
            Command::SetComputePipeline { .. } => "set_compute_pipeline",
            Command::SetVertexBuffer { .. } => "set_vertex_buffer",
            Command::SetIndexBuffer { .. } => "set_index_buffer",
            Command::SetBindGroup { .. } => "set_bind_group",
            Command::SetImmediates { .. } => "set_immediates",
            Command::SetViewport { .. } => "set_viewport",
            Command::SetScissor { .. } => "set_scissor",
            Command::SetStencilReference { .. } => "set_stencil_reference",
            Command::SetBlendConstants { .. } => "set_blend_constants",
            Command::Draw { .. } => "draw",
            Command::DrawIndexed { .. } => "draw_indexed",
            Command::DrawIndirect { .. } => "draw_indirect",
            Command::DrawIndexedIndirect { .. } => "draw_indexed_indirect",
            Command::DrawIndirectCount { .. } => "draw_indirect_count",
            Command::DrawIndexedIndirectCount { .. } => "draw_indexed_indirect_count",
            Command::DispatchWorkgroups { .. } => "dispatch_workgroups",
            Command::DispatchWorkgroupsIndirect { .. } => "dispatch_workgroups_indirect",
        }
    }

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
                let buffer_slice: &mut [u8] = unsafe { &mut *buffer.get_slice_ptr(range.clone())? };
                buffer_slice.fill(0);
                upload_after_host_write(buffer)?;
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
                    let src_end = src_offset
                        .checked_add(size.get())
                        .ok_or(crate::DeviceError::Lost)?;
                    let dst_end = dst_offset
                        .checked_add(size.get())
                        .ok_or(crate::DeviceError::Lost)?;
                    let src_region: &[u8] = unsafe { &*src.get_slice_ptr(src_offset..src_end)? };
                    let dst_region: &mut [u8] =
                        unsafe { &mut *dst.get_slice_ptr(dst_offset..dst_end)? };
                    dst_region.copy_from_slice(src_region);
                }
                upload_after_host_write(src)?;
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
            Command::CopyQueryResults {
                query_set,
                range,
                buffer,
                offset,
                stride,
            } => unsafe {
                submit_copy_query_results(
                    queue,
                    surface_queue,
                    query_set,
                    range.clone(),
                    buffer,
                    *offset,
                    *stride,
                )
            },
            Command::ResetQueries { query_set, range } => unsafe {
                submit_reset_query_results(queue, surface_queue, query_set, range.clone())
            },
            Command::BeginOcclusionQuery { query_set, index } => unsafe {
                submit_begin_occlusion_query(queue, surface_queue, query_set, *index)
            },
            Command::EndOcclusionQuery { query_set, index } => unsafe {
                submit_end_occlusion_query(queue, surface_queue, query_set, *index)
            },
            Command::WriteTimestamp(timestamp) => unsafe {
                submit_timestamp_write(queue, surface_queue, timestamp)
            },
            Command::ResourceBarrier { invalidate_flags } => unsafe {
                submit_resource_barrier(queue, surface_queue, *invalidate_flags)
            },
            Command::BeginRenderPass {
                images,
                resolve_images,
                extent,
                clear_values,
                depth,
                depth_clear_value,
                stencil_clear_value,
                beginning_timestamp,
                end_timestamp,
                multiview_mask,
            } => {
                state.target = Some(RenderTarget {
                    images: images.clone(),
                    resolve_images: resolve_images.clone(),
                    extent: *extent,
                    depth: *depth,
                    multiview_mask: *multiview_mask,
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
                        *stencil_clear_value,
                    )?;
                    if let Some(timestamp) = beginning_timestamp {
                        submit_timestamp_write(queue, surface_queue, timestamp)?;
                    }
                }
                state.render_pass_end_timestamp = end_timestamp.clone();
                Ok(())
            }
            Command::EndRenderPass => {
                if let Some(timestamp) = state.render_pass_end_timestamp.take() {
                    unsafe { submit_timestamp_write(queue, surface_queue, &timestamp)? };
                }
                unsafe { submit_end_render_pass(queue, surface_queue, state) }
            }
            Command::SetRenderPipeline { pipeline } => {
                state.pipeline = Some(pipeline.clone());
                Ok(())
            }
            Command::BeginComputePass {
                beginning_timestamp,
                end_timestamp,
            } => {
                state.in_compute_pass = true;
                state.compute_pipeline = None;
                state.compute_pass_end_timestamp = end_timestamp.clone();
                if let Some(timestamp) = beginning_timestamp {
                    unsafe { submit_timestamp_write(queue, surface_queue, timestamp)? };
                }
                Ok(())
            }
            Command::EndComputePass => {
                if !state.in_compute_pass {
                    return Err(crate::DeviceError::Lost);
                }
                if let Some(timestamp) = state.compute_pass_end_timestamp.take() {
                    unsafe { submit_timestamp_write(queue, surface_queue, &timestamp)? };
                }
                state.in_compute_pass = false;
                state.compute_pipeline = None;
                Ok(())
            }
            Command::SetComputePipeline { pipeline } => {
                if !state.in_compute_pass {
                    return Err(crate::DeviceError::Lost);
                }
                state.compute_pipeline = Some(pipeline.clone());
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
            Command::SetImmediates {
                layout,
                offset_bytes,
                data,
            } => unsafe {
                submit_set_immediates(queue, surface_queue, layout, *offset_bytes, data)
            },
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
            Command::DrawIndirect {
                buffer,
                offset,
                draw_count,
            } => unsafe {
                submit_draw_indirect(queue, surface_queue, state, buffer, *offset, *draw_count)
            },
            Command::DrawIndexedIndirect {
                buffer,
                offset,
                draw_count,
            } => unsafe {
                submit_draw_indexed_indirect(
                    queue,
                    surface_queue,
                    state,
                    buffer,
                    *offset,
                    *draw_count,
                )
            },
            Command::DrawIndirectCount {
                buffer,
                offset,
                count_buffer,
                count_offset,
                max_count,
            } => unsafe {
                submit_draw_indirect_count(
                    queue,
                    surface_queue,
                    state,
                    buffer,
                    *offset,
                    count_buffer,
                    *count_offset,
                    *max_count,
                )
            },
            Command::DrawIndexedIndirectCount {
                buffer,
                offset,
                count_buffer,
                count_offset,
                max_count,
            } => unsafe {
                submit_draw_indexed_indirect_count(
                    queue,
                    surface_queue,
                    state,
                    buffer,
                    *offset,
                    count_buffer,
                    *count_offset,
                    *max_count,
                )
            },
            Command::DispatchWorkgroups { count } => unsafe {
                submit_dispatch_workgroups(queue, surface_queue, state, *count)
            },
            Command::DispatchWorkgroupsIndirect { buffer, offset } => unsafe {
                submit_dispatch_workgroups_indirect(queue, surface_queue, state, buffer, *offset)
            },
        }
    }
}

fn upload_after_host_write(buffer: &Buffer) -> DeviceResult<()> {
    #[cfg(target_os = "horizon")]
    unsafe {
        buffer.upload_to_gpu()?;
    }
    #[cfg(not(target_os = "horizon"))]
    {
        let _ = buffer;
    }
    Ok(())
}

#[cfg(all(test, deko3d, not(target_os = "horizon")))]
mod tests {
    use super::*;

    #[test]
    fn color_copy_size_covers_integer_and_float_texels() {
        for (format, bytes) in [
            (wgt::TextureFormat::R8Uint, 1),
            (wgt::TextureFormat::R16Uint, 2),
            (wgt::TextureFormat::Rg32Sint, 8),
            (wgt::TextureFormat::Rgba16Float, 8),
            (wgt::TextureFormat::Rgba32Float, 16),
        ] {
            assert_eq!(color_copy_texel_size(format).unwrap(), bytes);
        }
        assert!(color_copy_texel_size(wgt::TextureFormat::Bc1RgbaUnorm).is_err());
    }

    #[test]
    fn selective_multiview_replays_only_active_view_indices() {
        let mask = core::num::NonZeroU32::new(0b10101).unwrap();
        assert_eq!(
            active_multiview_indices(mask).collect::<Vec<_>>(),
            [0, 2, 4]
        );
    }

    #[test]
    fn beginning_encoding_with_pending_commands_returns_an_error() {
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
            assert!(encoder.begin_encoding(None).is_err());
        }
    }
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
    }

    #[test]
    fn compute_pass_and_dispatch_encode_on_forced_host() {
        let mut encoder = CommandBuffer::new();
        unsafe {
            encoder.begin_compute_pass(&crate::ComputePassDescriptor {
                label: Some("host-compute"),
                timestamp_writes: None,
            });
            encoder.dispatch([2, 3, 4]);
            encoder.end_compute_pass();
        }
        let encoded = unsafe { encoder.end_encoding() }.unwrap();
        assert!(matches!(
            encoded.commands.as_slice(),
            [
                Command::BeginComputePass { .. },
                Command::DispatchWorkgroups { count: [2, 3, 4] },
                Command::EndComputePass
            ]
        ));
    }

    #[test]
    fn copy_dynamic_state_and_resource_transitions_encode_on_forced_host() {
        let mut encoder = CommandBuffer::new();
        let placeholder = Resource::Placeholder;
        let layout =
            Resource::PipelineLayout(alloc::sync::Arc::new(super::super::PipelineLayoutInner {
                immediate_size: 16,
                bind_group_layouts: Vec::new(),
            }));
        unsafe {
            encoder.copy_texture_to_texture(
                &placeholder,
                wgt::TextureUses::COPY_SRC,
                &placeholder,
                core::iter::empty(),
            );
            encoder.set_stencil_reference(0xAB);
            encoder.set_blend_constants(&[0.25, 0.5, 0.75, 1.0]);
            encoder.set_immediates(&layout, 4, &[1, 2]);
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
                Command::SetImmediates {
                    offset_bytes: 4,
                    data,
                    ..
                },
                Command::ResourceBarrier {
                    invalidate_flags: DEKO_RESOURCE_TRANSITION_INVALIDATE_FLAGS
                }
            ] if data == &[1, 2]
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
    fn timestamp_and_occlusion_queries_encode_on_forced_host() {
        let timestamp = Resource::QuerySet(alloc::sync::Arc::new(QuerySetInner {
            query_type: wgt::QueryType::Timestamp,
            count: 2,
        }));
        let occlusion = Resource::QuerySet(alloc::sync::Arc::new(QuerySetInner {
            query_type: wgt::QueryType::Occlusion,
            count: 2,
        }));
        let mut encoder = CommandBuffer::new();
        unsafe {
            encoder.reset_queries(&timestamp, 0..2);
            encoder.write_timestamp(&timestamp, 1);
            encoder.begin_query(&occlusion, 0);
            encoder.end_query(&occlusion, 0);
        }
        let encoded = unsafe { encoder.end_encoding() }.unwrap();
        assert!(matches!(
            encoded.commands.as_slice(),
            [
                Command::ResetQueries { range, .. },
                Command::WriteTimestamp(TimestampWrite { index: 1, .. }),
                Command::BeginOcclusionQuery { index: 0, .. },
                Command::EndOcclusionQuery { index: 0, .. },
            ] if range == &(0..2)
        ));
    }

    #[test]
    fn render_pass_sample_counts_match_deko_modes() {
        assert!(supports_sample_count(1));
        assert!(supports_sample_count(2));
        assert!(supports_sample_count(4));
        assert!(supports_sample_count(8));
        assert!(!supports_sample_count(16));
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

fn color_copy_texel_size(format: wgt::TextureFormat) -> DeviceResult<u32> {
    if format.block_dimensions() != (1, 1) {
        return Err(crate::DeviceError::Lost);
    }
    format
        .block_copy_size(Some(wgt::TextureAspect::All))
        .ok_or(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_buffer_to_texture(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &Buffer,
    dst: &TextureInner,
    regions: &[crate::BufferTextureCopy],
) -> DeviceResult<()> {
    let bytes_per_texel = color_copy_texel_size(dst.format())?;
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
                let host_bytes = &*src.get_slice_ptr(region.buffer_layout.offset..trace_end)?;
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

fn timestamp_writes(
    writes: Option<&crate::PassTimestampWrites<Resource>>,
) -> DeviceResult<(Option<TimestampWrite>, Option<TimestampWrite>)> {
    let Some(writes) = writes else {
        return Ok((None, None));
    };
    let Resource::QuerySet(query_set) = writes.query_set else {
        return Err(crate::DeviceError::Lost);
    };
    if !query_set.is_timestamp() {
        return Err(crate::DeviceError::Lost);
    }
    let timestamp = |index: Option<u32>| -> DeviceResult<Option<TimestampWrite>> {
        let Some(index) = index else {
            return Ok(None);
        };
        query_set.validate_range(index..index.checked_add(1).ok_or(crate::DeviceError::Lost)?)?;
        Ok(Some(TimestampWrite {
            query_set: query_set.clone(),
            index,
        }))
    };
    Ok((
        timestamp(writes.beginning_of_pass_write_index)?,
        timestamp(writes.end_of_pass_write_index)?,
    ))
}

fn supports_sample_count(sample_count: u32) -> bool {
    matches!(sample_count, 1 | 2 | 4 | 8)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_timestamp_write(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    timestamp: &TimestampWrite,
) -> DeviceResult<()> {
    let address = timestamp.query_set.report_address(timestamp.index)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, "write_timestamp", |cmdbuf| {
            dk::dkCmdBufReportCounter(cmdbuf, dk::DkCounter::DkCounter_Timestamp, address);
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_timestamp_write(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _timestamp: &TimestampWrite,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_reset_query_results(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    query_set: &QuerySetInner,
    range: Range<u32>,
) -> DeviceResult<()> {
    query_set.validate_range(range.clone())?;
    unsafe {
        submit_deko_commands(queue, surface_queue, "reset_queries", |cmdbuf| {
            for index in range {
                dk::dkCmdBufReportValue(cmdbuf, 0, query_set.report_address(index)?);
            }
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_reset_query_results(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _query_set: &QuerySetInner,
    _range: Range<u32>,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_begin_occlusion_query(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    query_set: &QuerySetInner,
    index: u32,
) -> DeviceResult<()> {
    if !query_set.is_occlusion() {
        return Err(crate::DeviceError::Lost);
    }
    query_set.validate_range(index..index.checked_add(1).ok_or(crate::DeviceError::Lost)?)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, "begin_occlusion_query", |cmdbuf| {
            dk::dkCmdBufResetCounter(cmdbuf, dk::DkCounter::DkCounter_SamplesPassed);
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_begin_occlusion_query(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _query_set: &QuerySetInner,
    _index: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_end_occlusion_query(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    query_set: &QuerySetInner,
    index: u32,
) -> DeviceResult<()> {
    if !query_set.is_occlusion() {
        return Err(crate::DeviceError::Lost);
    }
    let address = query_set.report_address(index)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, "end_occlusion_query", |cmdbuf| {
            dk::dkCmdBufReportCounter(cmdbuf, dk::DkCounter::DkCounter_SamplesPassed, address);
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_end_occlusion_query(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _query_set: &QuerySetInner,
    _index: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_query_results(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    query_set: &QuerySetInner,
    range: Range<u32>,
    buffer: &Buffer,
    offset: wgt::BufferAddress,
    stride: wgt::BufferSize,
) -> DeviceResult<()> {
    query_set.validate_range(range.clone())?;
    if stride.get() < DEKO_QUERY_RESULT_SIZE {
        return Err(crate::DeviceError::Lost);
    }
    let result_size =
        wgt::BufferSize::new(DEKO_QUERY_RESULT_SIZE).ok_or(crate::DeviceError::Lost)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, "copy_query_results", |cmdbuf| {
            for (destination_index, query_index) in range.enumerate() {
                let destination_index =
                    u64::try_from(destination_index).map_err(|_| crate::DeviceError::Lost)?;
                let destination_offset = offset
                    .checked_add(
                        destination_index
                            .checked_mul(stride.get())
                            .ok_or(crate::DeviceError::Lost)?,
                    )
                    .ok_or(crate::DeviceError::Lost)?;
                let source = query_set.report_address(query_index)?;
                let (destination, _) = buffer.gpu_binding(destination_offset, Some(result_size))?;
                dk::dkCmdBufCopyBuffer(cmdbuf, source, destination, DEKO_QUERY_RESULT_SIZE as u32);
            }
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_copy_query_results(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _query_set: &QuerySetInner,
    _range: Range<u32>,
    _buffer: &Buffer,
    _offset: wgt::BufferAddress,
    _stride: wgt::BufferSize,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
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
    let bytes_per_texel = color_copy_texel_size(src.format())?;
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
                let download_end = region
                    .buffer_layout
                    .offset
                    .checked_add(required_bytes)
                    .ok_or(crate::DeviceError::Lost)?;
                downloaded_ranges.push(region.buffer_layout.offset..download_end);
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
            let fingerprint_end = range
                .start
                .checked_add((range.end - range.start).min(256))
                .ok_or(crate::DeviceError::Lost)?;
            let bytes = &*dst.get_slice_ptr(range.start..fingerprint_end)?;
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
    stencil_clear_value: Option<u32>,
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
            if depth_clear_value.is_some() || stencil_clear_value.is_some() {
                dk::dkCmdBufClearDepthStencil(
                    cmdbuf,
                    depth_clear_value.is_some(),
                    depth_clear_value.unwrap_or(1.0),
                    if stencil_clear_value.is_some() {
                        0xff
                    } else {
                        0
                    },
                    stencil_clear_value.unwrap_or(0) as u8,
                );
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
    _stencil_clear_value: Option<u32>,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_end_render_pass(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &mut ExecutionState,
) -> DeviceResult<()> {
    let target = state.target.take().ok_or(crate::DeviceError::Lost)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, "end_render_pass", |cmdbuf| {
            for (source, destination) in target.images.iter().zip(&target.resolve_images) {
                if let (Some(source), Some(destination)) = (source, destination) {
                    dk::dkCmdBufResolveImage(cmdbuf, &source.1, &destination.1);
                }
            }
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_end_render_pass(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    state: &mut ExecutionState,
) -> DeviceResult<()> {
    state.target.take().ok_or(crate::DeviceError::Lost)?;
    Ok(())
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
unsafe fn submit_draw_indirect(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    buffer: &Buffer,
    offset: wgt::BufferAddress,
    draw_count: u32,
) -> DeviceResult<()> {
    let Some(indirect) =
        IndirectDrawRange::new::<dk::DkDrawIndirectData>(buffer, offset, draw_count)?
    else {
        return Ok(());
    };
    unsafe {
        submit_deko_draw(queue, surface_queue, state, |cmdbuf, pipeline| {
            for draw_index in 0..indirect.draw_count {
                let draw_addr = indirect.record_addr(draw_index)?;
                dk::dkCmdBufDrawIndirect(cmdbuf, pipeline.primitive, draw_addr);
            }
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_draw_indexed_indirect(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    buffer: &Buffer,
    offset: wgt::BufferAddress,
    draw_count: u32,
) -> DeviceResult<()> {
    let Some(indirect) =
        IndirectDrawRange::new::<dk::DkDrawIndexedIndirectData>(buffer, offset, draw_count)?
    else {
        return Ok(());
    };
    let index_binding = state
        .index_buffer
        .as_ref()
        .ok_or(crate::DeviceError::Lost)?;
    unsafe {
        submit_deko_draw(queue, surface_queue, state, |cmdbuf, pipeline| {
            let (index_addr, _) = index_binding
                .buffer
                .gpu_binding(index_binding.offset, index_binding.size)?;
            dk::dkCmdBufBindIdxBuffer(cmdbuf, map_index_format(index_binding.format), index_addr);
            for draw_index in 0..indirect.draw_count {
                let draw_addr = indirect.record_addr(draw_index)?;
                dk::dkCmdBufDrawIndexedIndirect(cmdbuf, pipeline.primitive, draw_addr);
            }
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_draw_indirect_count(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    buffer: &Buffer,
    offset: wgt::BufferAddress,
    count_buffer: &Buffer,
    count_offset: wgt::BufferAddress,
    max_count: u32,
) -> DeviceResult<()> {
    let draw_count = read_indirect_draw_count(count_buffer, count_offset, max_count)?;
    unsafe { submit_draw_indirect(queue, surface_queue, state, buffer, offset, draw_count) }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_draw_indexed_indirect_count(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    buffer: &Buffer,
    offset: wgt::BufferAddress,
    count_buffer: &Buffer,
    count_offset: wgt::BufferAddress,
    max_count: u32,
) -> DeviceResult<()> {
    let draw_count = read_indirect_draw_count(count_buffer, count_offset, max_count)?;
    unsafe { submit_draw_indexed_indirect(queue, surface_queue, state, buffer, offset, draw_count) }
}

#[cfg(target_os = "horizon")]
unsafe fn bind_compute_texture_bindings(
    cmdbuf: dk::DkCmdBuf,
    state: &ExecutionState,
    pipeline: &super::ComputePipelineInnerRaw,
) -> DeviceResult<()> {
    let textures = pipeline
        .compute_shader
        .inner
        .bindings
        .iter()
        .filter(|binding| binding.kind == ShaderBindingKind::Texture)
        .collect::<Vec<_>>();
    if textures.is_empty() {
        return Ok(());
    }
    let bound_group = |index: u32| {
        state
            .bind_groups
            .get(index as usize)
            .and_then(Option::as_ref)
            .ok_or(crate::DeviceError::Lost)
    };
    let (image_addr, sampler_addr, image_stride, sampler_stride) = textures
        .iter()
        .find_map(|binding| {
            bound_group(binding.group)
                .ok()
                .and_then(|group| group.group.texture_descriptor_set())
        })
        .ok_or(crate::DeviceError::Lost)?;
    for binding in textures {
        let group = bound_group(binding.group)?;
        unsafe {
            group.group.push_texture_binding(
                cmdbuf,
                image_addr,
                sampler_addr,
                image_stride,
                sampler_stride,
                binding.group,
                binding.binding,
                binding.target,
                dk::DkStage::DkStage_Compute,
            )?;
        }
    }
    unsafe {
        dk::dkCmdBufBindImageDescriptorSet(
            cmdbuf,
            image_addr,
            super::DEKO_TEXTURE_SAMPLER_COUNT as u32,
        );
        dk::dkCmdBufBindSamplerDescriptorSet(
            cmdbuf,
            sampler_addr,
            super::DEKO_TEXTURE_SAMPLER_COUNT as u32,
        );
    }
    Ok(())
}

#[cfg(target_os = "horizon")]
unsafe fn submit_dispatch_workgroups(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    count: [u32; 3],
) -> DeviceResult<()> {
    if !state.in_compute_pass {
        return Err(crate::DeviceError::Lost);
    }
    let pipeline = state
        .compute_pipeline
        .as_ref()
        .ok_or(crate::DeviceError::Lost)?;
    let pipeline = pipeline.raw();
    unsafe {
        submit_deko_commands(queue, surface_queue, "dispatch_workgroups", |cmdbuf| {
            let shader = [pipeline.compute_shader.raw_shader()];
            dk::dkCmdBufBindShaders(
                cmdbuf,
                dk::DkStageFlag_Compute,
                shader.as_ptr(),
                shader.len() as u32,
            );
            pipeline
                .pipeline_layout
                .bind_immediates(cmdbuf, wgt::ShaderStages::COMPUTE)?;
            bind_compute_texture_bindings(cmdbuf, state, pipeline)?;
            for binding in &pipeline.compute_shader.inner.bindings {
                let group = state
                    .bind_groups
                    .get(binding.group as usize)
                    .and_then(Option::as_ref)
                    .ok_or(crate::DeviceError::Lost)?;
                match binding.kind {
                    ShaderBindingKind::Uniform => group.group.bind_uniform_binding(
                        cmdbuf,
                        &group.dynamic_offsets,
                        binding.binding,
                        binding.target,
                        dk::DkStage::DkStage_Compute,
                    )?,
                    ShaderBindingKind::Storage => group.group.bind_storage_binding(
                        cmdbuf,
                        &group.dynamic_offsets,
                        binding.binding,
                        binding.target,
                        dk::DkStage::DkStage_Compute,
                    )?,
                    ShaderBindingKind::StorageTexture => group.group.bind_storage_texture(
                        cmdbuf,
                        &group.dynamic_offsets,
                        binding.binding,
                        binding.target,
                        dk::DkStage::DkStage_Compute,
                    )?,
                    ShaderBindingKind::Texture | ShaderBindingKind::Sampler => {}
                }
            }
            dk::dkCmdBufDispatchCompute(cmdbuf, count[0], count[1], count[2]);
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_dispatch_workgroups_indirect(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    buffer: &Buffer,
    offset: wgt::BufferAddress,
) -> DeviceResult<()> {
    if !state.in_compute_pass {
        return Err(crate::DeviceError::Lost);
    }
    let indirect_size = wgt::BufferSize::new(size_of::<dk::DkDispatchIndirectData>() as u64)
        .ok_or(crate::DeviceError::Lost)?;
    let (dispatch_addr, _) = buffer.gpu_binding(offset, Some(indirect_size))?;
    let pipeline = state
        .compute_pipeline
        .as_ref()
        .ok_or(crate::DeviceError::Lost)?;
    let pipeline = pipeline.raw();
    unsafe {
        submit_deko_commands(
            queue,
            surface_queue,
            "dispatch_workgroups_indirect",
            |cmdbuf| {
                let shader = [pipeline.compute_shader.raw_shader()];
                dk::dkCmdBufBindShaders(
                    cmdbuf,
                    dk::DkStageFlag_Compute,
                    shader.as_ptr(),
                    shader.len() as u32,
                );
                pipeline
                    .pipeline_layout
                    .bind_immediates(cmdbuf, wgt::ShaderStages::COMPUTE)?;
                bind_compute_texture_bindings(cmdbuf, state, pipeline)?;
                for binding in &pipeline.compute_shader.inner.bindings {
                    let group = state
                        .bind_groups
                        .get(binding.group as usize)
                        .and_then(Option::as_ref)
                        .ok_or(crate::DeviceError::Lost)?;
                    match binding.kind {
                        ShaderBindingKind::Uniform => group.group.bind_uniform_binding(
                            cmdbuf,
                            &group.dynamic_offsets,
                            binding.binding,
                            binding.target,
                            dk::DkStage::DkStage_Compute,
                        )?,
                        ShaderBindingKind::Storage => group.group.bind_storage_binding(
                            cmdbuf,
                            &group.dynamic_offsets,
                            binding.binding,
                            binding.target,
                            dk::DkStage::DkStage_Compute,
                        )?,
                        ShaderBindingKind::StorageTexture => group.group.bind_storage_texture(
                            cmdbuf,
                            &group.dynamic_offsets,
                            binding.binding,
                            binding.target,
                            dk::DkStage::DkStage_Compute,
                        )?,
                        ShaderBindingKind::Texture | ShaderBindingKind::Sampler => {}
                    }
                }
                dk::dkCmdBufDispatchComputeIndirect(cmdbuf, dispatch_addr);
                Ok(())
            },
        )
    }
}

#[cfg(target_os = "horizon")]
struct IndirectDrawRange {
    base_addr: dk::DkGpuAddr,
    stride: u64,
    draw_count: u32,
}

#[cfg(target_os = "horizon")]
impl IndirectDrawRange {
    fn new<T>(
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) -> DeviceResult<Option<Self>> {
        if draw_count == 0 {
            return Ok(None);
        }
        let stride = size_of::<T>() as u64;
        let indirect_size = stride
            .checked_mul(u64::from(draw_count))
            .and_then(wgt::BufferSize::new)
            .ok_or(crate::DeviceError::Lost)?;
        let (base_addr, _) = buffer.gpu_binding(offset, Some(indirect_size))?;
        Ok(Some(Self {
            base_addr,
            stride,
            draw_count,
        }))
    }

    fn record_addr(&self, draw_index: u32) -> DeviceResult<dk::DkGpuAddr> {
        let offset = u64::from(draw_index)
            .checked_mul(self.stride)
            .ok_or(crate::DeviceError::Lost)?;
        self.base_addr
            .checked_add(offset)
            .ok_or(crate::DeviceError::Lost)
    }
}

#[cfg(target_os = "horizon")]
fn read_indirect_draw_count(
    count_buffer: &Buffer,
    count_offset: wgt::BufferAddress,
    max_count: u32,
) -> DeviceResult<u32> {
    Ok(count_buffer.read_u32(count_offset)?.min(max_count))
}

#[cfg(target_os = "horizon")]
unsafe fn submit_deko_draw(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &ExecutionState,
    mut draw: impl FnMut(dk::DkCmdBuf, &super::RenderPipelineInnerRaw) -> DeviceResult<()>,
) -> DeviceResult<()> {
    let target = state.target.as_ref().ok_or(crate::DeviceError::Lost)?;
    let pipeline = state.pipeline.as_ref().ok_or(crate::DeviceError::Lost)?;
    let pipeline = pipeline.raw();
    if target.multiview_mask != pipeline.multiview_mask {
        return Err(crate::DeviceError::Lost);
    }
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
            pipeline
                .pipeline_layout
                .bind_immediates(cmdbuf, wgt::ShaderStages::VERTEX_FRAGMENT)?;
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
                        dk::DkStage::DkStage_Fragment,
                    )?;
                }
                dk::dkCmdBufBindImageDescriptorSet(
                    cmdbuf,
                    image_addr,
                    super::DEKO_TEXTURE_SAMPLER_COUNT as u32,
                );
                dk::dkCmdBufBindSamplerDescriptorSet(
                    cmdbuf,
                    sampler_addr,
                    super::DEKO_TEXTURE_SAMPLER_COUNT as u32,
                );
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
                for binding in bindings
                    .iter()
                    .filter(|binding| binding.kind == ShaderBindingKind::Storage)
                {
                    let group = bound_group(binding.group)?;
                    group.group.bind_storage_binding(
                        cmdbuf,
                        &group.dynamic_offsets,
                        binding.binding,
                        binding.target,
                        stage,
                    )?;
                }
                for binding in bindings
                    .iter()
                    .filter(|binding| binding.kind == ShaderBindingKind::StorageTexture)
                {
                    let group = bound_group(binding.group)?;
                    group.group.bind_storage_texture(
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
            dk::dkCmdBufBindMultisampleState(cmdbuf, &pipeline.multisample_state);
            dk::dkCmdBufSetSampleMask(cmdbuf, pipeline.sample_mask);
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
            if let Some(mask) = pipeline.multiview_mask {
                let buffer = pipeline
                    .multiview_buffer
                    .as_ref()
                    .ok_or(crate::DeviceError::Lost)?;
                let (gpu_addr, gpu_size) = buffer.gpu_binding(0, None)?;
                dk::dkCmdBufBindUniformBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Vertex,
                    super::DEKO_MULTIVIEW_BINDING,
                    gpu_addr,
                    gpu_size,
                );
                for view_index in active_multiview_indices(mask) {
                    dk::dkCmdBufPushConstants(
                        cmdbuf,
                        gpu_addr,
                        gpu_size,
                        0,
                        core::mem::size_of::<u32>() as u32,
                        ptr::from_ref(&view_index).cast(),
                    );
                    draw(cmdbuf, pipeline)?;
                }
                Ok(())
            } else {
                draw(cmdbuf, pipeline)
            }
        })
    }
}

fn active_multiview_indices(mask: core::num::NonZeroU32) -> impl Iterator<Item = u32> {
    let bits = mask.get();
    (0..u32::BITS).filter(move |index| bits & (1 << index) != 0)
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

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_draw_indirect(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _buffer: &Buffer,
    _offset: wgt::BufferAddress,
    _draw_count: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_draw_indexed_indirect(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _buffer: &Buffer,
    _offset: wgt::BufferAddress,
    _draw_count: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_draw_indirect_count(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _buffer: &Buffer,
    _offset: wgt::BufferAddress,
    _count_buffer: &Buffer,
    _count_offset: wgt::BufferAddress,
    _max_count: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_draw_indexed_indirect_count(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _buffer: &Buffer,
    _offset: wgt::BufferAddress,
    _count_buffer: &Buffer,
    _count_offset: wgt::BufferAddress,
    _max_count: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_dispatch_workgroups(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _count: [u32; 3],
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_dispatch_workgroups_indirect(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &ExecutionState,
    _buffer: &Buffer,
    _offset: wgt::BufferAddress,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_set_immediates(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    layout: &super::PipelineLayoutInner,
    offset_bytes: u32,
    data: &[u32],
) -> DeviceResult<()> {
    unsafe {
        submit_deko_commands(queue, surface_queue, "set_immediates", |cmdbuf| {
            layout.push_immediates(cmdbuf, offset_bytes, data)
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_set_immediates(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _layout: &super::PipelineLayoutInner,
    _offset_bytes: u32,
    _data: &[u32],
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
