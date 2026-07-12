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
use super::{map_texture_image_format, DEKO_QUERY_RESULT_SIZE};
use super::{
    Api, BindGroupInner, Buffer, ComputePipelineInner, DeviceResult, QuerySetInner, Queue,
    RawImage, RawQueueHandle, RenderPipelineInner, Resource, TextureInner,
    DEKO_COLOR_ATTACHMENT_COUNT,
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
    BeginPipelineStatisticsQuery {
        query_set: alloc::sync::Arc<QuerySetInner>,
        index: u32,
    },
    EndPipelineStatisticsQuery {
        query_set: alloc::sync::Arc<QuerySetInner>,
        index: u32,
    },
    WriteTimestamp(TimestampWrite),
    ResourceBarrier {
        invalidate_flags: u32,
    },
    BeginRenderPass {
        extent: wgt::Extent3d,
        colors: Vec<Option<ColorAttachmentState>>,
        depth_stencil: DepthStencilAttachmentState,
        beginning_timestamp: Option<TimestampWrite>,
        end_timestamp: Option<TimestampWrite>,
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
    SetScissorRect {
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
    Error,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "horizon"), allow(dead_code))]
struct TimestampWrite {
    query_set: alloc::sync::Arc<QuerySetInner>,
    index: u32,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct ColorAttachmentState {
    view: AttachmentViewState,
    extent: wgt::Extent3d,
    sample_count: u32,
    clear_value: Option<wgt::Color>,
    resolve_view: Option<AttachmentViewState>,
    discard: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
struct DepthStencilAttachmentState {
    view: Option<AttachmentViewState>,
    extent: Option<wgt::Extent3d>,
    depth_clear_value: Option<f32>,
    stencil_clear_value: Option<u8>,
    discard: bool,
}

impl DepthStencilAttachmentState {
    #[allow(dead_code)]
    fn has_clear(self) -> bool {
        self.depth_clear_value.is_some() || self.stencil_clear_value.is_some()
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct AttachmentViewState {
    image: RawImage,
    format: wgt::TextureFormat,
    aspect: wgt::TextureAspect,
    base_mip_level: u8,
    base_array_layer: u16,
}

impl AttachmentViewState {
    fn from_single_subresource(view: &Resource) -> DeviceResult<(Self, wgt::Extent3d, u32)> {
        let Resource::TextureView {
            image,
            format,
            aspect,
            extent,
            sample_count,
            base_mip_level,
            mip_level_count,
            base_array_layer,
            array_layer_count,
            ..
        } = view
        else {
            return Err(crate::DeviceError::Lost);
        };

        if *mip_level_count != 1 || *array_layer_count != 1 {
            return Err(crate::DeviceError::Lost);
        }

        Ok((
            Self {
                image: *image,
                format: *format,
                aspect: *aspect,
                base_mip_level: *base_mip_level,
                base_array_layer: *base_array_layer,
            },
            *extent,
            *sample_count,
        ))
    }

    #[cfg(target_os = "horizon")]
    fn deko_view(self) -> dk::DkImageView {
        let mut view = dk::DkImageView::defaults(self.image.0);
        view.format = map_texture_image_format(self.format)
            .expect("texture view format was validated at creation");
        if self.aspect == wgt::TextureAspect::StencilOnly
            || self.format == wgt::TextureFormat::Stencil8
        {
            view.dsSource = dk::DkDsSource::DkDsSource_Stencil;
        }
        view.mipLevelOffset = self.base_mip_level;
        view.mipLevelCount = 1;
        view.layerOffset = self.base_array_layer;
        view.layerCount = 1;
        view
    }
}

#[derive(Default)]
struct ExecutionState {
    target: Option<RenderTarget>,
    pipeline: Option<alloc::sync::Arc<RenderPipelineInner>>,
    compute_pipeline: Option<alloc::sync::Arc<ComputePipelineInner>>,
    in_compute_pass: bool,
    render_pass_end_timestamp: Option<TimestampWrite>,
    compute_pass_end_timestamp: Option<TimestampWrite>,
    vertex_buffers: Vec<Option<VertexBinding>>,
    index_buffer: Option<IndexBinding>,
    bind_groups: Vec<Option<BoundBindGroup>>,
    viewport: Option<ViewportState>,
    scissor: Option<ScissorState>,
    stencil_reference: u8,
    blend_constants: [f32; 4],
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct RenderTarget {
    extent: wgt::Extent3d,
    colors: Vec<RenderTargetColor>,
    depth_stencil_discard: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct RenderTargetColor {
    target_id: u32,
    view: AttachmentViewState,
    resolve_view: Option<AttachmentViewState>,
    discard: bool,
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

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct BoundBindGroup {
    group: alloc::sync::Arc<BindGroupInner>,
    dynamic_offsets: Vec<wgt::DynamicOffset>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ViewportState {
    rect: crate::Rect<f32>,
    depth_range: Range<f32>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ScissorState {
    rect: crate::Rect<u32>,
}

impl CommandBuffer {
    fn record_unsupported(&mut self) {
        self.commands.push(Command::Error);
    }

    /// # Safety
    ///
    /// Must be called with appropriate synchronization for the resources affected by the command,
    /// such as ensuring that buffers are not accessed by a command while aliasing references exist.
    pub(crate) unsafe fn execute(
        &self,
        queue: &Queue,
        surface_queue: Option<RawQueueHandle>,
    ) -> DeviceResult<()> {
        let mut state = ExecutionState::default();
        for command in &self.commands {
            unsafe { command.execute(queue, surface_queue, &mut state) }?;
        }
        Ok(())
    }

    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl crate::CommandEncoder for CommandBuffer {
    type A = Api;

    unsafe fn begin_encoding(&mut self, label: crate::Label) -> DeviceResult<()> {
        assert!(self.commands.is_empty());
        Ok(())
    }
    unsafe fn discard_encoding(&mut self) {
        self.commands.clear();
    }
    unsafe fn end_encoding(&mut self) -> DeviceResult<CommandBuffer> {
        Ok(CommandBuffer {
            commands: mem::take(&mut self.commands),
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
        _src_usage: wgt::TextureUses,
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
        if query_set.validate_range(index..end).is_err() {
            self.record_unsupported();
            return;
        }
        if query_set.is_occlusion() {
            self.commands.push(Command::BeginOcclusionQuery {
                query_set: query_set.clone(),
                index,
            });
        } else if query_set.is_pipeline_statistics() {
            self.commands.push(Command::BeginPipelineStatisticsQuery {
                query_set: query_set.clone(),
                index,
            });
        } else {
            self.record_unsupported();
        }
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
        if query_set.validate_range(index..end).is_err() {
            self.record_unsupported();
            return;
        }
        if query_set.is_occlusion() {
            self.commands.push(Command::EndOcclusionQuery {
                query_set: query_set.clone(),
                index,
            });
        } else if query_set.is_pipeline_statistics() {
            self.commands.push(Command::EndPipelineStatisticsQuery {
                query_set: query_set.clone(),
                index,
            });
        } else {
            self.record_unsupported();
        }
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
        _acceleration_structure: &Resource,
        _buf: &Buffer,
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
        if desc.multiview_mask.is_some()
            || !supports_render_pass_occlusion_query_set(desc.occlusion_query_set)
            || !supports_sample_count(desc.sample_count)
        {
            return Err(crate::DeviceError::Lost);
        }

        let colors = color_attachments(desc.color_attachments, desc.sample_count)?;
        let depth_stencil = depth_stencil_attachment(
            desc.depth_stencil_attachment.as_ref(),
            colors.iter().flatten().next().map(|color| color.extent),
            desc.sample_count,
        )?;
        let extent = render_pass_extent(&colors, depth_stencil)?;
        let (beginning_timestamp, end_timestamp) =
            timestamp_writes(desc.timestamp_writes.as_ref())?;
        self.commands.push(Command::BeginRenderPass {
            extent,
            colors,
            depth_stencil,
            beginning_timestamp,
            end_timestamp,
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
        self.commands.push(Command::SetViewport {
            rect: rect.clone(),
            depth_range,
        });
    }
    unsafe fn set_scissor_rect(&mut self, rect: &crate::Rect<u32>) {
        self.commands
            .push(Command::SetScissorRect { rect: rect.clone() });
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
            Err(_) => self.commands.push(Command::Error),
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

    unsafe fn dispatch_workgroups(&mut self, count: [u32; 3]) {
        self.commands.push(Command::DispatchWorkgroups { count });
    }
    unsafe fn dispatch_workgroups_indirect(&mut self, buffer: &Buffer, offset: wgt::BufferAddress) {
        self.commands.push(Command::DispatchWorkgroupsIndirect {
            buffer: buffer.clone(),
            offset,
        });
    }

    unsafe fn begin_ray_tracing_pass(&mut self, _desc: &crate::RayTracingPassDescriptor) {
        self.record_unsupported();
    }
    unsafe fn end_ray_tracing_pass(&mut self) {
        self.record_unsupported();
    }
    unsafe fn set_ray_tracing_pipeline(&mut self, _pipeline: &Resource) {
        self.record_unsupported();
    }
    unsafe fn trace_rays(
        &mut self,
        _count: [u32; 3],
        _ray_generation_group_data: crate::PipelineGroupData<Buffer>,
        _miss_group_data: crate::PipelineGroupData<Buffer>,
        _intersection_group_data: crate::PipelineGroupData<Buffer>,
    ) {
        self.record_unsupported();
    }

    unsafe fn build_acceleration_structures<'a, T>(
        &mut self,
        _descriptor_count: u32,
        _descriptors: T,
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
        _src: &Resource,
        _dst: &Resource,
        _copy: wgt::AccelerationStructureCopy,
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
                #[cfg(target_os = "horizon")]
                {
                    unsafe { submit_clear_buffer(queue, surface_queue, buffer, range.clone()) }
                }
                #[cfg(not(target_os = "horizon"))]
                {
                    // SAFETY:
                    // Caller is responsible for ensuring this does not alias.
                    let buffer_slice: &mut [u8] =
                        unsafe { &mut *buffer.get_slice_ptr(range.clone()) };
                    buffer_slice.fill(0);
                    upload_after_host_write(buffer)?;
                    Ok(())
                }
            }

            Command::CopyBufferToBuffer { src, dst, regions } => {
                #[cfg(target_os = "horizon")]
                {
                    unsafe { submit_copy_buffer_to_buffer(queue, surface_queue, src, dst, regions) }
                }

                #[cfg(not(target_os = "horizon"))]
                {
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
                    upload_after_host_write(dst)?;
                    Ok(())
                }
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
            Command::ResetQueries { query_set, range } => unsafe {
                submit_reset_query_results(queue, surface_queue, query_set, range.clone())
            },
            Command::BeginOcclusionQuery { query_set, index } => unsafe {
                submit_begin_occlusion_query(queue, surface_queue, query_set, *index)
            },
            Command::EndOcclusionQuery { query_set, index } => unsafe {
                submit_end_occlusion_query(queue, surface_queue, query_set, *index)
            },
            Command::BeginPipelineStatisticsQuery { query_set, index } => unsafe {
                submit_begin_pipeline_statistics_query(queue, surface_queue, query_set, *index)
            },
            Command::EndPipelineStatisticsQuery { query_set, index } => unsafe {
                submit_end_pipeline_statistics_query(queue, surface_queue, query_set, *index)
            },
            Command::WriteTimestamp(timestamp) => unsafe {
                submit_timestamp_write(queue, surface_queue, timestamp)
            },
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
            Command::ResourceBarrier { invalidate_flags } => unsafe {
                submit_resource_barrier(queue, surface_queue, *invalidate_flags)
            },
            Command::BeginRenderPass {
                extent,
                colors,
                depth_stencil,
                beginning_timestamp,
                end_timestamp,
            } => {
                state.target = Some(RenderTarget {
                    extent: *extent,
                    colors: colors
                        .iter()
                        .enumerate()
                        .filter_map(|(index, color)| {
                            color.as_ref().map(|color| RenderTargetColor {
                                target_id: index as u32,
                                view: color.view,
                                resolve_view: color.resolve_view,
                                discard: color.discard,
                            })
                        })
                        .collect(),
                    depth_stencil_discard: depth_stencil.discard,
                });
                state.viewport = None;
                state.scissor = None;
                state.stencil_reference = 0;
                state.blend_constants = [0.0; 4];
                unsafe {
                    submit_begin_render_pass(queue, surface_queue, *extent, colors, *depth_stencil)
                }?;
                state.render_pass_end_timestamp = end_timestamp.clone();
                if let Some(timestamp) = beginning_timestamp {
                    unsafe { submit_timestamp_write(queue, surface_queue, timestamp) }?;
                }
                Ok(())
            }
            Command::EndRenderPass => {
                if let Some(timestamp) = state.render_pass_end_timestamp.take() {
                    unsafe { submit_timestamp_write(queue, surface_queue, &timestamp) }?;
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
                    unsafe { submit_timestamp_write(queue, surface_queue, timestamp) }?;
                }
                Ok(())
            }
            Command::EndComputePass => {
                if !state.in_compute_pass {
                    return Err(crate::DeviceError::Lost);
                }
                if let Some(timestamp) = state.compute_pass_end_timestamp.take() {
                    unsafe { submit_timestamp_write(queue, surface_queue, &timestamp) }?;
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
                state.viewport = Some(ViewportState {
                    rect: rect.clone(),
                    depth_range: depth_range.clone(),
                });
                Ok(())
            }
            Command::SetScissorRect { rect } => {
                state.scissor = Some(ScissorState { rect: rect.clone() });
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
            Command::Error => Err(crate::DeviceError::Lost),
        }
    }
}

#[cfg(not(target_os = "horizon"))]
fn upload_after_host_write(buffer: &Buffer) -> DeviceResult<()> {
    let _ = buffer;
    Ok(())
}

fn supports_render_pass_occlusion_query_set(query_set: Option<&Resource>) -> bool {
    match query_set {
        None => true,
        Some(Resource::QuerySet(query_set)) => query_set.is_occlusion(),
        Some(_) => false,
    }
}

fn color_attachments(
    color_attachments: &[Option<crate::ColorAttachment<'_, Resource>>],
    sample_count: u32,
) -> DeviceResult<Vec<Option<ColorAttachmentState>>> {
    if color_attachments.len() > DEKO_COLOR_ATTACHMENT_COUNT as usize {
        return Err(crate::DeviceError::Lost);
    }
    let mut colors: Vec<Option<ColorAttachmentState>> = Vec::with_capacity(color_attachments.len());
    for attachment in color_attachments {
        let Some(attachment) = attachment else {
            colors.push(None);
            continue;
        };
        let color = color_attachment(attachment, sample_count)?;
        if let Some(first) = colors.iter().flatten().next() {
            if color.extent != first.extent {
                return Err(crate::DeviceError::Lost);
            }
        }
        colors.push(Some(color));
    }
    Ok(colors)
}

fn color_attachment(
    attachment: &crate::ColorAttachment<'_, Resource>,
    sample_count: u32,
) -> DeviceResult<ColorAttachmentState> {
    if attachment.depth_slice.is_some() {
        return Err(crate::DeviceError::Lost);
    }
    let (view, extent, target_sample_count) =
        AttachmentViewState::from_single_subresource(attachment.target.view)?;
    if target_sample_count != sample_count {
        return Err(crate::DeviceError::Lost);
    }
    let resolve_view = resolve_attachment(attachment.resolve_target.as_ref(), extent)?;
    if sample_count == 1 && resolve_view.is_some() {
        return Err(crate::DeviceError::Lost);
    }
    validate_attachment_store(attachment.ops)?;
    Ok(ColorAttachmentState {
        view,
        extent,
        sample_count,
        clear_value: attachment_clear_value(attachment.ops, attachment.clear_value)?,
        resolve_view,
        discard: attachment_should_discard(attachment.ops),
    })
}

fn resolve_attachment(
    attachment: Option<&crate::Attachment<'_, Resource>>,
    expected_extent: wgt::Extent3d,
) -> DeviceResult<Option<AttachmentViewState>> {
    let Some(attachment) = attachment else {
        return Ok(None);
    };
    if !attachment.usage.contains(wgt::TextureUses::COLOR_TARGET) {
        return Err(crate::DeviceError::Lost);
    }
    let (view, extent, sample_count) =
        AttachmentViewState::from_single_subresource(attachment.view)?;
    if extent != expected_extent || sample_count != 1 {
        return Err(crate::DeviceError::Lost);
    }
    Ok(Some(view))
}

fn depth_stencil_attachment(
    attachment: Option<&crate::DepthStencilAttachment<'_, Resource>>,
    expected_extent: Option<wgt::Extent3d>,
    expected_sample_count: u32,
) -> DeviceResult<DepthStencilAttachmentState> {
    let Some(attachment) = attachment else {
        return Ok(DepthStencilAttachmentState::default());
    };
    if !attachment
        .target
        .usage
        .contains(depth_stencil_attachment_usage(
            attachment.depth_ops,
            attachment.stencil_ops,
        ))
    {
        return Err(crate::DeviceError::Lost);
    }
    let (view, extent, sample_count) =
        AttachmentViewState::from_single_subresource(attachment.target.view)?;
    if expected_extent.is_some_and(|expected| extent != expected)
        || sample_count != expected_sample_count
    {
        return Err(crate::DeviceError::Lost);
    }

    let depth_clear_value = if attachment.depth_ops.is_empty() {
        None
    } else {
        validate_attachment_store(attachment.depth_ops)?;
        attachment_clear_value(attachment.depth_ops, attachment.clear_value.0)?
    };
    let stencil_clear_value = if attachment.stencil_ops.is_empty() {
        None
    } else {
        validate_attachment_store(attachment.stencil_ops)?;
        attachment_clear_value(attachment.stencil_ops, attachment.clear_value.1 as u8)?
    };
    Ok(DepthStencilAttachmentState {
        view: Some(view),
        extent: Some(extent),
        depth_clear_value,
        stencil_clear_value,
        discard: attachment_should_discard(attachment.depth_ops)
            || attachment_should_discard(attachment.stencil_ops),
    })
}

fn depth_stencil_attachment_usage(
    depth_ops: crate::AttachmentOps,
    stencil_ops: crate::AttachmentOps,
) -> wgt::TextureUses {
    if depth_ops.is_empty() && stencil_ops.is_empty() {
        wgt::TextureUses::DEPTH_STENCIL_READ
    } else {
        wgt::TextureUses::DEPTH_STENCIL_WRITE
    }
}

fn render_pass_extent(
    colors: &[Option<ColorAttachmentState>],
    depth_stencil: DepthStencilAttachmentState,
) -> DeviceResult<wgt::Extent3d> {
    colors
        .first()
        .and_then(|color| color.as_ref())
        .map(|color| color.extent)
        .or_else(|| colors.iter().flatten().next().map(|color| color.extent))
        .or(depth_stencil.extent)
        .ok_or(crate::DeviceError::Lost)
}

fn attachment_clear_value<T: Copy>(ops: crate::AttachmentOps, value: T) -> DeviceResult<Option<T>> {
    if ops.contains(crate::AttachmentOps::LOAD_CLEAR) {
        return Ok(Some(value));
    }
    validate_attachment_load(ops)?;
    Ok(None)
}

fn validate_attachment_store(ops: crate::AttachmentOps) -> DeviceResult<()> {
    if !ops.intersects(crate::AttachmentOps::STORE | crate::AttachmentOps::STORE_DISCARD) {
        return Err(crate::DeviceError::Lost);
    }
    Ok(())
}

fn attachment_should_discard(ops: crate::AttachmentOps) -> bool {
    ops.contains(crate::AttachmentOps::STORE_DISCARD)
}

fn validate_attachment_load(ops: crate::AttachmentOps) -> DeviceResult<()> {
    if ops.contains(crate::AttachmentOps::LOAD)
        || ops.contains(crate::AttachmentOps::LOAD_DONT_CARE)
    {
        return Ok(());
    }
    Err(crate::DeviceError::Lost)
}

fn supports_sample_count(sample_count: u32) -> bool {
    matches!(sample_count, 1 | 2 | 4 | 8)
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

#[cfg(target_os = "horizon")]
unsafe fn submit_resource_barrier(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    invalidate_flags: u32,
) -> DeviceResult<()> {
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            dk::dkCmdBufBarrier(cmdbuf, dk::DkBarrier::DkBarrier_Full, invalidate_flags);
            Ok(())
        })
    }
}

#[cfg(target_os = "horizon")]
unsafe fn submit_timestamp_write(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    timestamp: &TimestampWrite,
) -> DeviceResult<()> {
    let address = timestamp.query_set.report_address(timestamp.index, 0)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
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
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for index in range {
                for component in 0..query_set.result_count() {
                    dk::dkCmdBufReportValue(cmdbuf, 0, query_set.report_address(index, component)?);
                }
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
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
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
    let address = query_set.report_address(index, 0)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
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
unsafe fn submit_begin_pipeline_statistics_query(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    query_set: &QuerySetInner,
    index: u32,
) -> DeviceResult<()> {
    let statistics = query_set
        .pipeline_statistics()
        .filter(|statistics| QuerySetInner::supports_pipeline_statistics(*statistics))
        .ok_or(crate::DeviceError::Lost)?;
    query_set.validate_range(index..index.checked_add(1).ok_or(crate::DeviceError::Lost)?)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for (statistic, counter) in pipeline_statistics_counters() {
                if statistics.contains(statistic) {
                    dk::dkCmdBufResetCounter(cmdbuf, counter);
                }
            }
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_begin_pipeline_statistics_query(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _query_set: &QuerySetInner,
    _index: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_end_pipeline_statistics_query(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    query_set: &QuerySetInner,
    index: u32,
) -> DeviceResult<()> {
    let statistics = query_set
        .pipeline_statistics()
        .filter(|statistics| QuerySetInner::supports_pipeline_statistics(*statistics))
        .ok_or(crate::DeviceError::Lost)?;
    query_set.validate_range(index..index.checked_add(1).ok_or(crate::DeviceError::Lost)?)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            let mut component = 0;
            for (statistic, counter) in pipeline_statistics_counters() {
                if statistics.contains(statistic) {
                    let address = query_set.report_address(index, component)?;
                    dk::dkCmdBufReportCounter(cmdbuf, counter, address);
                    component += 1;
                }
            }
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_end_pipeline_statistics_query(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _query_set: &QuerySetInner,
    _index: u32,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
fn pipeline_statistics_counters() -> [(wgt::PipelineStatisticsTypes, dk::DkCounter); 4] {
    [
        (
            wgt::PipelineStatisticsTypes::VERTEX_SHADER_INVOCATIONS,
            dk::DkCounter::DkCounter_VertexShaderInvocations,
        ),
        (
            wgt::PipelineStatisticsTypes::CLIPPER_INVOCATIONS,
            dk::DkCounter::DkCounter_ClipperInputPrimitives,
        ),
        (
            wgt::PipelineStatisticsTypes::CLIPPER_PRIMITIVES_OUT,
            dk::DkCounter::DkCounter_ClipperOutputPrimitives,
        ),
        (
            wgt::PipelineStatisticsTypes::FRAGMENT_SHADER_INVOCATIONS,
            dk::DkCounter::DkCounter_FragmentShaderInvocations,
        ),
    ]
}

#[cfg(target_os = "horizon")]
unsafe fn submit_clear_buffer(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    dst: &Buffer,
    range: Range<wgt::BufferAddress>,
) -> DeviceResult<()> {
    let mut dst_offset = range.start;
    let mut remaining = range
        .end
        .checked_sub(range.start)
        .ok_or(crate::DeviceError::Lost)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            while remaining != 0 {
                let chunk = remaining.min(super::DEKO_CLEAR_BUFFER_SIZE);
                let chunk_size = wgt::BufferSize::new(chunk).ok_or(crate::DeviceError::Lost)?;
                let (src_addr, _) = queue.device.clear_buffer.gpu_binding(0, Some(chunk_size))?;
                let (dst_addr, _) = dst.gpu_binding(dst_offset, Some(chunk_size))?;
                dk::dkCmdBufCopyBuffer(cmdbuf, src_addr, dst_addr, chunk as u32);
                dst_offset = dst_offset
                    .checked_add(chunk)
                    .ok_or(crate::DeviceError::Lost)?;
                remaining -= chunk;
            }
            Ok(())
        })
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
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for region in regions {
                let mut src_offset = region.src_offset;
                let mut dst_offset = region.dst_offset;
                let mut remaining = region.size.get();
                while remaining != 0 {
                    let chunk = remaining.min(u64::from(u32::MAX));
                    let chunk_size = wgt::BufferSize::new(chunk).ok_or(crate::DeviceError::Lost)?;
                    let (src_addr, _) = src.gpu_binding(src_offset, Some(chunk_size))?;
                    let (dst_addr, _) = dst.gpu_binding(dst_offset, Some(chunk_size))?;
                    dk::dkCmdBufCopyBuffer(cmdbuf, src_addr, dst_addr, chunk as u32);
                    src_offset = src_offset
                        .checked_add(chunk)
                        .ok_or(crate::DeviceError::Lost)?;
                    dst_offset = dst_offset
                        .checked_add(chunk)
                        .ok_or(crate::DeviceError::Lost)?;
                    remaining -= chunk;
                }
            }
            Ok(())
        })
    }
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
    let result_count = u64::from(query_set.result_count());
    let query_result_size = result_count
        .checked_mul(DEKO_QUERY_RESULT_SIZE)
        .ok_or(crate::DeviceError::Lost)?;
    if stride.get() < query_result_size {
        return Err(crate::DeviceError::Lost);
    }
    let result_size =
        wgt::BufferSize::new(DEKO_QUERY_RESULT_SIZE).ok_or(crate::DeviceError::Lost)?;
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
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
                for component in 0..query_set.result_count() {
                    let component_offset = u64::from(component)
                        .checked_mul(DEKO_QUERY_RESULT_SIZE)
                        .ok_or(crate::DeviceError::Lost)?;
                    let destination_offset = destination_offset
                        .checked_add(component_offset)
                        .ok_or(crate::DeviceError::Lost)?;
                    let source = query_set.report_address(query_index, component)?;
                    let (destination, _) =
                        buffer.gpu_binding(destination_offset, Some(result_size))?;
                    dk::dkCmdBufCopyBuffer(
                        cmdbuf,
                        source,
                        destination,
                        DEKO_QUERY_RESULT_SIZE as u32,
                    );
                }
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

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_resource_barrier(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _invalidate_flags: u32,
) -> DeviceResult<()> {
    Ok(())
}

#[cfg(target_os = "horizon")]
fn texture_copy_rect(
    texture: &TextureInner,
    base: &crate::TextureCopyBase,
    size: crate::CopyExtent,
) -> DeviceResult<dk::DkImageRect> {
    if base.aspect != copy_format_aspect(texture.format())? || size.depth == 0 {
        return Err(crate::DeviceError::Lost);
    }

    let extent = texture.mip_extent(base.mip_level)?;
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
    if end_x > extent.width || end_y > extent.height {
        return Err(crate::DeviceError::Lost);
    }

    let z = if texture.dimension() == wgt::TextureDimension::D3 {
        if base.array_layer != 0 {
            return Err(crate::DeviceError::Lost);
        }
        let end_z = base
            .origin
            .z
            .checked_add(size.depth)
            .ok_or(crate::DeviceError::Lost)?;
        if end_z > extent.depth_or_array_layers {
            return Err(crate::DeviceError::Lost);
        }
        base.origin.z
    } else {
        if base.origin.z != 0 {
            return Err(crate::DeviceError::Lost);
        }
        let end_layer = base
            .array_layer
            .checked_add(size.depth)
            .ok_or(crate::DeviceError::Lost)?;
        if end_layer > extent.depth_or_array_layers {
            return Err(crate::DeviceError::Lost);
        }
        0
    };

    Ok(dk::DkImageRect {
        x: base.origin.x,
        y: base.origin.y,
        z,
        width: size.width,
        height: size.height,
        depth: size.depth,
    })
}

#[cfg(target_os = "horizon")]
fn texture_copy_view_type(
    texture: &TextureInner,
    extent: wgt::Extent3d,
) -> Option<dk::DkImageType> {
    match texture.dimension() {
        wgt::TextureDimension::D1 => Some(dk::DkImageType::DkImageType_1D),
        wgt::TextureDimension::D2 if extent.depth_or_array_layers > 1 => {
            Some(dk::DkImageType::DkImageType_2DArray)
        }
        wgt::TextureDimension::D3 => Some(dk::DkImageType::DkImageType_3D),
        _ => None,
    }
}

#[cfg(any(target_os = "horizon", test))]
fn copy_format_aspect(format: wgt::TextureFormat) -> DeviceResult<crate::FormatAspects> {
    match format {
        wgt::TextureFormat::Rgba8Unorm
        | wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Rgba8Uint
        | wgt::TextureFormat::Rgba8Sint
        | wgt::TextureFormat::R8Unorm
        | wgt::TextureFormat::Rg8Unorm
        | wgt::TextureFormat::R8Uint
        | wgt::TextureFormat::R8Sint
        | wgt::TextureFormat::Rg8Uint
        | wgt::TextureFormat::Rg8Sint
        | wgt::TextureFormat::R8Snorm
        | wgt::TextureFormat::Rg8Snorm
        | wgt::TextureFormat::Rgba8Snorm
        | wgt::TextureFormat::Bgra8Unorm
        | wgt::TextureFormat::Bgra8UnormSrgb
        | wgt::TextureFormat::R16Float
        | wgt::TextureFormat::Rg16Float
        | wgt::TextureFormat::R16Unorm
        | wgt::TextureFormat::R16Snorm
        | wgt::TextureFormat::Rg16Unorm
        | wgt::TextureFormat::Rg16Snorm
        | wgt::TextureFormat::R16Uint
        | wgt::TextureFormat::R16Sint
        | wgt::TextureFormat::Rg16Uint
        | wgt::TextureFormat::Rg16Sint
        | wgt::TextureFormat::Rgba16Unorm
        | wgt::TextureFormat::Rgba16Snorm
        | wgt::TextureFormat::Rgba16Uint
        | wgt::TextureFormat::Rgba16Sint
        | wgt::TextureFormat::R32Float
        | wgt::TextureFormat::R32Uint
        | wgt::TextureFormat::R32Sint
        | wgt::TextureFormat::Rg32Float
        | wgt::TextureFormat::Rg32Uint
        | wgt::TextureFormat::Rg32Sint
        | wgt::TextureFormat::Rgba32Float
        | wgt::TextureFormat::Rgba32Uint
        | wgt::TextureFormat::Rgba32Sint
        | wgt::TextureFormat::Rgba16Float => Ok(crate::FormatAspects::COLOR),
        wgt::TextureFormat::Depth16Unorm | wgt::TextureFormat::Depth32Float => {
            Ok(crate::FormatAspects::DEPTH)
        }
        wgt::TextureFormat::Stencil8 => Ok(crate::FormatAspects::STENCIL),
        _ => Err(crate::DeviceError::Lost),
    }
}

#[cfg(target_os = "horizon")]
fn texture_copy_view(
    texture: &TextureInner,
    base: &crate::TextureCopyBase,
    layer_count: u32,
) -> DeviceResult<dk::DkImageView> {
    if base.mip_level >= texture.mip_level_count() || layer_count == 0 {
        return Err(crate::DeviceError::Lost);
    }
    let extent = texture.mip_extent(base.mip_level)?;
    let mut view = dk::DkImageView::defaults(texture.raw_image().0);
    if let Some(view_type) = texture_copy_view_type(texture, extent) {
        view.type_ = view_type;
    }
    view.mipLevelOffset = u8::try_from(base.mip_level).map_err(|_| crate::DeviceError::Lost)?;
    view.mipLevelCount = 1;
    if texture.dimension() != wgt::TextureDimension::D3 {
        view.layerOffset = u16::try_from(base.array_layer).map_err(|_| crate::DeviceError::Lost)?;
        view.layerCount = u16::try_from(layer_count).map_err(|_| crate::DeviceError::Lost)?;
    }
    if texture.format() == wgt::TextureFormat::Stencil8 {
        view.dsSource = dk::DkDsSource::DkDsSource_Stencil;
    }
    Ok(view)
}

#[cfg(target_os = "horizon")]
fn copy_texel_size(format: wgt::TextureFormat) -> DeviceResult<u32> {
    match format {
        wgt::TextureFormat::Rgba8Unorm
        | wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Rgba8Uint
        | wgt::TextureFormat::Rgba8Sint
        | wgt::TextureFormat::Rgba8Snorm
        | wgt::TextureFormat::Bgra8Unorm
        | wgt::TextureFormat::Bgra8UnormSrgb => Ok(4),
        wgt::TextureFormat::Rg16Float
        | wgt::TextureFormat::Rg16Unorm
        | wgt::TextureFormat::Rg16Snorm
        | wgt::TextureFormat::Rg16Uint
        | wgt::TextureFormat::Rg16Sint => Ok(4),
        wgt::TextureFormat::Rgba16Float
        | wgt::TextureFormat::Rgba16Unorm
        | wgt::TextureFormat::Rgba16Snorm
        | wgt::TextureFormat::Rgba16Uint
        | wgt::TextureFormat::Rgba16Sint => Ok(8),
        wgt::TextureFormat::Rg32Float
        | wgt::TextureFormat::Rg32Uint
        | wgt::TextureFormat::Rg32Sint => Ok(8),
        wgt::TextureFormat::Rgba32Float
        | wgt::TextureFormat::Rgba32Uint
        | wgt::TextureFormat::Rgba32Sint => Ok(16),
        wgt::TextureFormat::Rg8Unorm
        | wgt::TextureFormat::Rg8Uint
        | wgt::TextureFormat::Rg8Sint
        | wgt::TextureFormat::Rg8Snorm
        | wgt::TextureFormat::R16Float
        | wgt::TextureFormat::R16Unorm
        | wgt::TextureFormat::R16Snorm
        | wgt::TextureFormat::R16Uint
        | wgt::TextureFormat::R16Sint => Ok(2),
        wgt::TextureFormat::R32Float
        | wgt::TextureFormat::R32Uint
        | wgt::TextureFormat::R32Sint => Ok(4),
        wgt::TextureFormat::R8Unorm
        | wgt::TextureFormat::R8Uint
        | wgt::TextureFormat::R8Sint
        | wgt::TextureFormat::R8Snorm => Ok(1),
        wgt::TextureFormat::Depth16Unorm => Ok(2),
        wgt::TextureFormat::Depth32Float => Ok(4),
        wgt::TextureFormat::Stencil8 => Ok(1),
        _ => Err(crate::DeviceError::Lost),
    }
}

#[cfg(any(target_os = "horizon", test))]
fn copy_formats_are_compatible(
    src_format: wgt::TextureFormat,
    dst_format: wgt::TextureFormat,
) -> bool {
    src_format.remove_srgb_suffix() == dst_format.remove_srgb_suffix()
}

#[cfg(target_os = "horizon")]
fn buffer_copy_region(
    buffer: &Buffer,
    region: &crate::BufferTextureCopy,
    texel_size: u32,
) -> DeviceResult<dk::DkCopyBuf> {
    let row_bytes = region
        .size
        .width
        .checked_mul(texel_size)
        .ok_or(crate::DeviceError::Lost)?;
    let bytes_per_row = region.buffer_layout.bytes_per_row.unwrap_or(row_bytes);
    if bytes_per_row < row_bytes || bytes_per_row % texel_size != 0 {
        return Err(crate::DeviceError::Lost);
    }
    let rows_per_image = region
        .buffer_layout
        .rows_per_image
        .unwrap_or(region.size.height);
    if rows_per_image < region.size.height {
        return Err(crate::DeviceError::Lost);
    }
    let depth = region.size.depth.max(1);
    let required_bytes = if region.size.height == 0 || region.size.width == 0 {
        0
    } else {
        let image_stride = u64::from(bytes_per_row)
            .checked_mul(u64::from(rows_per_image))
            .ok_or(crate::DeviceError::Lost)?;
        image_stride
            .checked_mul(u64::from(depth - 1))
            .and_then(|bytes| {
                bytes.checked_add(
                    u64::from(bytes_per_row).checked_mul(u64::from(region.size.height - 1))?,
                )
            })
            .and_then(|bytes| bytes.checked_add(u64::from(row_bytes)))
            .ok_or(crate::DeviceError::Lost)?
    };
    let required_bytes = wgt::BufferSize::new(required_bytes).ok_or(crate::DeviceError::Lost)?;
    let (addr, _) = buffer.gpu_binding(region.buffer_layout.offset, Some(required_bytes))?;

    Ok(dk::DkCopyBuf {
        addr,
        rowLength: if bytes_per_row == row_bytes {
            0
        } else {
            bytes_per_row / texel_size
        },
        imageHeight: if rows_per_image == region.size.height {
            0
        } else {
            rows_per_image
        },
    })
}

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_buffer_to_texture(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &Buffer,
    dst: &TextureInner,
    regions: &[crate::BufferTextureCopy],
) -> DeviceResult<()> {
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for region in regions {
                let copy_rect = texture_copy_rect(dst, &region.texture_base, region.size)?;
                let dst_view = texture_copy_view(dst, &region.texture_base, region.size.depth)?;
                let texel_size = copy_texel_size(dst.format())?;
                let copy_src = buffer_copy_region(src, region, texel_size)?;
                dk::dkCmdBufCopyBufferToImage(cmdbuf, &copy_src, &dst_view, &copy_rect, 0);
            }
            Ok(())
        })
    }
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
unsafe fn submit_copy_texture_to_texture(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &TextureInner,
    dst: &TextureInner,
    regions: &[crate::TextureCopy],
) -> DeviceResult<()> {
    if !copy_formats_are_compatible(src.format(), dst.format())
        || src.sample_count() != 1
        || dst.sample_count() != 1
    {
        return Err(crate::DeviceError::Lost);
    }
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for region in regions {
                let src_rect = texture_copy_rect(src, &region.src_base, region.size)?;
                let dst_rect = texture_copy_rect(dst, &region.dst_base, region.size)?;
                let src_view = texture_copy_view(src, &region.src_base, region.size.depth)?;
                let dst_view = texture_copy_view(dst, &region.dst_base, region.size.depth)?;
                dk::dkCmdBufCopyImage(cmdbuf, &src_view, &src_rect, &dst_view, &dst_rect, 0);
            }
            Ok(())
        })
    }
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

#[cfg(target_os = "horizon")]
unsafe fn submit_copy_texture_to_buffer(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    src: &TextureInner,
    dst: &Buffer,
    regions: &[crate::BufferTextureCopy],
) -> DeviceResult<()> {
    if src.sample_count() != 1 {
        return Err(crate::DeviceError::Lost);
    }
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for region in regions {
                let src_rect = texture_copy_rect(src, &region.texture_base, region.size)?;
                let src_view = texture_copy_view(src, &region.texture_base, region.size.depth)?;
                let texel_size = copy_texel_size(src.format())?;
                let copy_dst = buffer_copy_region(dst, region, texel_size)?;
                dk::dkCmdBufCopyImageToBuffer(cmdbuf, &src_view, &src_rect, &copy_dst, 0);
            }
            Ok(())
        })
    }
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

#[cfg(target_os = "horizon")]
unsafe fn submit_begin_render_pass(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    extent: wgt::Extent3d,
    colors: &[Option<ColorAttachmentState>],
    depth_stencil: DepthStencilAttachmentState,
) -> DeviceResult<()> {
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            let image_views = colors
                .iter()
                .map(|color| color.as_ref().map(|color| color.view.deko_view()))
                .collect::<Vec<_>>();
            let image_view_ptrs = image_views
                .iter()
                .map(|view| view.as_ref().map_or(ptr::null(), ptr::from_ref))
                .collect::<Vec<_>>();
            let image_view_ptr = if image_view_ptrs.is_empty() {
                ptr::null()
            } else {
                image_view_ptrs.as_ptr()
            };
            let depth_stencil_image_view = depth_stencil.view.map(AttachmentViewState::deko_view);
            let depth_stencil_image_view_ptr = depth_stencil_image_view
                .as_ref()
                .map_or(ptr::null(), |view| ptr::from_ref(view));
            dk::dkCmdBufBindRenderTargets(
                cmdbuf,
                image_view_ptr,
                image_view_ptrs.len() as u32,
                depth_stencil_image_view_ptr,
            );
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
            for (index, color) in colors.iter().enumerate() {
                if let Some(clear_value) = color.as_ref().and_then(|color| color.clear_value) {
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
            if depth_stencil.has_clear() {
                dk::dkCmdBufClearDepthStencil(
                    cmdbuf,
                    depth_stencil.depth_clear_value.is_some(),
                    depth_stencil.depth_clear_value.unwrap_or(1.0),
                    if depth_stencil.stencil_clear_value.is_some() {
                        0xFF
                    } else {
                        0
                    },
                    depth_stencil.stencil_clear_value.unwrap_or(0),
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
    _extent: wgt::Extent3d,
    _colors: &[Option<ColorAttachmentState>],
    _depth_stencil: DepthStencilAttachmentState,
) -> DeviceResult<()> {
    Err(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn submit_end_render_pass(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    state: &mut ExecutionState,
) -> DeviceResult<()> {
    let Some(target) = state.target.take() else {
        return Err(crate::DeviceError::Lost);
    };
    if !target
        .colors
        .iter()
        .any(|color| color.resolve_view.is_some() || color.discard)
        && !target.depth_stencil_discard
    {
        return Ok(());
    }
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for color in target.colors {
                if let Some(resolve_view) = color.resolve_view {
                    let src_view = color.view.deko_view();
                    let dst_view = resolve_view.deko_view();
                    dk::dkCmdBufResolveImage(cmdbuf, &src_view, &dst_view);
                }
                if color.discard {
                    dk::dkCmdBufDiscardColor(cmdbuf, color.target_id);
                }
            }
            if target.depth_stencil_discard {
                dk::dkCmdBufDiscardDepthStencil(cmdbuf);
            }
            Ok(())
        })
    }
}

#[cfg(not(target_os = "horizon"))]
unsafe fn submit_end_render_pass(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _state: &mut ExecutionState,
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
#[allow(clippy::too_many_arguments)]
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
#[allow(clippy::too_many_arguments)]
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
#[allow(clippy::too_many_arguments)]
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
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            let shader = [pipeline.compute_shader.raw_shader()];
            dk::dkCmdBufBindShaders(
                cmdbuf,
                dk::DkStageFlag_Compute,
                shader.as_ptr(),
                shader.len() as u32,
            );
            for (group_index, group) in state.bind_groups.iter().enumerate() {
                let Some(group) = group else { continue };
                group.group.bind_descriptor_sets(
                    cmdbuf,
                    group_index as u32,
                    &group.dynamic_offsets,
                    &pipeline.binding_map,
                )?;
            }
            pipeline
                .layout
                .bind_immediates(cmdbuf, wgt::ShaderStages::COMPUTE)?;
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
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            let shader = [pipeline.compute_shader.raw_shader()];
            dk::dkCmdBufBindShaders(
                cmdbuf,
                dk::DkStageFlag_Compute,
                shader.as_ptr(),
                shader.len() as u32,
            );
            for (group_index, group) in state.bind_groups.iter().enumerate() {
                let Some(group) = group else { continue };
                group.group.bind_descriptor_sets(
                    cmdbuf,
                    group_index as u32,
                    &group.dynamic_offsets,
                    &pipeline.binding_map,
                )?;
            }
            pipeline
                .layout
                .bind_immediates(cmdbuf, wgt::ShaderStages::COMPUTE)?;
            dk::dkCmdBufDispatchComputeIndirect(cmdbuf, dispatch_addr);
            Ok(())
        })
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
    draw: impl FnOnce(dk::DkCmdBuf, &super::RenderPipelineInnerRaw) -> DeviceResult<()>,
) -> DeviceResult<()> {
    let target = state.target.as_ref().ok_or(crate::DeviceError::Lost)?;
    let pipeline = state.pipeline.as_ref().ok_or(crate::DeviceError::Lost)?;
    let pipeline = pipeline.raw();
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            let viewport = state.viewport.as_ref().map_or_else(
                || dk::DkViewport {
                    x: 0.0,
                    y: 0.0,
                    width: target.extent.width as f32,
                    height: target.extent.height as f32,
                    near: 0.0,
                    far: 1.0,
                },
                |viewport| dk::DkViewport {
                    x: viewport.rect.x,
                    y: viewport.rect.y,
                    width: viewport.rect.w,
                    height: viewport.rect.h,
                    near: viewport.depth_range.start,
                    far: viewport.depth_range.end,
                },
            );
            let scissor = state.scissor.as_ref().map_or_else(
                || dk::DkScissor {
                    x: 0,
                    y: 0,
                    width: target.extent.width,
                    height: target.extent.height,
                },
                |scissor| dk::DkScissor {
                    x: scissor.rect.x,
                    y: scissor.rect.y,
                    width: scissor.rect.w,
                    height: scissor.rect.h,
                },
            );
            let shaders = [
                pipeline.vertex_shader.raw_shader(),
                pipeline
                    .fragment_shader
                    .as_ref()
                    .map_or(ptr::null(), |shader| shader.raw_shader()),
            ];
            let shader_count = if pipeline.fragment_shader.is_some() {
                2
            } else {
                1
            };
            dk::dkCmdBufSetViewports(cmdbuf, 0, &viewport, 1);
            dk::dkCmdBufSetScissors(cmdbuf, 0, &scissor, 1);
            dk::dkCmdBufBindShaders(
                cmdbuf,
                dk::DkStageFlag_GraphicsMask,
                shaders.as_ptr(),
                shader_count,
            );
            for (group_index, group) in state.bind_groups.iter().enumerate() {
                let Some(group) = group else { continue };
                group.group.bind_descriptor_sets(
                    cmdbuf,
                    group_index as u32,
                    &group.dynamic_offsets,
                    &pipeline.binding_map,
                )?;
            }
            pipeline
                .layout
                .bind_immediates(cmdbuf, pipeline.active_stages)?;
            dk::dkCmdBufBindRasterizerState(cmdbuf, &pipeline.rasterizer_state);
            dk::dkCmdBufSetDepthBias(
                cmdbuf,
                pipeline.depth_bias.constant as f32,
                pipeline.depth_bias.clamp,
                pipeline.depth_bias.slope_scale,
            );
            let primitive_restart_index = match pipeline.strip_index_format {
                Some(wgt::IndexFormat::Uint16) => Some(u16::MAX as u32),
                Some(wgt::IndexFormat::Uint32) => Some(u32::MAX),
                None => None,
            };
            dk::dkCmdBufSetPrimitiveRestart(
                cmdbuf,
                primitive_restart_index.is_some(),
                primitive_restart_index.unwrap_or(0),
            );
            dk::dkCmdBufBindColorState(cmdbuf, &pipeline.color_state);
            dk::dkCmdBufBindColorWriteState(cmdbuf, &pipeline.color_write_state);
            dk::dkCmdBufBindBlendStates(
                cmdbuf,
                0,
                pipeline.blend_states.as_ptr(),
                pipeline.blend_states.len() as u32,
            );
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
            draw(cmdbuf, pipeline)
        })
    }
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
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            layout.push_immediates(cmdbuf, offset_bytes, data)
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
unsafe fn submit_set_immediates(
    _queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    _layout: &super::PipelineLayoutInner,
    _offset_bytes: u32,
    _data: &[u32],
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
#[allow(clippy::too_many_arguments)]
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
#[allow(clippy::too_many_arguments)]
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
unsafe fn submit_deko_commands(
    queue: &Queue,
    _surface_queue: Option<RawQueueHandle>,
    record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
) -> DeviceResult<()> {
    let cmdbuf = unsafe { queue.active_cmdbuf() }?;
    record(cmdbuf)
}

#[cfg(all(test, not(target_os = "horizon")))]
mod tests {
    use super::*;
    use crate::CommandEncoder as _;
    use alloc::sync::Arc;

    fn test_buffer() -> Buffer {
        Buffer::new(&crate::BufferDescriptor {
            label: None,
            size: 4,
            usage: wgt::BufferUses::COPY_SRC | wgt::BufferUses::COPY_DST,
            memory_flags: crate::MemoryFlags::empty(),
        })
        .expect("host Deko3D test buffer")
    }

    fn test_texture() -> Resource {
        Resource::Texture(Arc::new(TextureInner {
            inner: super::super::TextureInnerRaw,
        }))
    }

    fn test_texture_range() -> wgt::ImageSubresourceRange {
        wgt::ImageSubresourceRange {
            aspect: wgt::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
        }
    }

    fn assert_single_resource_barrier(command_buffer: &CommandBuffer) {
        assert_eq!(command_buffer.commands.len(), 1);
        match command_buffer.commands[0] {
            Command::ResourceBarrier { invalidate_flags } => {
                assert_eq!(invalidate_flags, DEKO_RESOURCE_TRANSITION_INVALIDATE_FLAGS);
            }
            ref command => panic!("expected resource barrier, got {command:?}"),
        }
    }

    fn assert_unsupported_commands(command_buffer: &CommandBuffer, count: usize) {
        assert_eq!(command_buffer.commands.len(), count);
        assert!(command_buffer
            .commands
            .iter()
            .all(|command| matches!(command, Command::Error)));
    }

    #[test]
    fn accepts_srgb_copy_compatible_formats() {
        for (src, dst) in [
            (
                wgt::TextureFormat::Rgba8Unorm,
                wgt::TextureFormat::Rgba8UnormSrgb,
            ),
            (
                wgt::TextureFormat::Rgba8UnormSrgb,
                wgt::TextureFormat::Rgba8Unorm,
            ),
            (
                wgt::TextureFormat::Bgra8Unorm,
                wgt::TextureFormat::Bgra8UnormSrgb,
            ),
            (
                wgt::TextureFormat::Bgra8UnormSrgb,
                wgt::TextureFormat::Bgra8Unorm,
            ),
        ] {
            assert!(copy_formats_are_compatible(src, dst));
        }
        assert!(!copy_formats_are_compatible(
            wgt::TextureFormat::Rgba8Unorm,
            wgt::TextureFormat::Bgra8Unorm,
        ));
    }

    #[test]
    fn color_copy_aspects_include_r_and_rg_unorm_and_normalized_16bit_formats() {
        for format in [
            wgt::TextureFormat::R8Unorm,
            wgt::TextureFormat::Rg8Unorm,
            wgt::TextureFormat::R16Unorm,
            wgt::TextureFormat::R16Snorm,
            wgt::TextureFormat::Rg16Unorm,
            wgt::TextureFormat::Rg16Snorm,
            wgt::TextureFormat::Rgba16Unorm,
            wgt::TextureFormat::Rgba16Snorm,
            wgt::TextureFormat::R16Uint,
            wgt::TextureFormat::R16Sint,
            wgt::TextureFormat::Rg16Uint,
            wgt::TextureFormat::Rg16Sint,
            wgt::TextureFormat::Rgba16Uint,
            wgt::TextureFormat::Rgba16Sint,
            wgt::TextureFormat::R32Float,
            wgt::TextureFormat::R32Uint,
            wgt::TextureFormat::R32Sint,
            wgt::TextureFormat::Rg32Float,
            wgt::TextureFormat::Rg32Uint,
            wgt::TextureFormat::Rg32Sint,
            wgt::TextureFormat::Rgba32Float,
            wgt::TextureFormat::Rgba32Uint,
            wgt::TextureFormat::Rgba32Sint,
        ] {
            assert_eq!(copy_format_aspect(format), Ok(crate::FormatAspects::COLOR));
        }
    }

    #[test]
    fn query_commands_record_deferred_errors() {
        let resource = Resource::Placeholder;
        let buffer = test_buffer();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.begin_query(&resource, 0);
            encoder.end_query(&resource, 0);
            encoder.write_timestamp(&resource, 0);
            encoder.reset_queries(&resource, 0..1);
            encoder.copy_query_results(
                &resource,
                0..1,
                &buffer,
                0,
                wgt::BufferSize::new(8).unwrap(),
            );
        }

        assert_unsupported_commands(&encoder, 5);
    }

    #[test]
    fn occlusion_query_commands_are_recorded() {
        let resource = Resource::QuerySet(Arc::new(QuerySetInner {
            query_type: wgt::QueryType::Occlusion,
            count: 2,
        }));
        let buffer = test_buffer();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.reset_queries(&resource, 0..2);
            encoder.begin_query(&resource, 1);
            encoder.end_query(&resource, 1);
            encoder.copy_query_results(
                &resource,
                0..2,
                &buffer,
                0,
                wgt::BufferSize::new(8).unwrap(),
            );
        }

        assert_eq!(encoder.commands.len(), 4);
        assert!(matches!(encoder.commands[0], Command::ResetQueries { .. }));
        assert!(matches!(
            encoder.commands[1],
            Command::BeginOcclusionQuery { index: 1, .. }
        ));
        assert!(matches!(
            encoder.commands[2],
            Command::EndOcclusionQuery { index: 1, .. }
        ));
        assert!(matches!(
            encoder.commands[3],
            Command::CopyQueryResults { .. }
        ));
        assert!(supports_render_pass_occlusion_query_set(Some(&resource)));

        let texture = test_texture();
        assert!(!supports_render_pass_occlusion_query_set(Some(&texture)));
    }

    #[test]
    fn depth_only_render_passes_use_the_depth_extent() {
        let extent = wgt::Extent3d {
            width: 4,
            height: 2,
            depth_or_array_layers: 1,
        };
        assert!(color_attachments(&[], 1).unwrap().is_empty());
        let sparse_colors = color_attachments(&[None], 1).unwrap();
        assert_eq!(sparse_colors.len(), 1);
        assert!(sparse_colors[0].is_none());
        assert_eq!(
            render_pass_extent(
                &sparse_colors,
                DepthStencilAttachmentState {
                    extent: Some(extent),
                    ..Default::default()
                },
            )
            .unwrap(),
            extent
        );
        assert!(render_pass_extent(&[], DepthStencilAttachmentState::default()).is_err());
    }

    #[test]
    fn pipeline_statistics_query_commands_are_recorded() {
        let statistics = wgt::PipelineStatisticsTypes::VERTEX_SHADER_INVOCATIONS
            | wgt::PipelineStatisticsTypes::CLIPPER_INVOCATIONS
            | wgt::PipelineStatisticsTypes::CLIPPER_PRIMITIVES_OUT
            | wgt::PipelineStatisticsTypes::FRAGMENT_SHADER_INVOCATIONS;
        let resource = Resource::QuerySet(Arc::new(QuerySetInner {
            query_type: wgt::QueryType::PipelineStatistics(statistics),
            count: 2,
        }));
        let buffer = test_buffer();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.reset_queries(&resource, 0..2);
            encoder.begin_query(&resource, 1);
            encoder.end_query(&resource, 1);
            encoder.copy_query_results(
                &resource,
                0..2,
                &buffer,
                0,
                wgt::BufferSize::new(32).unwrap(),
            );
        }

        assert_eq!(encoder.commands.len(), 4);
        assert!(matches!(encoder.commands[0], Command::ResetQueries { .. }));
        assert!(matches!(
            encoder.commands[1],
            Command::BeginPipelineStatisticsQuery { index: 1, .. }
        ));
        assert!(matches!(
            encoder.commands[2],
            Command::EndPipelineStatisticsQuery { index: 1, .. }
        ));
        assert!(matches!(
            encoder.commands[3],
            Command::CopyQueryResults { .. }
        ));
    }

    #[test]
    fn timestamp_write_commands_are_recorded() {
        let resource = Resource::QuerySet(Arc::new(QuerySetInner {
            query_type: wgt::QueryType::Timestamp,
            count: 2,
        }));
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.write_timestamp(&resource, 1);
        }

        assert_eq!(encoder.commands.len(), 1);
        assert!(matches!(
            encoder.commands[0],
            Command::WriteTimestamp(TimestampWrite { index: 1, .. })
        ));
    }

    #[test]
    fn ray_tracing_commands_record_deferred_errors() {
        let resource = Resource::Placeholder;
        let buffer = test_buffer();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.begin_ray_tracing_pass(&crate::RayTracingPassDescriptor { label: None });
            encoder.end_ray_tracing_pass();
            encoder.set_ray_tracing_pipeline(&resource);
            encoder.trace_rays(
                [1, 1, 1],
                crate::PipelineGroupData {
                    buffer: &buffer,
                    offset: 0,
                    stride: 0,
                    count: 0,
                },
                crate::PipelineGroupData {
                    buffer: &buffer,
                    offset: 0,
                    stride: 0,
                    count: 0,
                },
                crate::PipelineGroupData {
                    buffer: &buffer,
                    offset: 0,
                    stride: 0,
                    count: 0,
                },
            );
        }

        assert_unsupported_commands(&encoder, 4);
    }

    #[test]
    fn acceleration_structure_commands_record_deferred_errors() {
        let resource = Resource::Placeholder;
        let buffer = test_buffer();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.read_acceleration_structure_compact_size(&resource, &buffer);
            encoder.build_acceleration_structures(
                0,
                core::iter::empty::<
                    crate::BuildAccelerationStructureDescriptor<'_, Buffer, Resource>,
                >(),
            );
            encoder.place_acceleration_structure_barrier(crate::AccelerationStructureBarrier {
                usage: crate::StateTransition {
                    from: crate::AccelerationStructureUses::BUILD_OUTPUT,
                    to: crate::AccelerationStructureUses::SHADER_INPUT,
                },
            });
            encoder.copy_acceleration_structure_to_acceleration_structure(
                &resource,
                &resource,
                wgt::AccelerationStructureCopy::Clone,
            );
        }

        assert_unsupported_commands(&encoder, 4);
    }

    #[test]
    fn transition_buffers_records_full_resource_barrier_for_usage_change() {
        let buffer = test_buffer();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.transition_buffers(
                [crate::BufferBarrier {
                    buffer: &buffer,
                    usage: crate::StateTransition {
                        from: wgt::BufferUses::COPY_DST,
                        to: wgt::BufferUses::VERTEX,
                    },
                }]
                .into_iter(),
            );
        }

        assert_single_resource_barrier(&encoder);
    }

    #[test]
    fn transition_textures_records_full_resource_barrier_for_usage_change() {
        let texture = test_texture();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.transition_textures(
                [crate::TextureBarrier {
                    texture: &texture,
                    range: test_texture_range(),
                    usage: crate::StateTransition {
                        from: wgt::TextureUses::COPY_DST,
                        to: wgt::TextureUses::RESOURCE,
                    },
                }]
                .into_iter(),
            );
        }

        assert_single_resource_barrier(&encoder);
    }

    #[test]
    fn unchanged_resource_usages_do_not_record_barriers() {
        let buffer = test_buffer();
        let texture = test_texture();
        let mut encoder = CommandBuffer::new();

        unsafe {
            encoder.transition_buffers(
                [crate::BufferBarrier {
                    buffer: &buffer,
                    usage: crate::StateTransition {
                        from: wgt::BufferUses::COPY_DST,
                        to: wgt::BufferUses::COPY_DST,
                    },
                }]
                .into_iter(),
            );
            encoder.transition_textures(
                [crate::TextureBarrier {
                    texture: &texture,
                    range: test_texture_range(),
                    usage: crate::StateTransition {
                        from: wgt::TextureUses::COPY_DST,
                        to: wgt::TextureUses::COPY_DST,
                    },
                }]
                .into_iter(),
            );
        }

        assert!(encoder.commands.is_empty());
    }

    #[test]
    fn supports_all_native_multisample_counts() {
        for sample_count in [1, 2, 4, 8] {
            assert!(supports_sample_count(sample_count));
        }
        for sample_count in [0, 3, 16] {
            assert!(!supports_sample_count(sample_count));
        }
    }

    #[test]
    fn read_only_depth_stencil_attachment_requires_read_usage() {
        assert_eq!(
            depth_stencil_attachment_usage(
                crate::AttachmentOps::empty(),
                crate::AttachmentOps::empty(),
            ),
            wgt::TextureUses::DEPTH_STENCIL_READ
        );
        assert_eq!(
            depth_stencil_attachment_usage(
                crate::AttachmentOps::LOAD | crate::AttachmentOps::STORE,
                crate::AttachmentOps::empty(),
            ),
            wgt::TextureUses::DEPTH_STENCIL_WRITE
        );
    }

    #[test]
    fn discard_store_operations_are_admitted() {
        let discard_ops = crate::AttachmentOps::LOAD | crate::AttachmentOps::STORE_DISCARD;
        assert!(validate_attachment_store(discard_ops).is_ok());
        assert!(attachment_should_discard(discard_ops));
        assert!(!attachment_should_discard(
            crate::AttachmentOps::LOAD | crate::AttachmentOps::STORE
        ));
    }
}
