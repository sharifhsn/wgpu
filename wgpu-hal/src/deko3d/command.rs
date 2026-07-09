use alloc::vec::Vec;
use core::mem;
use core::ops::Range;

#[cfg(target_os = "horizon")]
use core::mem::size_of;

#[cfg(target_os = "horizon")]
use core::ptr;

#[cfg(target_os = "horizon")]
use deko3d_sys as dk;

use super::{
    Api, BindGroupInner, Buffer, ComputePipelineInner, DeviceResult, Queue, RawImage,
    RawQueueHandle, RenderPipelineInner, Resource, TextureInner,
};

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
    BeginRenderPass {
        colors: Vec<ColorAttachmentState>,
        depth_stencil: DepthStencilAttachmentState,
    },
    EndRenderPass,
    SetRenderPipeline {
        pipeline: alloc::sync::Arc<RenderPipelineInner>,
    },
    BeginComputePass,
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

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct ColorAttachmentState {
    image: RawImage,
    extent: wgt::Extent3d,
    sample_count: u32,
    clear_value: Option<wgt::Color>,
    resolve_image: Option<RawImage>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
struct DepthStencilAttachmentState {
    image: Option<RawImage>,
    depth_clear_value: Option<f32>,
    stencil_clear_value: Option<u8>,
}

impl DepthStencilAttachmentState {
    #[allow(dead_code)]
    fn has_clear(self) -> bool {
        self.depth_clear_value.is_some() || self.stencil_clear_value.is_some()
    }
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
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct RenderTargetColor {
    image: RawImage,
    resolve_image: Option<RawImage>,
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
    }

    unsafe fn transition_textures<'a, T>(&mut self, barriers: T)
    where
        T: Iterator<Item = crate::TextureBarrier<'a, Resource>>,
    {
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

    unsafe fn begin_query(&mut self, set: &Resource, index: u32) {}
    unsafe fn end_query(&mut self, set: &Resource, index: u32) {}
    unsafe fn write_timestamp(&mut self, set: &Resource, index: u32) {}
    unsafe fn read_acceleration_structure_compact_size(
        &mut self,
        acceleration_structure: &Resource,
        buf: &Buffer,
    ) {
    }
    unsafe fn reset_queries(&mut self, set: &Resource, range: Range<u32>) {}
    unsafe fn copy_query_results(
        &mut self,
        set: &Resource,
        range: Range<u32>,
        buffer: &Buffer,
        offset: wgt::BufferAddress,
        stride: wgt::BufferSize,
    ) {
    }

    // render

    unsafe fn begin_render_pass(
        &mut self,
        desc: &crate::RenderPassDescriptor<Resource, Resource>,
    ) -> DeviceResult<()> {
        if desc.multiview_mask.is_some()
            || desc.timestamp_writes.is_some()
            || desc.occlusion_query_set.is_some()
            || !supports_sample_count(desc.sample_count)
        {
            return Err(crate::DeviceError::Lost);
        }

        let colors = color_attachments(desc.color_attachments, desc.sample_count)?;
        let extent = colors[0].extent;
        let depth_stencil =
            depth_stencil_attachment(desc.depth_stencil_attachment.as_ref(), extent)?;
        self.commands.push(Command::BeginRenderPass {
            colors,
            depth_stencil,
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
    unsafe fn set_immediates(&mut self, layout: &Resource, offset_bytes: u32, data: &[u32]) {}

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
        if desc.timestamp_writes.is_some() {
            self.commands.push(Command::Error);
            return;
        }
        self.commands.push(Command::BeginComputePass);
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

    unsafe fn begin_ray_tracing_pass(&mut self, desc: &crate::RayTracingPassDescriptor) {
        unimplemented!()
    }
    unsafe fn end_ray_tracing_pass(&mut self) {
        unimplemented!()
    }
    unsafe fn set_ray_tracing_pipeline(&mut self, pipeline: &Resource) {
        unimplemented!()
    }
    unsafe fn trace_rays(
        &mut self,
        count: [u32; 3],
        ray_generation_group_data: crate::PipelineGroupData<Buffer>,
        miss_group_data: crate::PipelineGroupData<Buffer>,
        intersection_group_data: crate::PipelineGroupData<Buffer>,
    ) {
        unimplemented!()
    }

    unsafe fn build_acceleration_structures<'a, T>(
        &mut self,
        _descriptor_count: u32,
        descriptors: T,
    ) where
        Api: 'a,
        T: IntoIterator<Item = crate::BuildAccelerationStructureDescriptor<'a, Buffer, Resource>>,
    {
    }

    unsafe fn place_acceleration_structure_barrier(
        &mut self,
        _barriers: crate::AccelerationStructureBarrier,
    ) {
    }

    unsafe fn copy_acceleration_structure_to_acceleration_structure(
        &mut self,
        src: &Resource,
        dst: &Resource,
        copy: wgt::AccelerationStructureCopy,
    ) {
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
            Command::BeginRenderPass {
                colors,
                depth_stencil,
            } => {
                let extent = colors.first().ok_or(crate::DeviceError::Lost)?.extent;
                state.target = Some(RenderTarget {
                    extent,
                    colors: colors
                        .iter()
                        .map(|color| RenderTargetColor {
                            image: color.image,
                            resolve_image: color.resolve_image,
                        })
                        .collect(),
                });
                state.viewport = None;
                state.scissor = None;
                state.stencil_reference = 0;
                state.blend_constants = [0.0; 4];
                unsafe { submit_begin_render_pass(queue, surface_queue, colors, *depth_stencil) }
            }
            Command::EndRenderPass => unsafe {
                submit_end_render_pass(queue, surface_queue, state)
            },
            Command::SetRenderPipeline { pipeline } => {
                state.pipeline = Some(pipeline.clone());
                Ok(())
            }
            Command::BeginComputePass => {
                state.in_compute_pass = true;
                state.compute_pipeline = None;
                Ok(())
            }
            Command::EndComputePass => {
                if !state.in_compute_pass {
                    return Err(crate::DeviceError::Lost);
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

fn color_attachments(
    color_attachments: &[Option<crate::ColorAttachment<'_, Resource>>],
    sample_count: u32,
) -> DeviceResult<Vec<ColorAttachmentState>> {
    if color_attachments.is_empty() || color_attachments.len() > 2 {
        return Err(crate::DeviceError::Lost);
    }
    let mut colors: Vec<ColorAttachmentState> = Vec::with_capacity(color_attachments.len());
    for attachment in color_attachments {
        let Some(attachment) = attachment else {
            return Err(crate::DeviceError::Lost);
        };
        let color = color_attachment(attachment, sample_count)?;
        if let Some(first) = colors.first() {
            if color.extent != first.extent {
                return Err(crate::DeviceError::Lost);
            }
        }
        colors.push(color);
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
    let Resource::TextureView {
        image,
        extent,
        sample_count: target_sample_count,
        ..
    } = attachment.target.view
    else {
        return Err(crate::DeviceError::Lost);
    };
    if *target_sample_count != sample_count {
        return Err(crate::DeviceError::Lost);
    }
    let resolve_image = resolve_attachment(attachment.resolve_target.as_ref(), *extent)?;
    if sample_count == 1 && resolve_image.is_some() {
        return Err(crate::DeviceError::Lost);
    }
    Ok(ColorAttachmentState {
        image: *image,
        extent: *extent,
        sample_count,
        clear_value: attachment_clear_value(attachment.ops, attachment.clear_value)?,
        resolve_image,
    })
}

fn resolve_attachment(
    attachment: Option<&crate::Attachment<'_, Resource>>,
    expected_extent: wgt::Extent3d,
) -> DeviceResult<Option<RawImage>> {
    let Some(attachment) = attachment else {
        return Ok(None);
    };
    if !attachment.usage.contains(wgt::TextureUses::COLOR_TARGET) {
        return Err(crate::DeviceError::Lost);
    }
    let Resource::TextureView {
        image,
        extent,
        sample_count,
        ..
    } = attachment.view
    else {
        return Err(crate::DeviceError::Lost);
    };
    if *extent != expected_extent || *sample_count != 1 {
        return Err(crate::DeviceError::Lost);
    }
    Ok(Some(*image))
}

fn depth_stencil_attachment(
    attachment: Option<&crate::DepthStencilAttachment<'_, Resource>>,
    expected_extent: wgt::Extent3d,
) -> DeviceResult<DepthStencilAttachmentState> {
    let Some(attachment) = attachment else {
        return Ok(DepthStencilAttachmentState::default());
    };
    if attachment.depth_ops.is_empty() && attachment.stencil_ops.is_empty() {
        return Err(crate::DeviceError::Lost);
    }
    if !attachment
        .target
        .usage
        .contains(wgt::TextureUses::DEPTH_STENCIL_WRITE)
    {
        return Err(crate::DeviceError::Lost);
    }
    let Resource::TextureView { image, extent, .. } = attachment.target.view else {
        return Err(crate::DeviceError::Lost);
    };
    if *extent != expected_extent {
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
        image: Some(*image),
        depth_clear_value,
        stencil_clear_value,
    })
}

fn attachment_clear_value<T: Copy>(ops: crate::AttachmentOps, value: T) -> DeviceResult<Option<T>> {
    if ops.contains(crate::AttachmentOps::LOAD_CLEAR) {
        return Ok(Some(value));
    }
    validate_attachment_load(ops)?;
    Ok(None)
}

fn validate_attachment_store(ops: crate::AttachmentOps) -> DeviceResult<()> {
    if ops.contains(crate::AttachmentOps::STORE_DISCARD)
        || !ops.contains(crate::AttachmentOps::STORE)
    {
        return Err(crate::DeviceError::Lost);
    }
    Ok(())
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
    matches!(sample_count, 1 | 4)
}

#[cfg(target_os = "horizon")]
fn texture_copy_rect(
    texture: &TextureInner,
    base: &crate::TextureCopyBase,
    size: crate::CopyExtent,
) -> DeviceResult<dk::DkImageRect> {
    if base.array_layer != 0
        || base.origin.z != 0
        || base.aspect != crate::FormatAspects::COLOR
        || size.depth != 1
    {
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

    Ok(dk::DkImageRect {
        x: base.origin.x,
        y: base.origin.y,
        z: base.origin.z,
        width: size.width,
        height: size.height,
        depth: size.depth,
    })
}

#[cfg(target_os = "horizon")]
fn texture_copy_view(texture: &TextureInner, mip_level: u32) -> DeviceResult<dk::DkImageView> {
    if mip_level >= texture.mip_level_count() {
        return Err(crate::DeviceError::Lost);
    }
    let mut view = dk::DkImageView::defaults(texture.raw_image().0);
    view.mipLevelOffset = u8::try_from(mip_level).map_err(|_| crate::DeviceError::Lost)?;
    view.mipLevelCount = 1;
    view.layerOffset = 0;
    view.layerCount = 1;
    Ok(view)
}

#[cfg(target_os = "horizon")]
fn copy_texel_size(format: wgt::TextureFormat) -> DeviceResult<u32> {
    match format {
        wgt::TextureFormat::Rgba8Unorm | wgt::TextureFormat::Rgba8UnormSrgb => Ok(4),
        _ => Err(crate::DeviceError::Lost),
    }
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
    let required_bytes = if region.size.height == 0 {
        0
    } else {
        u64::from(bytes_per_row)
            .checked_mul(u64::from(region.size.height - 1))
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
                let dst_view = texture_copy_view(dst, region.texture_base.mip_level)?;
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
    if src.format() != dst.format() || src.sample_count() != 1 || dst.sample_count() != 1 {
        return Err(crate::DeviceError::Lost);
    }
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for region in regions {
                let src_rect = texture_copy_rect(src, &region.src_base, region.size)?;
                let dst_rect = texture_copy_rect(dst, &region.dst_base, region.size)?;
                let src_view = texture_copy_view(src, region.src_base.mip_level)?;
                let dst_view = texture_copy_view(dst, region.dst_base.mip_level)?;
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
                let src_view = texture_copy_view(src, region.texture_base.mip_level)?;
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
    colors: &[ColorAttachmentState],
    depth_stencil: DepthStencilAttachmentState,
) -> DeviceResult<()> {
    let Some(first_color) = colors.first() else {
        return Err(crate::DeviceError::Lost);
    };
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            let image_views = colors
                .iter()
                .map(|color| dk::DkImageView::defaults(color.image.0))
                .collect::<Vec<_>>();
            let image_view_ptrs = image_views.iter().map(ptr::from_ref).collect::<Vec<_>>();
            let depth_stencil_image_view = depth_stencil
                .image
                .map(|image| dk::DkImageView::defaults(image.0));
            let depth_stencil_image_view_ptr = depth_stencil_image_view
                .as_ref()
                .map_or(ptr::null(), |view| ptr::from_ref(view));
            dk::dkCmdBufBindRenderTargets(
                cmdbuf,
                image_view_ptrs.as_ptr(),
                image_view_ptrs.len() as u32,
                depth_stencil_image_view_ptr,
            );
            let viewport = dk::DkViewport {
                x: 0.0,
                y: 0.0,
                width: first_color.extent.width as f32,
                height: first_color.extent.height as f32,
                near: 0.0,
                far: 1.0,
            };
            let scissor = dk::DkScissor {
                x: 0,
                y: 0,
                width: first_color.extent.width,
                height: first_color.extent.height,
            };
            dk::dkCmdBufSetViewports(cmdbuf, 0, &viewport, 1);
            dk::dkCmdBufSetScissors(cmdbuf, 0, &scissor, 1);
            for (index, color) in colors.iter().enumerate() {
                if let Some(clear_value) = color.clear_value {
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
    _colors: &[ColorAttachmentState],
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
        .any(|color| color.resolve_image.is_some())
    {
        return Ok(());
    }
    unsafe {
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            for color in target.colors {
                if let Some(resolve_image) = color.resolve_image {
                    let src_view = dk::DkImageView::defaults(color.image.0);
                    let dst_view = dk::DkImageView::defaults(resolve_image.0);
                    dk::dkCmdBufResolveImage(cmdbuf, &src_view, &dst_view);
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
            for group in state.bind_groups.iter().flatten() {
                group
                    .group
                    .bind_descriptor_sets(cmdbuf, &group.dynamic_offsets)?;
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
        submit_deko_commands(queue, surface_queue, |cmdbuf| {
            let shader = [pipeline.compute_shader.raw_shader()];
            dk::dkCmdBufBindShaders(
                cmdbuf,
                dk::DkStageFlag_Compute,
                shader.as_ptr(),
                shader.len() as u32,
            );
            for group in state.bind_groups.iter().flatten() {
                group
                    .group
                    .bind_descriptor_sets(cmdbuf, &group.dynamic_offsets)?;
            }
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
            for group in state.bind_groups.iter().flatten() {
                group
                    .group
                    .bind_descriptor_sets(cmdbuf, &group.dynamic_offsets)?;
            }
            dk::dkCmdBufBindRasterizerState(cmdbuf, &pipeline.rasterizer_state);
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
unsafe fn submit_deko_commands(
    queue: &Queue,
    surface_queue: Option<RawQueueHandle>,
    record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
) -> DeviceResult<()> {
    let raw_queue = surface_queue.unwrap_or_else(|| queue.raw_queue()).0;
    unsafe { queue.record_and_submit(raw_queue, record) }
}
