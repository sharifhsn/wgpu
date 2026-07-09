#![allow(unused_variables)]

#[cfg(target_os = "horizon")]
use alloc::boxed::Box;
#[cfg(target_os = "horizon")]
use alloc::collections::VecDeque;
use alloc::{string::String, vec, vec::Vec};
use core::{cell::UnsafeCell, ptr, sync::atomic::Ordering, time::Duration};

#[cfg(target_os = "horizon")]
use core::ffi::c_void;
use core::fmt;
#[cfg(target_os = "horizon")]
use core::mem::size_of;
#[cfg(target_os = "horizon")]
use core::sync::atomic::{AtomicBool, AtomicU32};

#[cfg(all(not(target_os = "horizon"), supports_64bit_atomics))]
use core::sync::atomic::AtomicU64;
#[cfg(all(not(target_os = "horizon"), not(supports_64bit_atomics)))]
use portable_atomic::AtomicU64;

use crate::TlasInstance;

cfg_if::cfg_if! {
    if #[cfg(supports_ptr_atomics)] {
        use alloc::sync::Arc;
    } else if #[cfg(feature = "portable-atomic")] {
        use portable_atomic_util::Arc;
    }
}

#[cfg(target_os = "horizon")]
use deko3d_sys as dk;

mod buffer;
pub use buffer::Buffer;
mod command;
pub use command::CommandBuffer;

#[derive(Clone, Debug)]
pub struct Api;
#[derive(Debug)]
pub struct Instance;
#[derive(Clone, Debug)]
pub struct Adapter;
pub struct Surface {
    state: UnsafeCell<Option<SurfaceState>>,
}
#[derive(Debug)]
pub struct Device {
    #[allow(dead_code)]
    inner: Arc<DeviceInner>,
}
#[derive(Debug)]
pub struct Queue {
    #[allow(dead_code)]
    device: Arc<DeviceInner>,
    #[allow(dead_code)]
    raw: RawQueue,
    #[cfg(target_os = "horizon")]
    active_cmdbuf: UnsafeCell<Option<dk::DkCmdBuf>>,
}
#[derive(Debug)]
pub struct Encoder;
#[derive(Clone, Debug)]
pub enum Resource {
    Placeholder,
    BindGroupLayout(BindGroupLayoutKind),
    PipelineLayout,
    PipelineCache,
    ShaderModule(Arc<ShaderModuleInner>),
    RenderPipeline(Arc<RenderPipelineInner>),
    ComputePipeline(Arc<ComputePipelineInner>),
    Texture(Arc<TextureInner>),
    Sampler(Arc<SamplerInner>),
    BindGroup(Arc<BindGroupInner>),
    SurfaceTexture {
        slot: i32,
        image: RawImage,
        queue: RawQueueHandle,
        extent: wgt::Extent3d,
    },
    TextureView {
        image: RawImage,
        extent: wgt::Extent3d,
        sample_count: u32,
        view_dimension: wgt::TextureViewDimension,
        base_mip_level: u8,
        mip_level_count: u8,
        base_array_layer: u16,
        array_layer_count: u16,
        owner: Option<Arc<TextureInner>>,
    },
}

#[derive(Clone, Debug)]
pub enum BindGroupLayoutKind {
    TextureSampler {
        texture_binding: u32,
        sampler_binding: u32,
        visibility: wgt::ShaderStages,
    },
    UniformBuffer {
        binding: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
    },
    StorageBuffer {
        binding: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
    },
    StorageTexture {
        binding: u32,
        visibility: wgt::ShaderStages,
        access: wgt::StorageTextureAccess,
        format: wgt::TextureFormat,
    },
    BufferGroup(Vec<BufferBindGroupLayoutKind>),
    BufferStorageTextureGroup {
        buffers: Vec<BufferBindGroupLayoutKind>,
        storage_textures: Vec<StorageTextureBindGroupLayoutKind>,
    },
    BufferTextureSamplerGroup {
        buffers: Vec<BufferBindGroupLayoutKind>,
        texture_samplers: Vec<TextureSamplerBindGroupLayoutKind>,
    },
    BufferTextureSamplerStorageTextureGroup {
        buffers: Vec<BufferBindGroupLayoutKind>,
        texture_samplers: Vec<TextureSamplerBindGroupLayoutKind>,
        storage_textures: Vec<StorageTextureBindGroupLayoutKind>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum BufferBindGroupLayoutKind {
    Uniform {
        binding: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
    },
    Storage {
        binding: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct TextureSamplerBindGroupLayoutKind {
    texture_binding: u32,
    #[allow(dead_code)]
    sampler_binding: u32,
    visibility: wgt::ShaderStages,
}

#[derive(Clone, Copy, Debug)]
pub struct StorageTextureBindGroupLayoutKind {
    binding: u32,
    visibility: wgt::ShaderStages,
    access: wgt::StorageTextureAccess,
    format: wgt::TextureFormat,
}

impl BufferBindGroupLayoutKind {
    fn binding(self) -> u32 {
        match self {
            Self::Uniform { binding, .. } | Self::Storage { binding, .. } => binding,
        }
    }

    fn visibility(self) -> wgt::ShaderStages {
        match self {
            Self::Uniform { visibility, .. } | Self::Storage { visibility, .. } => visibility,
        }
    }
}

pub struct Fence {
    #[cfg(target_os = "horizon")]
    locked: AtomicBool,
    #[cfg(target_os = "horizon")]
    state: UnsafeCell<FenceState>,
    #[cfg(not(target_os = "horizon"))]
    value: AtomicU64,
}

type DeviceResult<T> = Result<T, crate::DeviceError>;

#[derive(Debug)]
struct DeviceInner {
    #[allow(dead_code)]
    raw: RawDevice,
    #[cfg(target_os = "horizon")]
    texture_descriptor_heap: TextureDescriptorHeap,
}

struct SurfaceState {
    #[allow(dead_code)]
    inner: SurfaceStateInner,
}

pub struct ShaderModuleInner {
    #[allow(dead_code)]
    inner: ShaderModuleInnerRaw,
}

pub struct RenderPipelineInner {
    #[allow(dead_code)]
    inner: RenderPipelineInnerRaw,
}

pub struct ComputePipelineInner {
    #[allow(dead_code)]
    inner: ComputePipelineInnerRaw,
}

pub struct TextureInner {
    #[allow(dead_code)]
    inner: TextureInnerRaw,
}

pub struct SamplerInner {
    #[allow(dead_code)]
    inner: SamplerInnerRaw,
}

pub struct BindGroupInner {
    #[allow(dead_code)]
    inner: BindGroupInnerRaw,
}

#[cfg(target_os = "horizon")]
struct RawDevice(dk::DkDevice);

#[cfg(target_os = "horizon")]
struct RawQueue(dk::DkQueue);

#[cfg(target_os = "horizon")]
struct CommandRecorder {
    memory: Box<CommandMemory>,
    cmdbuf: dk::DkCmdBuf,
}

#[cfg(target_os = "horizon")]
struct CommandMemory {
    raw_device: dk::DkDevice,
    blocks: Vec<dk::DkMemBlock>,
    next_block_size: u32,
    allocation_failed: bool,
}

#[cfg(target_os = "horizon")]
struct PendingFence {
    value: crate::FenceValue,
    queue: dk::DkQueue,
    raw: dk::DkFence,
    _recorder: Option<CommandRecorder>,
}

#[cfg(target_os = "horizon")]
struct FenceState {
    completed: crate::FenceValue,
    pending: VecDeque<PendingFence>,
}

#[cfg(target_os = "horizon")]
struct FenceGuard<'a> {
    fence: &'a Fence,
}

#[cfg(target_os = "horizon")]
impl FenceGuard<'_> {
    fn state(&mut self) -> &mut FenceState {
        unsafe { &mut *self.fence.state.get() }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for FenceGuard<'_> {
    fn drop(&mut self) {
        self.fence.locked.store(false, Ordering::Release);
    }
}

#[cfg(target_os = "horizon")]
unsafe impl Send for Fence {}
#[cfg(target_os = "horizon")]
unsafe impl Sync for Fence {}

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy)]
pub struct RawImage(*const dk::DkImage);

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy)]
pub struct RawQueueHandle(dk::DkQueue);

#[cfg(target_os = "horizon")]
struct SurfaceStateInner {
    #[allow(dead_code)]
    device: Arc<DeviceInner>,
    render_queue: dk::DkQueue,
    framebuffer_mem_block: dk::DkMemBlock,
    framebuffers: [dk::DkImage; FRAMEBUFFER_COUNT],
    swapchain: dk::DkSwapchain,
    acquired: bool,
    extent: wgt::Extent3d,
}

#[cfg(target_os = "horizon")]
struct ShaderModuleInnerRaw {
    code_mem_block: dk::DkMemBlock,
    shader: dk::DkShader,
}

#[cfg(target_os = "horizon")]
pub(super) struct RenderPipelineInnerRaw {
    vertex_shader: Arc<ShaderModuleInner>,
    fragment_shader: Arc<ShaderModuleInner>,
    primitive: dk::DkPrimitive,
    vertex_buffers: Vec<dk::DkVtxBufferState>,
    vertex_attributes: Vec<dk::DkVtxAttribState>,
    rasterizer_state: dk::DkRasterizerState,
    color_state: dk::DkColorState,
    color_write_state: dk::DkColorWriteState,
    blend_states: Vec<dk::DkBlendState>,
    depth_stencil_state: dk::DkDepthStencilState,
    multisample_state: dk::DkMultisampleState,
    sample_mask: u32,
    stencil_read_mask: u8,
    stencil_write_mask: u8,
}

#[cfg(target_os = "horizon")]
pub(super) struct ComputePipelineInnerRaw {
    compute_shader: Arc<ShaderModuleInner>,
}

#[cfg(target_os = "horizon")]
struct TextureInnerRaw {
    mem_block: dk::DkMemBlock,
    image: dk::DkImage,
    dimension: wgt::TextureDimension,
    extent: wgt::Extent3d,
    format: wgt::TextureFormat,
    mip_level_count: u32,
    sample_count: u32,
}

#[cfg(target_os = "horizon")]
struct SamplerInnerRaw {
    sampler: dk::DkSampler,
}

#[cfg(target_os = "horizon")]
pub(super) enum BindGroupInnerRaw {
    TextureSampler(TextureSamplerBinding),
    UniformBuffer(UniformBufferBinding),
    StorageBuffer(StorageBufferBinding),
    StorageTexture(StorageTextureBinding),
    BufferGroup(Vec<BufferBinding>),
    BufferTextureSamplerGroup {
        buffers: Vec<BufferBinding>,
        texture_samplers: Vec<TextureSamplerBinding>,
    },
    BufferStorageTextureGroup {
        buffers: Vec<BufferBinding>,
        storage_textures: Vec<StorageTextureBinding>,
    },
    BufferTextureSamplerStorageTextureGroup {
        buffers: Vec<BufferBinding>,
        texture_samplers: Vec<TextureSamplerBinding>,
        storage_textures: Vec<StorageTextureBinding>,
    },
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Debug)]
pub(super) struct UniformBufferBinding {
    binding: u32,
    visibility: wgt::ShaderStages,
    has_dynamic_offset: bool,
    buffer: Buffer,
    offset: wgt::BufferAddress,
    size: Option<wgt::BufferSize>,
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Debug)]
pub(super) enum BufferBinding {
    Uniform(UniformBufferBinding),
    Storage(StorageBufferBinding),
}

#[cfg(target_os = "horizon")]
pub(super) struct TextureSamplerBinding {
    texture_binding: u32,
    visibility: wgt::ShaderStages,
    #[allow(dead_code)]
    sampler_binding: u32,
    #[allow(dead_code)]
    texture: Arc<TextureInner>,
    #[allow(dead_code)]
    sampler: Arc<SamplerInner>,
    image_descriptor_set_gpu_addr: dk::DkGpuAddr,
    sampler_descriptor_set_gpu_addr: dk::DkGpuAddr,
    image_descriptor_gpu_addr: dk::DkGpuAddr,
    sampler_descriptor_gpu_addr: dk::DkGpuAddr,
    image_descriptor_index: u32,
    sampler_descriptor_index: u32,
    descriptor_count: u32,
    image_descriptor: dk::DkImageDescriptor,
    sampler_descriptor: dk::DkSamplerDescriptor,
}

#[cfg(target_os = "horizon")]
#[derive(Debug)]
struct TextureDescriptorHeap {
    mem_block: dk::DkMemBlock,
    image_descriptor_set_gpu_addr: dk::DkGpuAddr,
    sampler_descriptor_set_gpu_addr: dk::DkGpuAddr,
    image_descriptor_stride: u64,
    sampler_descriptor_stride: u64,
    capacity: u32,
    next_slot: AtomicU32,
}

#[cfg(target_os = "horizon")]
struct TextureDescriptorSlot {
    image_descriptor_set_gpu_addr: dk::DkGpuAddr,
    sampler_descriptor_set_gpu_addr: dk::DkGpuAddr,
    image_descriptor_gpu_addr: dk::DkGpuAddr,
    sampler_descriptor_gpu_addr: dk::DkGpuAddr,
    image_descriptor_index: u32,
    sampler_descriptor_index: u32,
    descriptor_count: u32,
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Debug)]
pub(super) struct StorageBufferBinding {
    binding: u32,
    visibility: wgt::ShaderStages,
    read_only: bool,
    has_dynamic_offset: bool,
    buffer: Buffer,
    offset: wgt::BufferAddress,
    size: Option<wgt::BufferSize>,
}

#[cfg(target_os = "horizon")]
pub(super) struct StorageTextureBinding {
    binding: u32,
    visibility: wgt::ShaderStages,
    #[allow(dead_code)]
    access: wgt::StorageTextureAccess,
    #[allow(dead_code)]
    format: wgt::TextureFormat,
    #[allow(dead_code)]
    texture: Arc<TextureInner>,
    image_descriptor_set_gpu_addr: dk::DkGpuAddr,
    image_descriptor_gpu_addr: dk::DkGpuAddr,
    image_descriptor_index: u32,
    descriptor_count: u32,
    image_descriptor: dk::DkImageDescriptor,
}

#[cfg(target_os = "horizon")]
#[repr(C)]
#[derive(Clone, Copy)]
struct DkshHeader {
    magic: u32,
    header_sz: u32,
    control_sz: u32,
    code_sz: u32,
    programs_off: u32,
    num_programs: u32,
}

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
struct RawDevice;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
struct RawQueue;

#[cfg(not(target_os = "horizon"))]
#[derive(Clone, Copy, Debug)]
pub struct RawImage;

#[cfg(not(target_os = "horizon"))]
#[derive(Clone, Copy, Debug)]
pub struct RawQueueHandle;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
struct SurfaceStateInner;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
struct ShaderModuleInnerRaw;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
pub(super) struct RenderPipelineInnerRaw;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
pub(super) struct ComputePipelineInnerRaw;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
struct TextureInnerRaw;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
struct SamplerInnerRaw;

#[cfg(not(target_os = "horizon"))]
#[derive(Debug)]
pub(super) struct BindGroupInnerRaw;

#[cfg(target_os = "horizon")]
const FRAMEBUFFER_COUNT: usize = 2;
const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;
#[cfg(any(target_os = "horizon", test))]
const CMDMEMSIZE: u32 = 16 * 1024;
const DEKO_UNIFORM_BUFFER_COUNT: u32 = 16;
const DEKO_UNIFORM_BUF_MAX_SIZE: u64 = 0x10000;
const DEKO_STORAGE_BUFFER_COUNT: u32 = 16;
const DEKO_IMAGE_BINDING_COUNT: u32 = 8;
#[cfg(target_os = "horizon")]
const DEKO_TEXTURE_DESCRIPTOR_COUNT: u32 = 256;

impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface").finish_non_exhaustive()
    }
}

impl fmt::Debug for SurfaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceState").finish_non_exhaustive()
    }
}

impl fmt::Debug for Fence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fence").finish_non_exhaustive()
    }
}

impl fmt::Debug for ShaderModuleInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShaderModuleInner").finish_non_exhaustive()
    }
}

impl fmt::Debug for RenderPipelineInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderPipelineInner")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for ComputePipelineInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComputePipelineInner")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for TextureInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextureInner").finish_non_exhaustive()
    }
}

impl fmt::Debug for SamplerInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SamplerInner").finish_non_exhaustive()
    }
}

impl fmt::Debug for BindGroupInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BindGroupInner").finish_non_exhaustive()
    }
}

#[cfg(target_os = "horizon")]
impl fmt::Debug for RawDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawDevice").field(&self.0).finish()
    }
}

#[cfg(target_os = "horizon")]
impl fmt::Debug for RawQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawQueue").field(&self.0).finish()
    }
}

#[cfg(target_os = "horizon")]
impl fmt::Debug for CommandRecorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandRecorder")
            .field("memory_block_count", &self.memory.blocks.len())
            .field("cmdbuf", &self.cmdbuf)
            .finish()
    }
}

#[cfg(target_os = "horizon")]
impl fmt::Debug for RawImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawImage").field(&self.0).finish()
    }
}

#[cfg(target_os = "horizon")]
impl fmt::Debug for RawQueueHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawQueueHandle").field(&self.0).finish()
    }
}

unsafe impl Send for DeviceInner {}
unsafe impl Sync for DeviceInner {}
unsafe impl Send for Queue {}
unsafe impl Sync for Queue {}
unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}
unsafe impl Send for Resource {}
unsafe impl Sync for Resource {}
unsafe impl Send for ShaderModuleInner {}
unsafe impl Sync for ShaderModuleInner {}
unsafe impl Send for RenderPipelineInner {}
unsafe impl Sync for RenderPipelineInner {}
unsafe impl Send for ComputePipelineInner {}
unsafe impl Sync for ComputePipelineInner {}
unsafe impl Send for TextureInner {}
unsafe impl Sync for TextureInner {}
unsafe impl Send for SamplerInner {}
unsafe impl Sync for SamplerInner {}
unsafe impl Send for BindGroupInner {}
unsafe impl Sync for BindGroupInner {}
#[cfg(target_os = "horizon")]
unsafe impl Send for RawImage {}
#[cfg(target_os = "horizon")]
unsafe impl Sync for RawImage {}
#[cfg(target_os = "horizon")]
unsafe impl Send for RawQueueHandle {}
#[cfg(target_os = "horizon")]
unsafe impl Sync for RawQueueHandle {}

impl Surface {
    fn new() -> Self {
        Self {
            state: UnsafeCell::new(None),
        }
    }

    unsafe fn state_mut(&self) -> &mut Option<SurfaceState> {
        unsafe { &mut *self.state.get() }
    }

    #[cfg(target_os = "horizon")]
    unsafe fn configured_state(&self) -> Result<&mut SurfaceState, crate::SurfaceError> {
        unsafe { self.state_mut() }
            .as_mut()
            .ok_or(crate::SurfaceError::Other(
                "deko3d surface is not configured",
            ))
    }
}

impl DeviceInner {
    #[cfg(target_os = "horizon")]
    fn raw_device(&self) -> dk::DkDevice {
        self.raw.0
    }

    #[cfg(target_os = "horizon")]
    fn texture_descriptor_heap(&self) -> &TextureDescriptorHeap {
        &self.texture_descriptor_heap
    }
}

#[cfg(target_os = "horizon")]
impl TextureDescriptorHeap {
    unsafe fn new(raw_device: dk::DkDevice) -> DeviceResult<Self> {
        let image_descriptor_stride = u64::from(align_up(
            size_of::<dk::DkImageDescriptor>() as u32,
            dk::DK_IMAGE_DESCRIPTOR_ALIGNMENT,
        ));
        let sampler_descriptor_stride = u64::from(align_up(
            size_of::<dk::DkSamplerDescriptor>() as u32,
            dk::DK_SAMPLER_DESCRIPTOR_ALIGNMENT,
        ));
        let image_descriptor_bytes = image_descriptor_stride
            .checked_mul(u64::from(DEKO_TEXTURE_DESCRIPTOR_COUNT))
            .ok_or(crate::DeviceError::Lost)?;
        let sampler_descriptor_offset = u64::from(align_up(
            u32::try_from(image_descriptor_bytes).map_err(|_| crate::DeviceError::Lost)?,
            dk::DK_SAMPLER_DESCRIPTOR_ALIGNMENT,
        ));
        let sampler_descriptor_bytes = sampler_descriptor_stride
            .checked_mul(u64::from(DEKO_TEXTURE_DESCRIPTOR_COUNT))
            .ok_or(crate::DeviceError::Lost)?;
        let descriptor_bytes = sampler_descriptor_offset
            .checked_add(sampler_descriptor_bytes)
            .ok_or(crate::DeviceError::Lost)?;
        let allocation_size = align_up(
            u32::try_from(descriptor_bytes).map_err(|_| crate::DeviceError::Lost)?,
            dk::DK_MEMBLOCK_ALIGNMENT,
        );

        let mut mem_block_maker = dk::DkMemBlockMaker::defaults(raw_device, allocation_size);
        mem_block_maker.flags = dk::DkMemBlockFlags_CpuUncached | dk::DkMemBlockFlags_GpuCached;
        let mem_block = unsafe { dk::dkMemBlockCreate(&mem_block_maker) };
        if mem_block.is_null() {
            return Err(crate::DeviceError::OutOfMemory);
        }
        let descriptor_gpu_addr = unsafe { dk::dkMemBlockGetGpuAddr(mem_block) };
        if descriptor_gpu_addr == u64::MAX {
            unsafe { dk::dkMemBlockDestroy(mem_block) };
            return Err(crate::DeviceError::Lost);
        }

        Ok(Self {
            mem_block,
            image_descriptor_set_gpu_addr: descriptor_gpu_addr,
            sampler_descriptor_set_gpu_addr: descriptor_gpu_addr + sampler_descriptor_offset,
            image_descriptor_stride,
            sampler_descriptor_stride,
            capacity: DEKO_TEXTURE_DESCRIPTOR_COUNT,
            next_slot: AtomicU32::new(0),
        })
    }

    fn allocate_slot(&self) -> DeviceResult<TextureDescriptorSlot> {
        let index = self
            .next_slot
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |next| {
                (next < self.capacity).then_some(next + 1)
            })
            .map_err(|_| crate::DeviceError::OutOfMemory)?;
        Ok(TextureDescriptorSlot {
            image_descriptor_set_gpu_addr: self.image_descriptor_set_gpu_addr,
            sampler_descriptor_set_gpu_addr: self.sampler_descriptor_set_gpu_addr,
            image_descriptor_gpu_addr: self.image_descriptor_set_gpu_addr
                + self.image_descriptor_stride * u64::from(index),
            sampler_descriptor_gpu_addr: self.sampler_descriptor_set_gpu_addr
                + self.sampler_descriptor_stride * u64::from(index),
            image_descriptor_index: index,
            sampler_descriptor_index: index,
            descriptor_count: self.capacity,
        })
    }

    unsafe fn destroy(&mut self) {
        if !self.mem_block.is_null() {
            unsafe { dk::dkMemBlockDestroy(self.mem_block) };
            self.mem_block = ptr::null_mut();
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for TextureDescriptorHeap {
    fn drop(&mut self) {
        unsafe { self.destroy() };
    }
}

impl ShaderModuleInner {
    #[cfg(target_os = "horizon")]
    pub(super) fn raw_shader(&self) -> *const dk::DkShader {
        &self.inner.shader
    }
}

impl RenderPipelineInner {
    #[cfg(target_os = "horizon")]
    pub(super) fn raw(&self) -> &RenderPipelineInnerRaw {
        &self.inner
    }
}

impl ComputePipelineInner {
    #[cfg(target_os = "horizon")]
    pub(super) fn raw(&self) -> &ComputePipelineInnerRaw {
        &self.inner
    }
}

impl TextureInner {
    #[cfg(target_os = "horizon")]
    pub(super) fn raw_image(&self) -> RawImage {
        RawImage(&self.inner.image)
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn raw_image(&self) -> RawImage {
        RawImage
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn format(&self) -> wgt::TextureFormat {
        self.inner.format
    }

    #[cfg(not(target_os = "horizon"))]
    #[allow(dead_code)]
    pub(super) fn format(&self) -> wgt::TextureFormat {
        wgt::TextureFormat::Rgba8Unorm
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn dimension(&self) -> wgt::TextureDimension {
        self.inner.dimension
    }

    #[cfg(not(target_os = "horizon"))]
    #[allow(dead_code)]
    pub(super) fn dimension(&self) -> wgt::TextureDimension {
        wgt::TextureDimension::D2
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn mip_level_count(&self) -> u32 {
        self.inner.mip_level_count
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn mip_level_count(&self) -> u32 {
        1
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn mip_extent(&self, mip_level: u32) -> DeviceResult<wgt::Extent3d> {
        if mip_level >= self.inner.mip_level_count {
            return Err(crate::DeviceError::Lost);
        }
        Ok(wgt::Extent3d {
            width: mip_dimension(self.inner.extent.width, mip_level),
            height: mip_dimension(self.inner.extent.height, mip_level),
            depth_or_array_layers: if self.inner.dimension == wgt::TextureDimension::D3 {
                mip_dimension(self.inner.extent.depth_or_array_layers, mip_level)
            } else {
                self.inner.extent.depth_or_array_layers
            },
        })
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn mip_extent(&self, _mip_level: u32) -> DeviceResult<wgt::Extent3d> {
        Ok(wgt::Extent3d {
            width: 0,
            height: 0,
            depth_or_array_layers: 1,
        })
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn sample_count(&self) -> u32 {
        self.inner.sample_count
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn sample_count(&self) -> u32 {
        1
    }
}

impl BindGroupInner {
    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn bind_descriptor_sets(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
    ) -> DeviceResult<()> {
        match &self.inner {
            BindGroupInnerRaw::TextureSampler(binding) => unsafe {
                binding.bind(cmdbuf, dynamic_offsets)?;
            },
            BindGroupInnerRaw::UniformBuffer(binding) => unsafe {
                binding.bind(cmdbuf, dynamic_offsets)?;
            },
            BindGroupInnerRaw::StorageBuffer(binding) => unsafe {
                binding.bind(cmdbuf, dynamic_offsets)?;
            },
            BindGroupInnerRaw::StorageTexture(binding) => unsafe {
                binding.bind(cmdbuf, dynamic_offsets)?;
            },
            BindGroupInnerRaw::BufferGroup(bindings) => {
                let mut dynamic_offsets = dynamic_offsets.iter();
                for binding in bindings {
                    let dynamic_offset = if binding.has_dynamic_offset() {
                        Some(*dynamic_offsets.next().ok_or(crate::DeviceError::Lost)?)
                    } else {
                        None
                    };
                    unsafe { binding.bind(cmdbuf, dynamic_offset)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
            }
            BindGroupInnerRaw::BufferTextureSamplerGroup {
                buffers,
                texture_samplers,
            } => {
                let mut dynamic_offsets = dynamic_offsets.iter();
                for binding in buffers {
                    let dynamic_offset = if binding.has_dynamic_offset() {
                        Some(*dynamic_offsets.next().ok_or(crate::DeviceError::Lost)?)
                    } else {
                        None
                    };
                    unsafe { binding.bind(cmdbuf, dynamic_offset)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                for texture_sampler in texture_samplers {
                    unsafe { texture_sampler.bind(cmdbuf, &[])? };
                }
            }
            BindGroupInnerRaw::BufferStorageTextureGroup {
                buffers,
                storage_textures,
            } => {
                let mut dynamic_offsets = dynamic_offsets.iter();
                for binding in buffers {
                    let dynamic_offset = if binding.has_dynamic_offset() {
                        Some(*dynamic_offsets.next().ok_or(crate::DeviceError::Lost)?)
                    } else {
                        None
                    };
                    unsafe { binding.bind(cmdbuf, dynamic_offset)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                for storage_texture in storage_textures {
                    unsafe { storage_texture.bind(cmdbuf, &[])? };
                }
            }
            BindGroupInnerRaw::BufferTextureSamplerStorageTextureGroup {
                buffers,
                texture_samplers,
                storage_textures,
            } => {
                let mut dynamic_offsets = dynamic_offsets.iter();
                for binding in buffers {
                    let dynamic_offset = if binding.has_dynamic_offset() {
                        Some(*dynamic_offsets.next().ok_or(crate::DeviceError::Lost)?)
                    } else {
                        None
                    };
                    unsafe { binding.bind(cmdbuf, dynamic_offset)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                unsafe {
                    for texture_sampler in texture_samplers {
                        texture_sampler.bind(cmdbuf, &[])?;
                    }
                    for storage_texture in storage_textures {
                        storage_texture.bind(cmdbuf, &[])?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "horizon")]
impl TextureSamplerBinding {
    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
    ) -> DeviceResult<()> {
        if !dynamic_offsets.is_empty() {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            dk::dkCmdBufPushData(
                cmdbuf,
                self.image_descriptor_gpu_addr,
                ptr::addr_of!(self.image_descriptor).cast(),
                size_of::<dk::DkImageDescriptor>() as u32,
            );
            dk::dkCmdBufPushData(
                cmdbuf,
                self.sampler_descriptor_gpu_addr,
                ptr::addr_of!(self.sampler_descriptor).cast(),
                size_of::<dk::DkSamplerDescriptor>() as u32,
            );
            dk::dkCmdBufBindImageDescriptorSet(
                cmdbuf,
                self.image_descriptor_set_gpu_addr,
                self.descriptor_count,
            );
            dk::dkCmdBufBindSamplerDescriptorSet(
                cmdbuf,
                self.sampler_descriptor_set_gpu_addr,
                self.descriptor_count,
            );
            let handle =
                dk::dkMakeTextureHandle(self.image_descriptor_index, self.sampler_descriptor_index);
            if self.visibility.contains(wgt::ShaderStages::VERTEX) {
                dk::dkCmdBufBindTexture(
                    cmdbuf,
                    dk::DkStage::DkStage_Vertex,
                    self.texture_binding,
                    handle,
                );
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                dk::dkCmdBufBindTexture(
                    cmdbuf,
                    dk::DkStage::DkStage_Fragment,
                    self.texture_binding,
                    handle,
                );
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                dk::dkCmdBufBindTexture(
                    cmdbuf,
                    dk::DkStage::DkStage_Compute,
                    self.texture_binding,
                    handle,
                );
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "horizon")]
impl UniformBufferBinding {
    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
    ) -> DeviceResult<()> {
        let dynamic_offset = match (self.has_dynamic_offset, dynamic_offsets) {
            (true, [offset]) => u64::from(*offset),
            (false, []) => 0,
            _ => return Err(crate::DeviceError::Lost),
        };
        if dynamic_offset % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0 {
            return Err(crate::DeviceError::Lost);
        }
        let offset = self
            .offset
            .checked_add(dynamic_offset)
            .ok_or(crate::DeviceError::Lost)?;
        let (gpu_addr, gpu_size) = self.buffer.gpu_binding(offset, self.size)?;
        if gpu_addr % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0
            || gpu_size > dk::DK_UNIFORM_BUF_MAX_SIZE
        {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            if self.visibility.contains(wgt::ShaderStages::VERTEX) {
                dk::dkCmdBufBindUniformBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Vertex,
                    self.binding,
                    gpu_addr,
                    gpu_size,
                );
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                dk::dkCmdBufBindUniformBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Fragment,
                    self.binding,
                    gpu_addr,
                    gpu_size,
                );
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                dk::dkCmdBufBindUniformBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Compute,
                    self.binding,
                    gpu_addr,
                    gpu_size,
                );
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "horizon")]
impl BufferBinding {
    fn has_dynamic_offset(&self) -> bool {
        match self {
            Self::Uniform(binding) => binding.has_dynamic_offset,
            Self::Storage(binding) => binding.has_dynamic_offset,
        }
    }

    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offset: Option<wgt::DynamicOffset>,
    ) -> DeviceResult<()> {
        match self {
            Self::Uniform(binding) => match dynamic_offset {
                Some(offset) => unsafe { binding.bind(cmdbuf, &[offset]) },
                None => unsafe { binding.bind(cmdbuf, &[]) },
            },
            Self::Storage(binding) => match dynamic_offset {
                Some(offset) => unsafe { binding.bind(cmdbuf, &[offset]) },
                None => unsafe { binding.bind(cmdbuf, &[]) },
            },
        }
    }
}

#[cfg(target_os = "horizon")]
impl StorageBufferBinding {
    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
    ) -> DeviceResult<()> {
        let dynamic_offset = match (self.has_dynamic_offset, dynamic_offsets) {
            (true, [offset]) => u64::from(*offset),
            (false, []) => 0,
            _ => return Err(crate::DeviceError::Lost),
        };
        if dynamic_offset % u64::from(wgt::STORAGE_BINDING_SIZE_ALIGNMENT) != 0 {
            return Err(crate::DeviceError::Lost);
        }
        let _storage_is_read_only = self.read_only;
        let offset = self
            .offset
            .checked_add(dynamic_offset)
            .ok_or(crate::DeviceError::Lost)?;
        let (gpu_addr, gpu_size) = self.buffer.gpu_binding(offset, self.size)?;
        unsafe {
            if self.visibility.contains(wgt::ShaderStages::VERTEX) {
                dk::dkCmdBufBindStorageBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Vertex,
                    self.binding,
                    gpu_addr,
                    gpu_size,
                );
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                dk::dkCmdBufBindStorageBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Fragment,
                    self.binding,
                    gpu_addr,
                    gpu_size,
                );
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                dk::dkCmdBufBindStorageBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Compute,
                    self.binding,
                    gpu_addr,
                    gpu_size,
                );
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "horizon")]
impl StorageTextureBinding {
    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
    ) -> DeviceResult<()> {
        if !dynamic_offsets.is_empty() {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            dk::dkCmdBufPushData(
                cmdbuf,
                self.image_descriptor_gpu_addr,
                ptr::addr_of!(self.image_descriptor).cast(),
                size_of::<dk::DkImageDescriptor>() as u32,
            );
            dk::dkCmdBufBindImageDescriptorSet(
                cmdbuf,
                self.image_descriptor_set_gpu_addr,
                self.descriptor_count,
            );
            let handle = dk::dkMakeImageHandle(self.image_descriptor_index);
            if self.visibility.contains(wgt::ShaderStages::VERTEX) {
                dk::dkCmdBufBindImage(cmdbuf, dk::DkStage::DkStage_Vertex, self.binding, handle);
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                dk::dkCmdBufBindImage(cmdbuf, dk::DkStage::DkStage_Fragment, self.binding, handle);
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                dk::dkCmdBufBindImage(cmdbuf, dk::DkStage::DkStage_Compute, self.binding, handle);
            }
        }
        Ok(())
    }
}

impl Queue {
    #[cfg(target_os = "horizon")]
    unsafe fn begin_recording(&self, cmdbuf: dk::DkCmdBuf) -> DeviceResult<()> {
        let active = unsafe { &mut *self.active_cmdbuf.get() };
        if active.is_some() {
            return Err(crate::DeviceError::Lost);
        }
        *active = Some(cmdbuf);
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    unsafe fn end_recording(&self) {
        let active = unsafe { &mut *self.active_cmdbuf.get() };
        *active = None;
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn active_cmdbuf(&self) -> DeviceResult<dk::DkCmdBuf> {
        unsafe { *self.active_cmdbuf.get() }.ok_or(crate::DeviceError::Lost)
    }
}

#[cfg(target_os = "horizon")]
impl CommandRecorder {
    unsafe fn new(raw_device: dk::DkDevice) -> DeviceResult<Self> {
        let mut memory = Box::new(CommandMemory::new(raw_device));
        let mut cmdbuf_maker = dk::DkCmdBufMaker::defaults(raw_device);
        let user_data: *mut CommandMemory = &mut *memory;
        cmdbuf_maker.userData = user_data.cast::<c_void>();
        cmdbuf_maker.cbAddMem = Some(command_recorder_add_memory);
        let cmdbuf = unsafe { dk::dkCmdBufCreate(&cmdbuf_maker) };
        if cmdbuf.is_null() {
            return Err(crate::DeviceError::Lost);
        }

        let mut recorder = Self { memory, cmdbuf };
        if !unsafe { recorder.memory.add_block(cmdbuf, CMDMEMSIZE as usize) } {
            return Err(crate::DeviceError::OutOfMemory);
        }
        Ok(recorder)
    }

    fn check_memory(&self) -> DeviceResult<()> {
        if self.memory.allocation_failed {
            Err(crate::DeviceError::OutOfMemory)
        } else {
            Ok(())
        }
    }

    unsafe fn submit(&mut self, raw_queue: dk::DkQueue, fence: &mut dk::DkFence) {
        let cmds = unsafe { dk::dkCmdBufFinishList(self.cmdbuf) };
        unsafe {
            dk::dkQueueSubmitCommands(raw_queue, cmds);
            dk::dkQueueSignalFence(raw_queue, fence, false);
            dk::dkQueueFlush(raw_queue);
        }
    }
}

#[cfg(target_os = "horizon")]
impl CommandMemory {
    fn new(raw_device: dk::DkDevice) -> Self {
        Self {
            raw_device,
            blocks: Vec::new(),
            next_block_size: CMDMEMSIZE,
            allocation_failed: false,
        }
    }

    unsafe fn add_block(&mut self, cmdbuf: dk::DkCmdBuf, min_req_size: usize) -> bool {
        let Some(allocation_size) =
            command_memory_block_size(min_req_size, self.next_block_size, dk::DK_CMDMEM_ALIGNMENT)
        else {
            self.allocation_failed = true;
            return false;
        };
        if self.blocks.try_reserve(1).is_err() {
            self.allocation_failed = true;
            return false;
        }

        let mut maker = dk::DkMemBlockMaker::defaults(self.raw_device, allocation_size);
        maker.flags = dk::DkMemBlockFlags_CpuUncached | dk::DkMemBlockFlags_GpuCached;
        let block = unsafe { dk::dkMemBlockCreate(&maker) };
        if block.is_null() {
            self.allocation_failed = true;
            return false;
        }

        unsafe { dk::dkCmdBufAddMemory(cmdbuf, block, 0, allocation_size) };
        self.blocks.push(block);
        self.next_block_size = allocation_size.checked_mul(2).unwrap_or(allocation_size);
        true
    }
}

#[cfg(target_os = "horizon")]
impl Drop for CommandMemory {
    fn drop(&mut self) {
        for block in self.blocks.drain(..) {
            if !block.is_null() {
                unsafe { dk::dkMemBlockDestroy(block) };
            }
        }
    }
}

#[cfg(target_os = "horizon")]
unsafe extern "C" fn command_recorder_add_memory(
    user_data: *mut c_void,
    cmdbuf: dk::DkCmdBuf,
    min_req_size: usize,
) {
    let Some(memory) = (unsafe { (user_data as *mut CommandMemory).as_mut() }) else {
        return;
    };
    let _ = unsafe { memory.add_block(cmdbuf, min_req_size) };
}

#[cfg(any(target_os = "horizon", test))]
fn command_memory_block_size(
    min_req_size: usize,
    next_block_size: u32,
    alignment: u32,
) -> Option<u32> {
    if alignment == 0 {
        return None;
    }
    let min_req_size = u32::try_from(min_req_size).ok()?;
    let requested = min_req_size.max(next_block_size).max(CMDMEMSIZE);
    let rounded = requested.checked_add(alignment - 1)? / alignment * alignment;
    Some(rounded)
}

#[cfg(target_os = "horizon")]
unsafe fn poll_fence_state(state: &mut FenceState) -> DeviceResult<()> {
    loop {
        let Some(pending) = state.pending.front_mut() else {
            return Ok(());
        };
        match unsafe { dk::dkFenceWait(&mut pending.raw, 0) } {
            dk::DkResult::DkResult_Success => {
                let value = pending.value;
                state.pending.pop_front();
                state.completed = state.completed.max(value);
            }
            dk::DkResult::DkResult_Timeout => return Ok(()),
            _ => return Err(crate::DeviceError::Lost),
        }
    }
}

impl Fence {
    #[cfg(target_os = "horizon")]
    fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(FenceState {
                completed: 0,
                pending: VecDeque::new(),
            }),
        }
    }

    #[cfg(not(target_os = "horizon"))]
    fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    #[cfg(target_os = "horizon")]
    fn lock(&self) -> FenceGuard<'_> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        FenceGuard { fence: self }
    }

    #[cfg(target_os = "horizon")]
    unsafe fn push_submission(
        &self,
        value: crate::FenceValue,
        queue: dk::DkQueue,
        raw: dk::DkFence,
        recorder: Option<CommandRecorder>,
    ) -> DeviceResult<()> {
        let mut guard = self.lock();
        guard.state().pending.push_back(PendingFence {
            value,
            queue,
            raw,
            _recorder: recorder,
        });
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    unsafe fn wait_on_previous_queue(&self, queue: dk::DkQueue) {
        let mut guard = self.lock();
        let Some(previous) = guard.state().pending.back_mut() else {
            return;
        };
        if previous.queue != queue {
            unsafe { dk::dkQueueWaitFence(queue, &mut previous.raw) };
        }
    }

    #[cfg(target_os = "horizon")]
    unsafe fn completed_value(&self) -> DeviceResult<crate::FenceValue> {
        let mut guard = self.lock();
        unsafe { poll_fence_state(guard.state()) }?;
        Ok(guard.state().completed)
    }

    #[cfg(target_os = "horizon")]
    unsafe fn wait_for(
        &self,
        value: crate::FenceValue,
        timeout: Option<Duration>,
    ) -> DeviceResult<bool> {
        let mut guard = self.lock();
        let state = guard.state();
        unsafe { poll_fence_state(state) }?;
        if state.completed >= value {
            return Ok(true);
        }

        let Some(index) = state
            .pending
            .iter()
            .position(|pending| pending.value >= value)
        else {
            return Ok(false);
        };
        let timeout_ns = match timeout {
            None => -1,
            Some(duration) => i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX),
        };
        let result = unsafe { dk::dkFenceWait(&mut state.pending[index].raw, timeout_ns) };
        match result {
            dk::DkResult::DkResult_Success => {
                let completed = state.pending[index].value;
                for _ in 0..=index {
                    state.pending.pop_front();
                }
                state.completed = state.completed.max(completed);
                Ok(state.completed >= value)
            }
            dk::DkResult::DkResult_Timeout => Ok(false),
            _ => Err(crate::DeviceError::Lost),
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for Fence {
    fn drop(&mut self) {
        let mut guard = self.lock();
        for pending in &mut guard.state().pending {
            unsafe {
                let _ = dk::dkFenceWait(&mut pending.raw, -1);
            }
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for DeviceInner {
    fn drop(&mut self) {
        unsafe { self.texture_descriptor_heap.destroy() };
        if !self.raw.0.is_null() {
            unsafe { dk::dkDeviceDestroy(self.raw.0) };
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for Queue {
    fn drop(&mut self) {
        if !self.raw.0.is_null() {
            unsafe {
                dk::dkQueueWaitIdle(self.raw.0);
                dk::dkQueueDestroy(self.raw.0);
            }
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for CommandRecorder {
    fn drop(&mut self) {
        unsafe {
            if !self.cmdbuf.is_null() {
                dk::dkCmdBufDestroy(self.cmdbuf);
            }
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for ShaderModuleInnerRaw {
    fn drop(&mut self) {
        if !self.code_mem_block.is_null() {
            unsafe { dk::dkMemBlockDestroy(self.code_mem_block) };
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for TextureInnerRaw {
    fn drop(&mut self) {
        if !self.mem_block.is_null() {
            unsafe { dk::dkMemBlockDestroy(self.mem_block) };
        }
    }
}

#[cfg(target_os = "horizon")]
impl SurfaceState {
    unsafe fn new(
        device: Arc<DeviceInner>,
        config: &crate::SurfaceConfiguration,
    ) -> Result<Self, crate::SurfaceError> {
        let Some(image_format) = map_color_image_format(config.format) else {
            return Err(crate::SurfaceError::Other(
                "deko3d surface only supports RGBA8/BGRA8 unorm and sRGB formats for now",
            ));
        };
        if config.present_mode != wgt::PresentMode::Fifo {
            return Err(crate::SurfaceError::Other(
                "deko3d surface only supports FIFO present mode for now",
            ));
        }
        if !config.usage.contains(wgt::TextureUses::COLOR_TARGET) {
            return Err(crate::SurfaceError::Other(
                "deko3d surface requires COLOR_TARGET usage",
            ));
        }
        if config.extent.width == 0
            || config.extent.height == 0
            || config.extent.depth_or_array_layers != 1
        {
            return Err(crate::SurfaceError::Other(
                "deko3d surface requires a non-empty 2D extent",
            ));
        }

        let raw_device = device.raw_device();
        let mut image_layout_maker = dk::DkImageLayoutMaker::defaults(raw_device);
        image_layout_maker.flags = dk::DkImageFlags_UsageRender
            | dk::DkImageFlags_UsagePresent
            | dk::DkImageFlags_HwCompression;
        image_layout_maker.format = image_format;
        image_layout_maker.dimensions[0] = config.extent.width;
        image_layout_maker.dimensions[1] = config.extent.height;

        let mut framebuffer_layout = dk::DkImageLayout::zeroed();
        unsafe { dk::dkImageLayoutInitialize(&mut framebuffer_layout, &image_layout_maker) };

        let framebuffer_align = unsafe { dk::dkImageLayoutGetAlignment(&framebuffer_layout) };
        let framebuffer_size = align_up(
            unsafe { dk::dkImageLayoutGetSize(&framebuffer_layout) } as u32,
            framebuffer_align,
        );

        let mut mem_block_maker =
            dk::DkMemBlockMaker::defaults(raw_device, FRAMEBUFFER_COUNT as u32 * framebuffer_size);
        mem_block_maker.flags = dk::DkMemBlockFlags_GpuCached | dk::DkMemBlockFlags_Image;
        let framebuffer_mem_block = unsafe { dk::dkMemBlockCreate(&mem_block_maker) };
        if framebuffer_mem_block.is_null() {
            return Err(crate::SurfaceError::Device(crate::DeviceError::OutOfMemory));
        }

        let mut framebuffers = [dk::DkImage::zeroed(), dk::DkImage::zeroed()];
        let mut swapchain_images = [ptr::null(); FRAMEBUFFER_COUNT];
        for (i, image) in framebuffers.iter_mut().enumerate() {
            let image_ptr: *const dk::DkImage = image;
            swapchain_images[i] = image_ptr;
            unsafe {
                dk::dkImageInitialize(
                    image,
                    &framebuffer_layout,
                    framebuffer_mem_block,
                    i as u32 * framebuffer_size,
                )
            };
        }

        let swapchain_maker = dk::DkSwapchainMaker::defaults(
            raw_device,
            unsafe { dk::nwindowGetDefault() }.cast(),
            swapchain_images.as_ptr(),
            FRAMEBUFFER_COUNT as u32,
        );
        let swapchain = unsafe { dk::dkSwapchainCreate(&swapchain_maker) };
        if swapchain.is_null() {
            unsafe { dk::dkMemBlockDestroy(framebuffer_mem_block) };
            return Err(crate::SurfaceError::Other(
                "deko3d swapchain creation failed",
            ));
        }

        let mut queue_maker = dk::DkQueueMaker::defaults(raw_device);
        queue_maker.flags = dk::DkQueueFlags_Graphics;
        let render_queue = unsafe { dk::dkQueueCreate(&queue_maker) };
        if render_queue.is_null() {
            unsafe {
                dk::dkSwapchainDestroy(swapchain);
                dk::dkMemBlockDestroy(framebuffer_mem_block);
            }
            return Err(crate::SurfaceError::Device(crate::DeviceError::Lost));
        }

        Ok(Self {
            inner: SurfaceStateInner {
                device,
                render_queue,
                framebuffer_mem_block,
                framebuffers,
                swapchain,
                acquired: false,
                extent: config.extent,
            },
        })
    }
}

#[cfg(target_os = "horizon")]
impl ShaderModuleInner {
    unsafe fn new_dksh(raw_device: dk::DkDevice, bytes: &[u8]) -> Result<Self, crate::ShaderError> {
        if bytes.len() < size_of::<DkshHeader>() {
            return Err(shader_error("deko3d DKSH input is shorter than its header"));
        }

        let header = unsafe { ptr::read_unaligned(bytes.as_ptr().cast::<DkshHeader>()) };
        if header.control_sz == 0 || header.code_sz == 0 || header.num_programs == 0 {
            return Err(shader_error("deko3d DKSH input has an invalid header"));
        }

        let control_len = usize::try_from(header.control_sz)
            .map_err(|_| shader_error("deko3d DKSH control section is too large"))?;
        let code_len = usize::try_from(header.code_sz)
            .map_err(|_| shader_error("deko3d DKSH code section is too large"))?;
        let total_len = control_len
            .checked_add(code_len)
            .ok_or_else(|| shader_error("deko3d DKSH sections overflow"))?;
        if total_len > bytes.len() {
            return Err(shader_error("deko3d DKSH sections exceed input length"));
        }

        let code_allocation = align_up(
            header
                .code_sz
                .checked_add(dk::DK_SHADER_CODE_UNUSABLE_SIZE)
                .ok_or_else(|| shader_error("deko3d DKSH code size overflows"))?,
            dk::DK_MEMBLOCK_ALIGNMENT,
        );
        let mut mem_block_maker = dk::DkMemBlockMaker::defaults(raw_device, code_allocation);
        mem_block_maker.flags = dk::DkMemBlockFlags_CpuUncached
            | dk::DkMemBlockFlags_GpuCached
            | dk::DkMemBlockFlags_Code;
        let code_mem_block = unsafe { dk::dkMemBlockCreate(&mem_block_maker) };
        if code_mem_block.is_null() {
            return Err(crate::ShaderError::Device(crate::DeviceError::OutOfMemory));
        }

        let result = unsafe {
            let code_dst = dk::dkMemBlockGetCpuAddr(code_mem_block);
            if code_dst.is_null() {
                Err(shader_error("deko3d code memory is not CPU-visible"))
            } else {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr().add(control_len),
                    code_dst.cast::<u8>(),
                    code_len,
                );
                let mut shader = dk::DkShader::zeroed();
                let mut shader_maker = dk::DkShaderMaker::defaults(code_mem_block, 0);
                shader_maker.control = bytes.as_ptr().cast();
                dk::dkShaderInitialize(&mut shader, &shader_maker);
                if dk::dkShaderIsValid(&shader) {
                    Ok(Self {
                        inner: ShaderModuleInnerRaw {
                            code_mem_block,
                            shader,
                        },
                    })
                } else {
                    Err(shader_error("deko3d DKSH shader initialization failed"))
                }
            }
        };

        if result.is_err() {
            unsafe { dk::dkMemBlockDestroy(code_mem_block) };
        }
        result
    }
}

#[cfg(target_os = "horizon")]
impl RenderPipelineInner {
    fn new(
        desc: &crate::RenderPipelineDescriptor<Resource, Resource, Resource>,
    ) -> Result<Self, crate::PipelineError> {
        if desc.multiview_mask.is_some()
            || desc.multisample.alpha_to_coverage_enabled
            || !matches!(desc.cache, None | Some(Resource::PipelineCache))
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let ms_mode = map_sample_count(desc.multisample.count)
            .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
        let sample_mask = u32::try_from(desc.multisample.mask)
            .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
        let primitive = map_primitive_topology(&desc.primitive)?;
        if desc.primitive.strip_index_format.is_some()
            || desc.primitive.unclipped_depth
            || desc.primitive.polygon_mode != wgt::PolygonMode::Fill
            || desc.primitive.conservative
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let (color_state, color_write_state, blend_states) = map_color_targets(desc.color_targets)?;
        let rasterizer_state = map_rasterizer_state(&desc.primitive);
        let depth_stencil_state = map_depth_stencil_state(desc.depth_stencil.as_ref())?;
        let mut multisample_state = dk::DkMultisampleState::defaults();
        multisample_state.set_mode(ms_mode);
        multisample_state.set_rasterizer_mode(ms_mode);

        let crate::VertexProcessor::Standard {
            vertex_buffers,
            vertex_stage,
        } = &desc.vertex_processor
        else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let Some(fragment_stage) = &desc.fragment_stage else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        // Deko3D DKSH blobs are already compiled for a concrete shader program.
        // wgpu-core validates passthrough/Naga entry-point metadata before HAL
        // pipeline creation, and Deko3D does not inspect the source-level name.

        let Resource::ShaderModule(vertex_shader) = vertex_stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let Resource::ShaderModule(fragment_shader) = fragment_stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };

        let mut vertex_buffer_states = Vec::new();
        let mut vertex_attributes = Vec::new();
        for (buffer_id, layout) in vertex_buffers.iter().enumerate() {
            let Some(layout) = layout else {
                continue;
            };
            if layout.step_mode != wgt::VertexStepMode::Vertex {
                return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
            }
            vertex_buffer_states.push(dk::DkVtxBufferState {
                stride: u32::try_from(layout.array_stride)
                    .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                divisor: 0,
            });
            for attribute in layout.attributes {
                let (size, type_) = map_vertex_format(attribute.format)?;
                vertex_attributes.push(dk::DkVtxAttribState::new(
                    u32::try_from(buffer_id)
                        .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                    false,
                    u32::try_from(attribute.offset)
                        .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                    size,
                    type_,
                    false,
                ));
            }
        }

        Ok(Self {
            inner: RenderPipelineInnerRaw {
                vertex_shader: vertex_shader.clone(),
                fragment_shader: fragment_shader.clone(),
                primitive,
                vertex_buffers: vertex_buffer_states,
                vertex_attributes,
                rasterizer_state,
                color_state,
                color_write_state,
                blend_states,
                depth_stencil_state,
                multisample_state,
                sample_mask,
                stencil_read_mask: desc
                    .depth_stencil
                    .as_ref()
                    .map_or(0xFF, |depth_stencil| depth_stencil.stencil.read_mask as u8),
                stencil_write_mask: desc
                    .depth_stencil
                    .as_ref()
                    .map_or(0xFF, |depth_stencil| depth_stencil.stencil.write_mask as u8),
            },
        })
    }
}

#[cfg(target_os = "horizon")]
impl ComputePipelineInner {
    fn new(
        desc: &crate::ComputePipelineDescriptor<Resource, Resource, Resource>,
    ) -> Result<Self, crate::PipelineError> {
        if !matches!(desc.layout, Resource::PipelineLayout)
            || !matches!(desc.cache, None | Some(Resource::PipelineCache))
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }

        let Resource::ShaderModule(compute_shader) = desc.stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };

        Ok(Self {
            inner: ComputePipelineInnerRaw {
                compute_shader: compute_shader.clone(),
            },
        })
    }
}

#[cfg(target_os = "horizon")]
fn map_primitive_topology(
    primitive: &wgt::PrimitiveState,
) -> Result<dk::DkPrimitive, crate::PipelineError> {
    match primitive.topology {
        wgt::PrimitiveTopology::PointList => Ok(dk::DkPrimitive::DkPrimitive_Points),
        wgt::PrimitiveTopology::LineList => Ok(dk::DkPrimitive::DkPrimitive_Lines),
        wgt::PrimitiveTopology::LineStrip => Ok(dk::DkPrimitive::DkPrimitive_LineStrip),
        wgt::PrimitiveTopology::TriangleList => Ok(dk::DkPrimitive::DkPrimitive_Triangles),
        wgt::PrimitiveTopology::TriangleStrip => Ok(dk::DkPrimitive::DkPrimitive_TriangleStrip),
    }
}

#[cfg(target_os = "horizon")]
fn map_sample_count(sample_count: u32) -> DeviceResult<dk::DkMsMode> {
    match sample_count {
        1 => Ok(dk::DkMsMode::DkMsMode_1x),
        4 => Ok(dk::DkMsMode::DkMsMode_4x),
        _ => Err(crate::DeviceError::Lost),
    }
}

#[cfg(target_os = "horizon")]
fn map_rasterizer_state(primitive: &wgt::PrimitiveState) -> dk::DkRasterizerState {
    let mut state = dk::DkRasterizerState::defaults();
    let cull_mode = match primitive.cull_mode {
        None => dk::DkFace_None,
        Some(wgt::Face::Front) => dk::DkFace_Front,
        Some(wgt::Face::Back) => dk::DkFace_Back,
    };
    let front_face = match primitive.front_face {
        wgt::FrontFace::Cw => dk::DkFrontFace_CW,
        wgt::FrontFace::Ccw => dk::DkFrontFace_CCW,
    };
    state.set_cull_mode(cull_mode);
    state.set_front_face(front_face);
    state
}

#[cfg(target_os = "horizon")]
fn map_depth_stencil_state(
    depth_stencil: Option<&wgt::DepthStencilState>,
) -> Result<dk::DkDepthStencilState, crate::PipelineError> {
    let mut state = dk::DkDepthStencilState::defaults();
    state.set_depth_test_enable(false);
    state.set_depth_write_enable(false);
    state.set_stencil_test_enable(false);
    state.set_depth_compare_op(dk::DkCompareOp::DkCompareOp_Always);

    let Some(depth_stencil) = depth_stencil else {
        return Ok(state);
    };

    if depth_stencil.bias.is_enabled()
        || (depth_stencil.format != wgt::TextureFormat::Depth32Float
            && depth_stencil.format != wgt::TextureFormat::Stencil8)
    {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }

    if depth_stencil.format == wgt::TextureFormat::Stencil8
        && (depth_stencil.depth_compare.is_some()
            || depth_stencil.depth_write_enabled.unwrap_or(false))
    {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }
    if depth_stencil.format == wgt::TextureFormat::Depth32Float
        && depth_stencil.stencil.is_enabled()
    {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }

    if let Some(compare) = depth_stencil.depth_compare {
        state.set_depth_test_enable(true);
        state.set_depth_compare_op(map_compare_function(compare));
    }
    state.set_depth_write_enable(depth_stencil.depth_write_enabled.unwrap_or(false));
    if depth_stencil.stencil.is_enabled() {
        state.set_stencil_test_enable(true);
        map_stencil_face_state(&mut state, true, depth_stencil.stencil.front)?;
        map_stencil_face_state(&mut state, false, depth_stencil.stencil.back)?;
    }

    Ok(state)
}

#[cfg(target_os = "horizon")]
fn map_stencil_face_state(
    state: &mut dk::DkDepthStencilState,
    front: bool,
    face: wgt::StencilFaceState,
) -> Result<(), crate::PipelineError> {
    let fail = map_stencil_operation(face.fail_op);
    let pass = map_stencil_operation(face.pass_op);
    let depth_fail = map_stencil_operation(face.depth_fail_op);
    let compare = map_compare_function(face.compare);
    if front {
        state.set_stencil_front(fail, pass, depth_fail, compare);
    } else {
        state.set_stencil_back(fail, pass, depth_fail, compare);
    }
    Ok(())
}

#[cfg(target_os = "horizon")]
fn map_stencil_operation(operation: wgt::StencilOperation) -> dk::DkStencilOp {
    match operation {
        wgt::StencilOperation::Keep => dk::DkStencilOp::DkStencilOp_Keep,
        wgt::StencilOperation::Zero => dk::DkStencilOp::DkStencilOp_Zero,
        wgt::StencilOperation::Replace => dk::DkStencilOp::DkStencilOp_Replace,
        wgt::StencilOperation::Invert => dk::DkStencilOp::DkStencilOp_Invert,
        wgt::StencilOperation::IncrementClamp => dk::DkStencilOp::DkStencilOp_Incr,
        wgt::StencilOperation::DecrementClamp => dk::DkStencilOp::DkStencilOp_Decr,
        wgt::StencilOperation::IncrementWrap => dk::DkStencilOp::DkStencilOp_IncrWrap,
        wgt::StencilOperation::DecrementWrap => dk::DkStencilOp::DkStencilOp_DecrWrap,
    }
}

#[cfg(target_os = "horizon")]
fn map_compare_function(compare: wgt::CompareFunction) -> dk::DkCompareOp {
    match compare {
        wgt::CompareFunction::Never => dk::DkCompareOp::DkCompareOp_Never,
        wgt::CompareFunction::Less => dk::DkCompareOp::DkCompareOp_Less,
        wgt::CompareFunction::Equal => dk::DkCompareOp::DkCompareOp_Equal,
        wgt::CompareFunction::LessEqual => dk::DkCompareOp::DkCompareOp_Lequal,
        wgt::CompareFunction::Greater => dk::DkCompareOp::DkCompareOp_Greater,
        wgt::CompareFunction::NotEqual => dk::DkCompareOp::DkCompareOp_NotEqual,
        wgt::CompareFunction::GreaterEqual => dk::DkCompareOp::DkCompareOp_Gequal,
        wgt::CompareFunction::Always => dk::DkCompareOp::DkCompareOp_Always,
    }
}

#[cfg(target_os = "horizon")]
fn map_color_targets(
    color_targets: &[Option<wgt::ColorTargetState>],
) -> Result<
    (
        dk::DkColorState,
        dk::DkColorWriteState,
        Vec<dk::DkBlendState>,
    ),
    crate::PipelineError,
> {
    if color_targets.is_empty() || color_targets.len() > 2 {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }

    let mut color_state = dk::DkColorState::defaults();
    let mut color_write_state = dk::DkColorWriteState::defaults();
    let mut blend_states = Vec::with_capacity(color_targets.len());
    for (index, color_target) in color_targets.iter().enumerate() {
        let Some(color_target) = color_target else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        if map_color_image_format(color_target.format).is_none() {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let index = u32::try_from(index)
            .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
        color_write_state.set_mask(index, map_color_write_mask(color_target.write_mask));
        let (blend_enable, blend_state) = map_blend_state(color_target.blend)?;
        color_state.set_blend_enable(index, blend_enable);
        blend_states.push(blend_state);
    }

    Ok((color_state, color_write_state, blend_states))
}

#[cfg(target_os = "horizon")]
fn map_color_write_mask(write_mask: wgt::ColorWrites) -> u32 {
    let mut mask = 0;
    if write_mask.contains(wgt::ColorWrites::RED) {
        mask |= dk::DkColorMask_R;
    }
    if write_mask.contains(wgt::ColorWrites::GREEN) {
        mask |= dk::DkColorMask_G;
    }
    if write_mask.contains(wgt::ColorWrites::BLUE) {
        mask |= dk::DkColorMask_B;
    }
    if write_mask.contains(wgt::ColorWrites::ALPHA) {
        mask |= dk::DkColorMask_A;
    }

    mask
}

#[cfg(target_os = "horizon")]
fn map_blend_state(
    blend: Option<wgt::BlendState>,
) -> Result<(bool, dk::DkBlendState), crate::PipelineError> {
    let mut blend_state = dk::DkBlendState::defaults();

    if let Some(blend) = blend {
        let color_op = map_blend_operation(blend.color.operation);
        let alpha_op = map_blend_operation(blend.alpha.operation);
        let src_color = map_blend_factor(blend.color.src_factor)?;
        let dst_color = map_blend_factor(blend.color.dst_factor)?;
        let src_alpha = map_blend_factor(blend.alpha.src_factor)?;
        let dst_alpha = map_blend_factor(blend.alpha.dst_factor)?;
        blend_state.set_ops(color_op, alpha_op);
        blend_state.set_factors(src_color, dst_color, src_alpha, dst_alpha);
        return Ok((true, blend_state));
    }

    Ok((false, blend_state))
}

#[cfg(target_os = "horizon")]
fn map_blend_operation(operation: wgt::BlendOperation) -> dk::DkBlendOp {
    match operation {
        wgt::BlendOperation::Add => dk::DkBlendOp::DkBlendOp_Add,
        wgt::BlendOperation::Subtract => dk::DkBlendOp::DkBlendOp_Sub,
        wgt::BlendOperation::ReverseSubtract => dk::DkBlendOp::DkBlendOp_RevSub,
        wgt::BlendOperation::Min => dk::DkBlendOp::DkBlendOp_Min,
        wgt::BlendOperation::Max => dk::DkBlendOp::DkBlendOp_Max,
    }
}

#[cfg(target_os = "horizon")]
fn map_blend_factor(factor: wgt::BlendFactor) -> Result<dk::DkBlendFactor, crate::PipelineError> {
    match factor {
        wgt::BlendFactor::Zero => Ok(dk::DkBlendFactor::DkBlendFactor_Zero),
        wgt::BlendFactor::One => Ok(dk::DkBlendFactor::DkBlendFactor_One),
        wgt::BlendFactor::Src => Ok(dk::DkBlendFactor::DkBlendFactor_SrcColor),
        wgt::BlendFactor::OneMinusSrc => Ok(dk::DkBlendFactor::DkBlendFactor_InvSrcColor),
        wgt::BlendFactor::SrcAlpha => Ok(dk::DkBlendFactor::DkBlendFactor_SrcAlpha),
        wgt::BlendFactor::OneMinusSrcAlpha => Ok(dk::DkBlendFactor::DkBlendFactor_InvSrcAlpha),
        wgt::BlendFactor::Dst => Ok(dk::DkBlendFactor::DkBlendFactor_DstColor),
        wgt::BlendFactor::OneMinusDst => Ok(dk::DkBlendFactor::DkBlendFactor_InvDstColor),
        wgt::BlendFactor::DstAlpha => Ok(dk::DkBlendFactor::DkBlendFactor_DstAlpha),
        wgt::BlendFactor::OneMinusDstAlpha => Ok(dk::DkBlendFactor::DkBlendFactor_InvDstAlpha),
        wgt::BlendFactor::SrcAlphaSaturated => {
            Ok(dk::DkBlendFactor::DkBlendFactor_SrcAlphaSaturate)
        }
        wgt::BlendFactor::Src1 => Ok(dk::DkBlendFactor::DkBlendFactor_Src1Color),
        wgt::BlendFactor::OneMinusSrc1 => Ok(dk::DkBlendFactor::DkBlendFactor_InvSrc1Color),
        wgt::BlendFactor::Src1Alpha => Ok(dk::DkBlendFactor::DkBlendFactor_Src1Alpha),
        wgt::BlendFactor::OneMinusSrc1Alpha => Ok(dk::DkBlendFactor::DkBlendFactor_InvSrc1Alpha),
        wgt::BlendFactor::Constant => Ok(dk::DkBlendFactor::DkBlendFactor_ConstColor),
        wgt::BlendFactor::OneMinusConstant => Ok(dk::DkBlendFactor::DkBlendFactor_InvConstColor),
    }
}

#[cfg(target_os = "horizon")]
impl TextureInner {
    unsafe fn new(raw_device: dk::DkDevice, desc: &crate::TextureDescriptor) -> DeviceResult<Self> {
        let format_support = texture_format_support(desc.format).ok_or(crate::DeviceError::Lost)?;
        let ms_mode = map_sample_count(desc.sample_count)?;
        if !matches!(
            desc.dimension,
            wgt::TextureDimension::D2 | wgt::TextureDimension::D3
        ) || desc.mip_level_count == 0
            || desc.mip_level_count > u32::from(u8::MAX)
            || desc.size.width == 0
            || desc.size.height == 0
            || desc.size.depth_or_array_layers == 0
            || (desc.dimension == wgt::TextureDimension::D3 && desc.sample_count != 1)
            || (desc.size.depth_or_array_layers != 1 && desc.sample_count != 1)
            || !format_support.supports_usage(desc.usage)
        {
            return Err(crate::DeviceError::Lost);
        }
        if desc.dimension == wgt::TextureDimension::D3
            && desc.usage.intersects(
                wgt::TextureUses::COLOR_TARGET
                    | wgt::TextureUses::DEPTH_STENCIL_READ
                    | wgt::TextureUses::DEPTH_STENCIL_WRITE,
            )
        {
            return Err(crate::DeviceError::Lost);
        }

        let mut image_layout_maker = dk::DkImageLayoutMaker::defaults(raw_device);
        if desc.dimension == wgt::TextureDimension::D3 {
            image_layout_maker.type_ = dk::DkImageType::DkImageType_3D;
        } else if desc.size.depth_or_array_layers > 1 {
            image_layout_maker.type_ = dk::DkImageType::DkImageType_2DArray;
        }
        image_layout_maker.format = format_support.image_format;
        if desc.usage.intersects(
            wgt::TextureUses::COLOR_TARGET
                | wgt::TextureUses::DEPTH_STENCIL_READ
                | wgt::TextureUses::DEPTH_STENCIL_WRITE,
        ) {
            image_layout_maker.flags |= dk::DkImageFlags_UsageRender;
        }
        if desc
            .usage
            .intersects(wgt::TextureUses::COPY_SRC | wgt::TextureUses::COPY_DST)
        {
            image_layout_maker.flags |= dk::DkImageFlags_Usage2DEngine;
        }
        if desc.usage.intersects(
            wgt::TextureUses::STORAGE_READ_ONLY
                | wgt::TextureUses::STORAGE_WRITE_ONLY
                | wgt::TextureUses::STORAGE_READ_WRITE,
        ) {
            image_layout_maker.flags |= dk::DkImageFlags_UsageLoadStore;
        }
        image_layout_maker.msMode = ms_mode;
        image_layout_maker.dimensions[0] = desc.size.width;
        image_layout_maker.dimensions[1] = desc.size.height;
        image_layout_maker.dimensions[2] = desc.size.depth_or_array_layers;
        image_layout_maker.mipLevels = desc.mip_level_count;

        let mut texture_layout = dk::DkImageLayout::zeroed();
        unsafe { dk::dkImageLayoutInitialize(&mut texture_layout, &image_layout_maker) };

        let texture_align = unsafe { dk::dkImageLayoutGetAlignment(&texture_layout) };
        let texture_size = align_up(
            unsafe { dk::dkImageLayoutGetSize(&texture_layout) } as u32,
            texture_align.max(dk::DK_MEMBLOCK_ALIGNMENT),
        );
        let mut mem_block_maker = dk::DkMemBlockMaker::defaults(raw_device, texture_size);
        mem_block_maker.flags = dk::DkMemBlockFlags_GpuCached | dk::DkMemBlockFlags_Image;
        let mem_block = unsafe { dk::dkMemBlockCreate(&mem_block_maker) };
        if mem_block.is_null() {
            return Err(crate::DeviceError::OutOfMemory);
        }

        let mut image = dk::DkImage::zeroed();
        unsafe { dk::dkImageInitialize(&mut image, &texture_layout, mem_block, 0) };

        Ok(Self {
            inner: TextureInnerRaw {
                mem_block,
                image,
                dimension: desc.dimension,
                extent: desc.size,
                format: desc.format,
                mip_level_count: desc.mip_level_count,
                sample_count: desc.sample_count,
            },
        })
    }
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy)]
struct TextureFormatSupport {
    image_format: dk::DkImageFormat,
    usage: TextureUsageRequirement,
}

#[cfg(target_os = "horizon")]
impl TextureFormatSupport {
    fn supports_usage(self, usage: wgt::TextureUses) -> bool {
        match self.usage {
            TextureUsageRequirement::Contains(supported) => supported.contains(usage),
        }
    }
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy)]
enum TextureUsageRequirement {
    Contains(wgt::TextureUses),
}

#[cfg(target_os = "horizon")]
fn texture_format_support(format: wgt::TextureFormat) -> Option<TextureFormatSupport> {
    match format {
        wgt::TextureFormat::Rgba8Unorm => Some(TextureFormatSupport {
            image_format: map_color_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::STORAGE_READ_ONLY
                    | wgt::TextureUses::STORAGE_WRITE_ONLY
                    | wgt::TextureUses::STORAGE_READ_WRITE
                    | wgt::TextureUses::COLOR_TARGET,
            ),
        }),
        wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Bgra8Unorm
        | wgt::TextureFormat::Bgra8UnormSrgb => Some(TextureFormatSupport {
            image_format: map_color_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC
                    | wgt::TextureUses::COLOR_TARGET,
            ),
        }),
        wgt::TextureFormat::R8Unorm | wgt::TextureFormat::Rg8Unorm => Some(TextureFormatSupport {
            image_format: map_texture_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC,
            ),
        }),
        wgt::TextureFormat::Depth32Float => Some(TextureFormatSupport {
            image_format: dk::DkImageFormat::DkImageFormat_ZF32,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::DEPTH_STENCIL_READ
                    | wgt::TextureUses::DEPTH_STENCIL_WRITE
                    | wgt::TextureUses::COPY_SRC
                    | wgt::TextureUses::COPY_DST,
            ),
        }),
        wgt::TextureFormat::Stencil8 => Some(TextureFormatSupport {
            image_format: dk::DkImageFormat::DkImageFormat_S8,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::DEPTH_STENCIL_READ
                    | wgt::TextureUses::DEPTH_STENCIL_WRITE
                    | wgt::TextureUses::COPY_SRC
                    | wgt::TextureUses::COPY_DST,
            ),
        }),
        _ => None,
    }
}

fn deko3d_texture_format_capabilities(
    format: wgt::TextureFormat,
) -> crate::TextureFormatCapabilities {
    match format {
        wgt::TextureFormat::Rgba8Unorm
        | wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Bgra8Unorm
        | wgt::TextureFormat::Bgra8UnormSrgb => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT_BLEND
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
                | if format == wgt::TextureFormat::Rgba8Unorm {
                    crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                        | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                        | crate::TextureFormatCapabilities::STORAGE_READ_WRITE
                } else {
                    crate::TextureFormatCapabilities::empty()
                }
        }
        wgt::TextureFormat::Depth32Float | wgt::TextureFormat::Stencil8 => {
            crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::R8Unorm | wgt::TextureFormat::Rg8Unorm => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        _ => crate::TextureFormatCapabilities::empty(),
    }
}

#[cfg(target_os = "horizon")]
fn map_texture_image_format(format: wgt::TextureFormat) -> Option<dk::DkImageFormat> {
    match format {
        wgt::TextureFormat::R8Unorm => Some(dk::DkImageFormat::DkImageFormat_R8_Unorm),
        wgt::TextureFormat::Rg8Unorm => Some(dk::DkImageFormat::DkImageFormat_RG8_Unorm),
        _ => map_color_image_format(format),
    }
}

#[cfg(target_os = "horizon")]
fn map_color_image_format(format: wgt::TextureFormat) -> Option<dk::DkImageFormat> {
    match format {
        wgt::TextureFormat::Rgba8Unorm => Some(dk::DkImageFormat::DkImageFormat_RGBA8_Unorm),
        wgt::TextureFormat::Rgba8UnormSrgb => {
            Some(dk::DkImageFormat::DkImageFormat_RGBA8_Unorm_sRGB)
        }
        wgt::TextureFormat::Bgra8Unorm => Some(dk::DkImageFormat::DkImageFormat_BGRA8_Unorm),
        wgt::TextureFormat::Bgra8UnormSrgb => {
            Some(dk::DkImageFormat::DkImageFormat_BGRA8_Unorm_sRGB)
        }
        _ => None,
    }
}

#[cfg(target_os = "horizon")]
fn map_texture_view_dimension(
    dimension: wgt::TextureViewDimension,
) -> DeviceResult<Option<dk::DkImageType>> {
    match dimension {
        wgt::TextureViewDimension::D2 => Ok(None),
        wgt::TextureViewDimension::D3 => Ok(Some(dk::DkImageType::DkImageType_3D)),
        wgt::TextureViewDimension::D2Array => Ok(Some(dk::DkImageType::DkImageType_2DArray)),
        wgt::TextureViewDimension::Cube => Ok(Some(dk::DkImageType::DkImageType_Cubemap)),
        wgt::TextureViewDimension::CubeArray => Ok(Some(dk::DkImageType::DkImageType_CubemapArray)),
        _ => Err(crate::DeviceError::Lost),
    }
}

fn texture_view_dimension_matches(
    texture: wgt::TextureDimension,
    view: wgt::TextureViewDimension,
) -> bool {
    match texture {
        wgt::TextureDimension::D2 => matches!(
            view,
            wgt::TextureViewDimension::D2
                | wgt::TextureViewDimension::D2Array
                | wgt::TextureViewDimension::Cube
                | wgt::TextureViewDimension::CubeArray
        ),
        wgt::TextureDimension::D3 => view == wgt::TextureViewDimension::D3,
        _ => false,
    }
}

#[cfg(target_os = "horizon")]
impl SamplerInner {
    fn new(desc: &crate::SamplerDescriptor) -> DeviceResult<Self> {
        if desc.compare.is_some() || desc.anisotropy_clamp != 1 || desc.border_color.is_some() {
            return Err(crate::DeviceError::Lost);
        }

        let mut sampler = dk::DkSampler::defaults();
        sampler.minFilter = map_filter_mode(desc.min_filter);
        sampler.magFilter = map_filter_mode(desc.mag_filter);
        sampler.mipFilter = match desc.mipmap_filter {
            wgt::MipmapFilterMode::Nearest => dk::DkMipFilter::DkMipFilter_None,
            wgt::MipmapFilterMode::Linear => dk::DkMipFilter::DkMipFilter_Linear,
        };
        sampler.wrapMode = [
            map_address_mode(desc.address_modes[0])?,
            map_address_mode(desc.address_modes[1])?,
            map_address_mode(desc.address_modes[2])?,
        ];
        sampler.lodClampMin = desc.lod_clamp.start;
        sampler.lodClampMax = desc.lod_clamp.end;

        Ok(Self {
            inner: SamplerInnerRaw { sampler },
        })
    }
}

#[cfg(target_os = "horizon")]
impl BindGroupInner {
    unsafe fn new(
        raw_device: dk::DkDevice,
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
    ) -> DeviceResult<Self> {
        let Resource::BindGroupLayout(kind) = desc.layout else {
            return Err(crate::DeviceError::Lost);
        };
        match kind {
            BindGroupLayoutKind::TextureSampler {
                texture_binding,
                sampler_binding,
                visibility,
            } => unsafe {
                Self::new_texture_sampler(
                    texture_descriptor_heap,
                    desc,
                    *texture_binding,
                    *sampler_binding,
                    *visibility,
                )
            },
            BindGroupLayoutKind::UniformBuffer {
                binding,
                visibility,
                has_dynamic_offset,
            } => Self::new_uniform_buffer(desc, *binding, *visibility, *has_dynamic_offset),
            BindGroupLayoutKind::StorageBuffer {
                binding,
                visibility,
                read_only,
                has_dynamic_offset,
            } => Self::new_storage_buffer(
                desc,
                *binding,
                *visibility,
                *read_only,
                *has_dynamic_offset,
            ),
            BindGroupLayoutKind::StorageTexture {
                binding,
                visibility,
                access,
                format,
            } => unsafe {
                Self::new_storage_texture(
                    texture_descriptor_heap,
                    desc,
                    *binding,
                    *visibility,
                    *access,
                    *format,
                )
            },
            BindGroupLayoutKind::BufferGroup(entries) => Self::new_buffer_group(desc, entries),
            BindGroupLayoutKind::BufferTextureSamplerGroup {
                buffers,
                texture_samplers,
            } => unsafe {
                Self::new_buffer_texture_sampler_group(
                    texture_descriptor_heap,
                    desc,
                    buffers,
                    texture_samplers,
                )
            },
            BindGroupLayoutKind::BufferStorageTextureGroup {
                buffers,
                storage_textures,
            } => unsafe {
                Self::new_buffer_storage_texture_group(
                    texture_descriptor_heap,
                    desc,
                    buffers,
                    storage_textures,
                )
            },
            BindGroupLayoutKind::BufferTextureSamplerStorageTextureGroup {
                buffers,
                texture_samplers,
                storage_textures,
            } => unsafe {
                Self::new_buffer_texture_sampler_storage_texture_group(
                    texture_descriptor_heap,
                    desc,
                    buffers,
                    texture_samplers,
                    storage_textures,
                )
            },
        }
    }

    unsafe fn new_texture_sampler(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        texture_binding: u32,
        sampler_binding: u32,
        visibility: wgt::ShaderStages,
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != 0
            || desc.samplers.len() != 1
            || desc.textures.len() != 1
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 2
        {
            return Err(crate::DeviceError::Lost);
        }

        let texture_sampler = unsafe {
            Self::make_texture_sampler_binding(
                texture_descriptor_heap,
                desc,
                texture_binding,
                sampler_binding,
                visibility,
            )?
        };

        Ok(Self {
            inner: BindGroupInnerRaw::TextureSampler(texture_sampler),
        })
    }

    unsafe fn make_texture_sampler_binding(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        texture_binding: u32,
        sampler_binding: u32,
        visibility: wgt::ShaderStages,
    ) -> DeviceResult<TextureSamplerBinding> {
        let Some(texture_entry) = desc
            .entries
            .iter()
            .find(|entry| entry.binding == texture_binding)
        else {
            return Err(crate::DeviceError::Lost);
        };
        if texture_entry.count != 1 {
            return Err(crate::DeviceError::Lost);
        }
        let Some(sampler_entry) = desc
            .entries
            .iter()
            .find(|entry| entry.binding == sampler_binding)
        else {
            return Err(crate::DeviceError::Lost);
        };
        if sampler_entry.count != 1 {
            return Err(crate::DeviceError::Lost);
        }

        let Some(sampler) = desc.samplers.get(sampler_entry.resource_index as usize) else {
            return Err(crate::DeviceError::Lost);
        };
        let Resource::Sampler(sampler) = *sampler else {
            return Err(crate::DeviceError::Lost);
        };
        let Some(texture_binding_resource) =
            desc.textures.get(texture_entry.resource_index as usize)
        else {
            return Err(crate::DeviceError::Lost);
        };
        if !texture_binding_resource
            .usage
            .contains(wgt::TextureUses::RESOURCE)
        {
            return Err(crate::DeviceError::Lost);
        }
        let Resource::TextureView {
            image,
            view_dimension,
            base_mip_level,
            mip_level_count,
            base_array_layer,
            array_layer_count,
            owner: Some(texture),
            ..
        } = texture_binding_resource.view
        else {
            return Err(crate::DeviceError::Lost);
        };

        let descriptor_slot = texture_descriptor_heap.allocate_slot()?;

        let mut image_descriptor = dk::DkImageDescriptor::zeroed();
        let mut image_view = dk::DkImageView::defaults(image.0);
        if let Some(view_type) = map_texture_view_dimension(*view_dimension)? {
            image_view.type_ = view_type;
        }
        image_view.mipLevelOffset = *base_mip_level;
        image_view.mipLevelCount = *mip_level_count;
        image_view.layerOffset = *base_array_layer;
        image_view.layerCount = *array_layer_count;
        let mut sampler_descriptor = dk::DkSamplerDescriptor::zeroed();
        unsafe {
            dk::dkImageDescriptorInitialize(&mut image_descriptor, &image_view, false, false);
            dk::dkSamplerDescriptorInitialize(&mut sampler_descriptor, &sampler.inner.sampler);
        }

        Ok(TextureSamplerBinding {
            texture_binding,
            visibility,
            sampler_binding,
            texture: texture.clone(),
            sampler: sampler.clone(),
            image_descriptor_set_gpu_addr: descriptor_slot.image_descriptor_set_gpu_addr,
            sampler_descriptor_set_gpu_addr: descriptor_slot.sampler_descriptor_set_gpu_addr,
            image_descriptor_gpu_addr: descriptor_slot.image_descriptor_gpu_addr,
            sampler_descriptor_gpu_addr: descriptor_slot.sampler_descriptor_gpu_addr,
            image_descriptor_index: descriptor_slot.image_descriptor_index,
            sampler_descriptor_index: descriptor_slot.sampler_descriptor_index,
            descriptor_count: descriptor_slot.descriptor_count,
            image_descriptor,
            sampler_descriptor,
        })
    }

    unsafe fn new_storage_texture(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
        access: wgt::StorageTextureAccess,
        format: wgt::TextureFormat,
    ) -> DeviceResult<Self> {
        if !desc.buffers.is_empty()
            || !desc.samplers.is_empty()
            || desc.textures.len() != 1
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 1
        {
            return Err(crate::DeviceError::Lost);
        }

        let entry = &desc.entries[0];
        if entry.binding != binding || entry.resource_index != 0 || entry.count != 1 {
            return Err(crate::DeviceError::Lost);
        }

        let storage_texture = unsafe {
            Self::make_storage_texture_binding(
                texture_descriptor_heap,
                binding,
                visibility,
                access,
                format,
                &desc.textures[0],
            )?
        };

        Ok(Self {
            inner: BindGroupInnerRaw::StorageTexture(storage_texture),
        })
    }

    unsafe fn make_storage_texture_binding(
        texture_descriptor_heap: &TextureDescriptorHeap,
        binding: u32,
        visibility: wgt::ShaderStages,
        access: wgt::StorageTextureAccess,
        format: wgt::TextureFormat,
        texture_binding: &crate::TextureBinding<'_, Resource>,
    ) -> DeviceResult<StorageTextureBinding> {
        let required_usage = match access {
            wgt::StorageTextureAccess::ReadOnly => wgt::TextureUses::STORAGE_READ_ONLY,
            wgt::StorageTextureAccess::WriteOnly => wgt::TextureUses::STORAGE_WRITE_ONLY,
            wgt::StorageTextureAccess::ReadWrite => wgt::TextureUses::STORAGE_READ_WRITE,
            wgt::StorageTextureAccess::Atomic => return Err(crate::DeviceError::Lost),
        };
        if format != wgt::TextureFormat::Rgba8Unorm
            || !texture_binding.usage.contains(required_usage)
        {
            return Err(crate::DeviceError::Lost);
        }
        let Resource::TextureView {
            image,
            view_dimension,
            base_mip_level,
            mip_level_count,
            base_array_layer,
            array_layer_count,
            owner: Some(texture),
            ..
        } = texture_binding.view
        else {
            return Err(crate::DeviceError::Lost);
        };

        if *mip_level_count != 1 || *array_layer_count == 0 {
            return Err(crate::DeviceError::Lost);
        }

        let descriptor_slot = texture_descriptor_heap.allocate_slot()?;

        let mut image_descriptor = dk::DkImageDescriptor::zeroed();
        let mut image_view = dk::DkImageView::defaults(image.0);
        if let Some(view_type) = map_texture_view_dimension(*view_dimension)? {
            image_view.type_ = view_type;
        }
        image_view.mipLevelOffset = *base_mip_level;
        image_view.mipLevelCount = 1;
        image_view.layerOffset = *base_array_layer;
        image_view.layerCount = *array_layer_count;
        unsafe {
            dk::dkImageDescriptorInitialize(&mut image_descriptor, &image_view, true, false);
        }

        Ok(StorageTextureBinding {
            binding,
            visibility,
            access,
            format,
            texture: texture.clone(),
            image_descriptor_set_gpu_addr: descriptor_slot.image_descriptor_set_gpu_addr,
            image_descriptor_gpu_addr: descriptor_slot.image_descriptor_gpu_addr,
            image_descriptor_index: descriptor_slot.image_descriptor_index,
            descriptor_count: descriptor_slot.descriptor_count,
            image_descriptor,
        })
    }

    fn new_uniform_buffer(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != 1
            || !desc.samplers.is_empty()
            || !desc.textures.is_empty()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 1
        {
            return Err(crate::DeviceError::Lost);
        }

        let entry = &desc.entries[0];
        if entry.binding != binding || entry.resource_index != 0 || entry.count != 1 {
            return Err(crate::DeviceError::Lost);
        }

        Ok(Self {
            inner: BindGroupInnerRaw::UniformBuffer(Self::make_uniform_buffer_binding(
                binding,
                visibility,
                has_dynamic_offset,
                &desc.buffers[0],
            )?),
        })
    }

    fn new_storage_buffer(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != 1
            || !desc.samplers.is_empty()
            || !desc.textures.is_empty()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 1
        {
            return Err(crate::DeviceError::Lost);
        }

        let entry = &desc.entries[0];
        if entry.binding != binding || entry.resource_index != 0 || entry.count != 1 {
            return Err(crate::DeviceError::Lost);
        }

        Ok(Self {
            inner: BindGroupInnerRaw::StorageBuffer(Self::make_storage_buffer_binding(
                binding,
                visibility,
                read_only,
                has_dynamic_offset,
                &desc.buffers[0],
            )?),
        })
    }

    fn new_buffer_group(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        layout_entries: &[BufferBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != layout_entries.len()
            || !desc.samplers.is_empty()
            || !desc.textures.is_empty()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != layout_entries.len()
        {
            return Err(crate::DeviceError::Lost);
        }

        let mut bindings = Vec::with_capacity(layout_entries.len());
        for layout_entry in layout_entries {
            let Some(entry) = desc
                .entries
                .iter()
                .find(|entry| entry.binding == layout_entry.binding())
            else {
                return Err(crate::DeviceError::Lost);
            };
            if entry.count != 1 {
                return Err(crate::DeviceError::Lost);
            }
            let Some(buffer) = desc.buffers.get(entry.resource_index as usize) else {
                return Err(crate::DeviceError::Lost);
            };

            match *layout_entry {
                BufferBindGroupLayoutKind::Uniform {
                    binding,
                    visibility,
                    has_dynamic_offset,
                } => {
                    bindings.push(BufferBinding::Uniform(Self::make_uniform_buffer_binding(
                        binding,
                        visibility,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
                BufferBindGroupLayoutKind::Storage {
                    binding,
                    visibility,
                    read_only,
                    has_dynamic_offset,
                } => {
                    bindings.push(BufferBinding::Storage(Self::make_storage_buffer_binding(
                        binding,
                        visibility,
                        read_only,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
            }
        }

        Ok(Self {
            inner: BindGroupInnerRaw::BufferGroup(bindings),
        })
    }

    unsafe fn new_buffer_texture_sampler_group(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        buffer_layouts: &[BufferBindGroupLayoutKind],
        texture_sampler_layouts: &[TextureSamplerBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != buffer_layouts.len()
            || desc.samplers.len() != texture_sampler_layouts.len()
            || desc.textures.len() != texture_sampler_layouts.len()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != buffer_layouts.len() + texture_sampler_layouts.len() * 2
        {
            return Err(crate::DeviceError::Lost);
        }

        let mut buffers = Vec::with_capacity(buffer_layouts.len());
        for layout_entry in buffer_layouts {
            let Some(entry) = desc
                .entries
                .iter()
                .find(|entry| entry.binding == layout_entry.binding())
            else {
                return Err(crate::DeviceError::Lost);
            };
            if entry.count != 1 {
                return Err(crate::DeviceError::Lost);
            }
            let Some(buffer) = desc.buffers.get(entry.resource_index as usize) else {
                return Err(crate::DeviceError::Lost);
            };

            match *layout_entry {
                BufferBindGroupLayoutKind::Uniform {
                    binding,
                    visibility,
                    has_dynamic_offset,
                } => {
                    buffers.push(BufferBinding::Uniform(Self::make_uniform_buffer_binding(
                        binding,
                        visibility,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
                BufferBindGroupLayoutKind::Storage {
                    binding,
                    visibility,
                    read_only,
                    has_dynamic_offset,
                } => {
                    buffers.push(BufferBinding::Storage(Self::make_storage_buffer_binding(
                        binding,
                        visibility,
                        read_only,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
            }
        }

        let mut texture_samplers = Vec::with_capacity(texture_sampler_layouts.len());
        for layout in texture_sampler_layouts {
            texture_samplers.push(unsafe {
                Self::make_texture_sampler_binding(
                    texture_descriptor_heap,
                    desc,
                    layout.texture_binding,
                    layout.sampler_binding,
                    layout.visibility,
                )?
            });
        }

        Ok(Self {
            inner: BindGroupInnerRaw::BufferTextureSamplerGroup {
                buffers,
                texture_samplers,
            },
        })
    }

    unsafe fn new_buffer_storage_texture_group(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        buffer_layouts: &[BufferBindGroupLayoutKind],
        storage_texture_layouts: &[StorageTextureBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != buffer_layouts.len()
            || !desc.samplers.is_empty()
            || desc.textures.len() != storage_texture_layouts.len()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != buffer_layouts.len() + storage_texture_layouts.len()
        {
            return Err(crate::DeviceError::Lost);
        }

        let mut buffers = Vec::with_capacity(buffer_layouts.len());
        for layout_entry in buffer_layouts {
            let Some(entry) = desc
                .entries
                .iter()
                .find(|entry| entry.binding == layout_entry.binding())
            else {
                return Err(crate::DeviceError::Lost);
            };
            if entry.count != 1 {
                return Err(crate::DeviceError::Lost);
            }
            let Some(buffer) = desc.buffers.get(entry.resource_index as usize) else {
                return Err(crate::DeviceError::Lost);
            };

            match *layout_entry {
                BufferBindGroupLayoutKind::Uniform {
                    binding,
                    visibility,
                    has_dynamic_offset,
                } => {
                    buffers.push(BufferBinding::Uniform(Self::make_uniform_buffer_binding(
                        binding,
                        visibility,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
                BufferBindGroupLayoutKind::Storage {
                    binding,
                    visibility,
                    read_only,
                    has_dynamic_offset,
                } => {
                    buffers.push(BufferBinding::Storage(Self::make_storage_buffer_binding(
                        binding,
                        visibility,
                        read_only,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
            }
        }

        let mut storage_textures = Vec::with_capacity(storage_texture_layouts.len());
        for storage_texture_layout in storage_texture_layouts {
            let Some(entry) = desc
                .entries
                .iter()
                .find(|entry| entry.binding == storage_texture_layout.binding)
            else {
                return Err(crate::DeviceError::Lost);
            };
            if entry.count != 1 {
                return Err(crate::DeviceError::Lost);
            }
            let Some(texture_binding) = desc.textures.get(entry.resource_index as usize) else {
                return Err(crate::DeviceError::Lost);
            };
            storage_textures.push(unsafe {
                Self::make_storage_texture_binding(
                    texture_descriptor_heap,
                    storage_texture_layout.binding,
                    storage_texture_layout.visibility,
                    storage_texture_layout.access,
                    storage_texture_layout.format,
                    texture_binding,
                )?
            });
        }

        Ok(Self {
            inner: BindGroupInnerRaw::BufferStorageTextureGroup {
                buffers,
                storage_textures,
            },
        })
    }

    unsafe fn new_buffer_texture_sampler_storage_texture_group(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        buffer_layouts: &[BufferBindGroupLayoutKind],
        texture_sampler_layouts: &[TextureSamplerBindGroupLayoutKind],
        storage_texture_layouts: &[StorageTextureBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != buffer_layouts.len()
            || desc.samplers.len() != texture_sampler_layouts.len()
            || desc.textures.len() != texture_sampler_layouts.len() + storage_texture_layouts.len()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len()
                != buffer_layouts.len()
                    + texture_sampler_layouts.len() * 2
                    + storage_texture_layouts.len()
        {
            return Err(crate::DeviceError::Lost);
        }

        let mut buffers = Vec::with_capacity(buffer_layouts.len());
        for layout_entry in buffer_layouts {
            let Some(entry) = desc
                .entries
                .iter()
                .find(|entry| entry.binding == layout_entry.binding())
            else {
                return Err(crate::DeviceError::Lost);
            };
            if entry.count != 1 {
                return Err(crate::DeviceError::Lost);
            }
            let Some(buffer) = desc.buffers.get(entry.resource_index as usize) else {
                return Err(crate::DeviceError::Lost);
            };

            match *layout_entry {
                BufferBindGroupLayoutKind::Uniform {
                    binding,
                    visibility,
                    has_dynamic_offset,
                } => {
                    buffers.push(BufferBinding::Uniform(Self::make_uniform_buffer_binding(
                        binding,
                        visibility,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
                BufferBindGroupLayoutKind::Storage {
                    binding,
                    visibility,
                    read_only,
                    has_dynamic_offset,
                } => {
                    buffers.push(BufferBinding::Storage(Self::make_storage_buffer_binding(
                        binding,
                        visibility,
                        read_only,
                        has_dynamic_offset,
                        buffer,
                    )?));
                }
            }
        }

        let mut texture_samplers = Vec::with_capacity(texture_sampler_layouts.len());
        for layout in texture_sampler_layouts {
            texture_samplers.push(unsafe {
                Self::make_texture_sampler_binding(
                    texture_descriptor_heap,
                    desc,
                    layout.texture_binding,
                    layout.sampler_binding,
                    layout.visibility,
                )?
            });
        }

        let mut storage_textures = Vec::with_capacity(storage_texture_layouts.len());
        for storage_texture_layout in storage_texture_layouts {
            let Some(entry) = desc
                .entries
                .iter()
                .find(|entry| entry.binding == storage_texture_layout.binding)
            else {
                return Err(crate::DeviceError::Lost);
            };
            if entry.count != 1 {
                return Err(crate::DeviceError::Lost);
            }
            let Some(texture_binding) = desc.textures.get(entry.resource_index as usize) else {
                return Err(crate::DeviceError::Lost);
            };
            storage_textures.push(unsafe {
                Self::make_storage_texture_binding(
                    texture_descriptor_heap,
                    storage_texture_layout.binding,
                    storage_texture_layout.visibility,
                    storage_texture_layout.access,
                    storage_texture_layout.format,
                    texture_binding,
                )?
            });
        }

        Ok(Self {
            inner: BindGroupInnerRaw::BufferTextureSamplerStorageTextureGroup {
                buffers,
                texture_samplers,
                storage_textures,
            },
        })
    }

    fn make_uniform_buffer_binding(
        binding: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
        buffer: &crate::BufferBinding<'_, Buffer>,
    ) -> DeviceResult<UniformBufferBinding> {
        if buffer.offset % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0 {
            return Err(crate::DeviceError::Lost);
        }
        if buffer
            .size
            .is_some_and(|size| size.get() > u64::from(dk::DK_UNIFORM_BUF_MAX_SIZE))
        {
            return Err(crate::DeviceError::Lost);
        }

        Ok(UniformBufferBinding {
            binding,
            visibility,
            has_dynamic_offset,
            buffer: buffer.buffer.clone(),
            offset: buffer.offset,
            size: buffer.size,
        })
    }

    fn make_storage_buffer_binding(
        binding: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
        buffer: &crate::BufferBinding<'_, Buffer>,
    ) -> DeviceResult<StorageBufferBinding> {
        if buffer.offset % u64::from(wgt::STORAGE_BINDING_SIZE_ALIGNMENT) != 0 {
            return Err(crate::DeviceError::Lost);
        }

        Ok(StorageBufferBinding {
            binding,
            visibility,
            read_only,
            has_dynamic_offset,
            buffer: buffer.buffer.clone(),
            offset: buffer.offset,
            size: buffer.size,
        })
    }
}

#[cfg(target_os = "horizon")]
fn map_vertex_format(
    format: wgt::VertexFormat,
) -> Result<(dk::DkVtxAttribSize, dk::DkVtxAttribType), crate::PipelineError> {
    match format {
        wgt::VertexFormat::Float32 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_1x32,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
        )),
        wgt::VertexFormat::Float32x2 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_2x32,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
        )),
        wgt::VertexFormat::Float32x3 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_3x32,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
        )),
        wgt::VertexFormat::Float32x4 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_4x32,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
        )),
        _ => Err(crate::PipelineError::Device(crate::DeviceError::Lost)),
    }
}

#[cfg(target_os = "horizon")]
fn map_filter_mode(filter: wgt::FilterMode) -> dk::DkFilter {
    match filter {
        wgt::FilterMode::Nearest => dk::DkFilter::DkFilter_Nearest,
        wgt::FilterMode::Linear => dk::DkFilter::DkFilter_Linear,
    }
}

#[cfg(target_os = "horizon")]
fn map_address_mode(address: wgt::AddressMode) -> DeviceResult<dk::DkWrapMode> {
    match address {
        wgt::AddressMode::ClampToEdge => Ok(dk::DkWrapMode::DkWrapMode_ClampToEdge),
        wgt::AddressMode::Repeat => Ok(dk::DkWrapMode::DkWrapMode_Repeat),
        wgt::AddressMode::MirrorRepeat => Ok(dk::DkWrapMode::DkWrapMode_MirroredRepeat),
        wgt::AddressMode::ClampToBorder => Err(crate::DeviceError::Lost),
    }
}

fn supported_bind_group_layout_kind(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    if entries.len() == 1 {
        if let Some(kind) = supported_buffer_bind_group_layout_kind(entries[0]) {
            return Some(kind);
        }
        return supported_storage_texture_bind_group_layout_kind(entries[0]);
    }

    if entries.len() == 2 {
        if let Some(kind) = supported_texture_bind_group_layout_kind(entries) {
            return Some(kind);
        }
    }

    if entries.len() > 1 {
        if let Some(kind) =
            supported_buffer_texture_sampler_storage_texture_bind_group_layout_kind_group(entries)
        {
            return Some(kind);
        }
        if let Some(kind) = supported_buffer_texture_sampler_bind_group_layout_kind_group(entries) {
            return Some(kind);
        }
        if let Some(kind) = supported_buffer_storage_texture_bind_group_layout_kind_group(entries) {
            return Some(kind);
        }
        return supported_buffer_bind_group_layout_kind_group(entries);
    }

    None
}

fn supported_pipeline_layout(bind_group_layouts: &[Option<&Resource>]) -> bool {
    if bind_group_layouts.len() > crate::MAX_BIND_GROUPS {
        return false;
    }

    let mut vertex_uniform_bindings = 0u32;
    let mut fragment_uniform_bindings = 0u32;
    let mut compute_uniform_bindings = 0u32;
    let mut vertex_storage_bindings = 0u32;
    let mut fragment_storage_bindings = 0u32;
    let mut compute_storage_bindings = 0u32;
    let mut vertex_image_bindings = 0u32;
    let mut fragment_image_bindings = 0u32;
    let mut compute_image_bindings = 0u32;
    for layout in bind_group_layouts.iter().flatten() {
        match layout {
            Resource::BindGroupLayout(BindGroupLayoutKind::TextureSampler {
                texture_binding,
                visibility,
                ..
            }) => {
                let Some(binding_mask) = 1u32.checked_shl(*texture_binding) else {
                    return false;
                };
                if visibility.contains(wgt::ShaderStages::VERTEX) {
                    if vertex_image_bindings & binding_mask != 0 {
                        return false;
                    }
                    vertex_image_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                    if fragment_image_bindings & binding_mask != 0 {
                        return false;
                    }
                    fragment_image_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::COMPUTE) {
                    if compute_image_bindings & binding_mask != 0 {
                        return false;
                    }
                    compute_image_bindings |= binding_mask;
                }
            }
            Resource::BindGroupLayout(BindGroupLayoutKind::UniformBuffer {
                binding,
                visibility,
                ..
            }) => {
                let Some(binding_mask) = 1u32.checked_shl(*binding) else {
                    return false;
                };
                if visibility.contains(wgt::ShaderStages::VERTEX) {
                    if vertex_uniform_bindings & binding_mask != 0 {
                        return false;
                    }
                    vertex_uniform_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                    if fragment_uniform_bindings & binding_mask != 0 {
                        return false;
                    }
                    fragment_uniform_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::COMPUTE) {
                    if compute_uniform_bindings & binding_mask != 0 {
                        return false;
                    }
                    compute_uniform_bindings |= binding_mask;
                }
            }
            Resource::BindGroupLayout(BindGroupLayoutKind::StorageBuffer {
                binding,
                visibility,
                ..
            }) => {
                let Some(binding_mask) = 1u32.checked_shl(*binding) else {
                    return false;
                };
                if visibility.contains(wgt::ShaderStages::VERTEX) {
                    if vertex_storage_bindings & binding_mask != 0 {
                        return false;
                    }
                    vertex_storage_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                    if fragment_storage_bindings & binding_mask != 0 {
                        return false;
                    }
                    fragment_storage_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::COMPUTE) {
                    if compute_storage_bindings & binding_mask != 0 {
                        return false;
                    }
                    compute_storage_bindings |= binding_mask;
                }
            }
            Resource::BindGroupLayout(BindGroupLayoutKind::StorageTexture {
                binding,
                visibility,
                ..
            }) => {
                let Some(binding_mask) = 1u32.checked_shl(*binding) else {
                    return false;
                };
                if visibility.contains(wgt::ShaderStages::VERTEX) {
                    if vertex_image_bindings & binding_mask != 0 {
                        return false;
                    }
                    vertex_image_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                    if fragment_image_bindings & binding_mask != 0 {
                        return false;
                    }
                    fragment_image_bindings |= binding_mask;
                }
                if visibility.contains(wgt::ShaderStages::COMPUTE) {
                    if compute_image_bindings & binding_mask != 0 {
                        return false;
                    }
                    compute_image_bindings |= binding_mask;
                }
            }
            Resource::BindGroupLayout(BindGroupLayoutKind::BufferGroup(entries)) => {
                for entry in entries {
                    let binding = entry.binding();
                    let visibility = entry.visibility();
                    let Some(binding_mask) = 1u32.checked_shl(binding) else {
                        return false;
                    };
                    match entry {
                        BufferBindGroupLayoutKind::Uniform { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_uniform_bindings |= binding_mask;
                            }
                        }
                        BufferBindGroupLayoutKind::Storage { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_storage_bindings |= binding_mask;
                            }
                        }
                    }
                }
            }
            Resource::BindGroupLayout(BindGroupLayoutKind::BufferTextureSamplerGroup {
                buffers,
                texture_samplers,
            }) => {
                for texture_sampler in texture_samplers {
                    let Some(binding_mask) = 1u32.checked_shl(texture_sampler.texture_binding)
                    else {
                        return false;
                    };
                    if texture_sampler
                        .visibility
                        .contains(wgt::ShaderStages::VERTEX)
                    {
                        if vertex_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        vertex_image_bindings |= binding_mask;
                    }
                    if texture_sampler
                        .visibility
                        .contains(wgt::ShaderStages::FRAGMENT)
                    {
                        if fragment_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        fragment_image_bindings |= binding_mask;
                    }
                    if texture_sampler
                        .visibility
                        .contains(wgt::ShaderStages::COMPUTE)
                    {
                        if compute_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        compute_image_bindings |= binding_mask;
                    }
                }
                for entry in buffers {
                    let binding = entry.binding();
                    let visibility = entry.visibility();
                    let Some(binding_mask) = 1u32.checked_shl(binding) else {
                        return false;
                    };
                    match entry {
                        BufferBindGroupLayoutKind::Uniform { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_uniform_bindings |= binding_mask;
                            }
                        }
                        BufferBindGroupLayoutKind::Storage { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_storage_bindings |= binding_mask;
                            }
                        }
                    }
                }
            }
            Resource::BindGroupLayout(BindGroupLayoutKind::BufferStorageTextureGroup {
                buffers,
                storage_textures,
            }) => {
                for storage_texture in storage_textures {
                    let Some(binding_mask) = 1u32.checked_shl(storage_texture.binding) else {
                        return false;
                    };
                    if storage_texture
                        .visibility
                        .contains(wgt::ShaderStages::VERTEX)
                    {
                        if vertex_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        vertex_image_bindings |= binding_mask;
                    }
                    if storage_texture
                        .visibility
                        .contains(wgt::ShaderStages::FRAGMENT)
                    {
                        if fragment_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        fragment_image_bindings |= binding_mask;
                    }
                    if storage_texture
                        .visibility
                        .contains(wgt::ShaderStages::COMPUTE)
                    {
                        if compute_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        compute_image_bindings |= binding_mask;
                    }
                }
                for entry in buffers {
                    let binding = entry.binding();
                    let visibility = entry.visibility();
                    let Some(binding_mask) = 1u32.checked_shl(binding) else {
                        return false;
                    };
                    match entry {
                        BufferBindGroupLayoutKind::Uniform { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_uniform_bindings |= binding_mask;
                            }
                        }
                        BufferBindGroupLayoutKind::Storage { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_storage_bindings |= binding_mask;
                            }
                        }
                    }
                }
            }
            Resource::BindGroupLayout(
                BindGroupLayoutKind::BufferTextureSamplerStorageTextureGroup {
                    buffers,
                    texture_samplers,
                    storage_textures,
                },
            ) => {
                for texture_sampler in texture_samplers {
                    let Some(binding_mask) = 1u32.checked_shl(texture_sampler.texture_binding)
                    else {
                        return false;
                    };
                    if texture_sampler
                        .visibility
                        .contains(wgt::ShaderStages::VERTEX)
                    {
                        if vertex_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        vertex_image_bindings |= binding_mask;
                    }
                    if texture_sampler
                        .visibility
                        .contains(wgt::ShaderStages::FRAGMENT)
                    {
                        if fragment_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        fragment_image_bindings |= binding_mask;
                    }
                    if texture_sampler
                        .visibility
                        .contains(wgt::ShaderStages::COMPUTE)
                    {
                        if compute_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        compute_image_bindings |= binding_mask;
                    }
                }

                for storage_texture in storage_textures {
                    let Some(binding_mask) = 1u32.checked_shl(storage_texture.binding) else {
                        return false;
                    };
                    if storage_texture
                        .visibility
                        .contains(wgt::ShaderStages::VERTEX)
                    {
                        if vertex_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        vertex_image_bindings |= binding_mask;
                    }
                    if storage_texture
                        .visibility
                        .contains(wgt::ShaderStages::FRAGMENT)
                    {
                        if fragment_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        fragment_image_bindings |= binding_mask;
                    }
                    if storage_texture
                        .visibility
                        .contains(wgt::ShaderStages::COMPUTE)
                    {
                        if compute_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        compute_image_bindings |= binding_mask;
                    }
                }

                for entry in buffers {
                    let binding = entry.binding();
                    let visibility = entry.visibility();
                    let Some(binding_mask) = 1u32.checked_shl(binding) else {
                        return false;
                    };
                    match entry {
                        BufferBindGroupLayoutKind::Uniform { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_uniform_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_uniform_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_uniform_bindings |= binding_mask;
                            }
                        }
                        BufferBindGroupLayoutKind::Storage { .. } => {
                            if visibility.contains(wgt::ShaderStages::VERTEX) {
                                if vertex_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                vertex_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                                if fragment_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                fragment_storage_bindings |= binding_mask;
                            }
                            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                                if compute_storage_bindings & binding_mask != 0 {
                                    return false;
                                }
                                compute_storage_bindings |= binding_mask;
                            }
                        }
                    }
                }
            }
            _ => return false,
        }
    }

    true
}

fn supported_texture_bind_group_layout_kind(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut texture_binding = None;
    let mut sampler_binding = None;
    let mut visibility = wgt::ShaderStages::empty();
    for entry in entries {
        if entry.count.is_some()
            || entry.visibility.is_empty()
            || !(wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility)
        {
            return None;
        }
        match (entry.binding, entry.ty) {
            (
                binding,
                wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Float { .. },
                    view_dimension:
                        wgt::TextureViewDimension::D2
                        | wgt::TextureViewDimension::D3
                        | wgt::TextureViewDimension::D2Array
                        | wgt::TextureViewDimension::Cube
                        | wgt::TextureViewDimension::CubeArray,
                    multisampled: false,
                },
            ) if binding < DEKO_IMAGE_BINDING_COUNT => {
                texture_binding = Some(binding);
                visibility |= entry.visibility;
            }
            (
                binding,
                wgt::BindingType::Sampler(
                    wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
                ),
            ) => {
                sampler_binding = Some(binding);
                visibility |= entry.visibility;
            }
            _ => return None,
        }
    }
    Some(BindGroupLayoutKind::TextureSampler {
        texture_binding: texture_binding?,
        sampler_binding: sampler_binding?,
        visibility,
    })
}

fn supported_storage_texture_bind_group_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<BindGroupLayoutKind> {
    let entry = supported_storage_texture_binding_layout_kind(entry)?;
    Some(BindGroupLayoutKind::StorageTexture {
        binding: entry.binding,
        visibility: entry.visibility,
        access: entry.access,
        format: entry.format,
    })
}

fn supported_storage_texture_binding_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<StorageTextureBindGroupLayoutKind> {
    if entry.count.is_some()
        || entry.visibility.is_empty()
        || entry.binding >= DEKO_IMAGE_BINDING_COUNT
    {
        return None;
    }

    match entry.ty {
        wgt::BindingType::StorageTexture {
            access,
            format: wgt::TextureFormat::Rgba8Unorm,
            view_dimension,
        } if matches!(
            (access, view_dimension),
            (
                wgt::StorageTextureAccess::ReadOnly | wgt::StorageTextureAccess::ReadWrite,
                wgt::TextureViewDimension::D2
                    | wgt::TextureViewDimension::D2Array
                    | wgt::TextureViewDimension::D3,
            ) | (
                wgt::StorageTextureAccess::WriteOnly,
                wgt::TextureViewDimension::D2
                    | wgt::TextureViewDimension::D2Array
                    | wgt::TextureViewDimension::D3,
            )
        ) && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
            .contains(entry.visibility) =>
        {
            Some(StorageTextureBindGroupLayoutKind {
                binding: entry.binding,
                visibility: entry.visibility,
                access,
                format: wgt::TextureFormat::Rgba8Unorm,
            })
        }
        _ => None,
    }
}

fn supported_buffer_texture_sampler_storage_texture_bind_group_layout_kind_group(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut buffer_entries = Vec::with_capacity(entries.len().saturating_sub(3));
    let mut textures = Vec::new();
    let mut samplers = Vec::new();
    let mut storage_textures = Vec::new();
    let mut bindings = 0u32;

    for entry in entries {
        let binding_mask = 1u32.checked_shl(entry.binding)?;
        if bindings & binding_mask != 0 {
            return None;
        }
        bindings |= binding_mask;

        if let Some(kind) = supported_buffer_binding_layout_kind(*entry) {
            buffer_entries.push(kind);
            continue;
        }

        if let Some(kind) = supported_storage_texture_binding_layout_kind(*entry) {
            storage_textures.push(kind);
            continue;
        }

        if entry.count.is_some()
            || entry.visibility.is_empty()
            || !(wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility)
        {
            return None;
        }

        match entry.ty {
            wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { .. },
                view_dimension:
                    wgt::TextureViewDimension::D2
                    | wgt::TextureViewDimension::D3
                    | wgt::TextureViewDimension::D2Array
                    | wgt::TextureViewDimension::Cube
                    | wgt::TextureViewDimension::CubeArray,
                multisampled: false,
            } if entry.binding < DEKO_IMAGE_BINDING_COUNT => {
                textures.push((entry.binding, entry.visibility));
            }
            wgt::BindingType::Sampler(
                wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
            ) => {
                samplers.push((entry.binding, entry.visibility));
            }
            _ => return None,
        }
    }

    if textures.is_empty() || textures.len() != samplers.len() || storage_textures.is_empty() {
        return None;
    }

    let mut texture_samplers = Vec::with_capacity(textures.len());
    for (texture_binding, texture_visibility) in textures {
        let sampler_binding = texture_binding.checked_add(1)?;
        let sampler_index = samplers
            .iter()
            .position(|(binding, _)| *binding == sampler_binding)?;
        let (_, sampler_visibility) = samplers.swap_remove(sampler_index);
        texture_samplers.push(TextureSamplerBindGroupLayoutKind {
            texture_binding,
            sampler_binding,
            visibility: texture_visibility | sampler_visibility,
        });
    }
    if !samplers.is_empty() {
        return None;
    }

    Some(
        BindGroupLayoutKind::BufferTextureSamplerStorageTextureGroup {
            buffers: buffer_entries,
            texture_samplers,
            storage_textures,
        },
    )
}

fn supported_buffer_texture_sampler_bind_group_layout_kind_group(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut buffer_entries = Vec::with_capacity(entries.len().saturating_sub(2));
    let mut textures = Vec::new();
    let mut samplers = Vec::new();
    let mut bindings = 0u32;

    for entry in entries {
        let binding_mask = 1u32.checked_shl(entry.binding)?;
        if bindings & binding_mask != 0 {
            return None;
        }
        bindings |= binding_mask;

        if let Some(kind) = supported_buffer_binding_layout_kind(*entry) {
            buffer_entries.push(kind);
            continue;
        }

        if entry.count.is_some()
            || entry.visibility.is_empty()
            || !(wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility)
        {
            return None;
        }

        match entry.ty {
            wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { .. },
                view_dimension:
                    wgt::TextureViewDimension::D2
                    | wgt::TextureViewDimension::D3
                    | wgt::TextureViewDimension::D2Array
                    | wgt::TextureViewDimension::Cube
                    | wgt::TextureViewDimension::CubeArray,
                multisampled: false,
            } if entry.binding < DEKO_IMAGE_BINDING_COUNT => {
                textures.push((entry.binding, entry.visibility));
            }
            wgt::BindingType::Sampler(
                wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
            ) => {
                samplers.push((entry.binding, entry.visibility));
            }
            _ => return None,
        }
    }

    if textures.is_empty() || textures.len() != samplers.len() {
        return None;
    }

    let mut texture_samplers = Vec::with_capacity(textures.len());
    for (texture_binding, texture_visibility) in textures {
        let sampler_binding = texture_binding.checked_add(1)?;
        let sampler_index = samplers
            .iter()
            .position(|(binding, _)| *binding == sampler_binding)?;
        let (_, sampler_visibility) = samplers.swap_remove(sampler_index);
        texture_samplers.push(TextureSamplerBindGroupLayoutKind {
            texture_binding,
            sampler_binding,
            visibility: texture_visibility | sampler_visibility,
        });
    }
    if !samplers.is_empty() {
        return None;
    }

    Some(BindGroupLayoutKind::BufferTextureSamplerGroup {
        buffers: buffer_entries,
        texture_samplers,
    })
}

fn supported_buffer_storage_texture_bind_group_layout_kind_group(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut buffer_entries = Vec::with_capacity(entries.len().saturating_sub(1));
    let mut storage_textures = Vec::new();
    let mut bindings = 0u32;
    for entry in entries {
        let binding_mask = 1u32.checked_shl(entry.binding)?;
        if bindings & binding_mask != 0 {
            return None;
        }
        bindings |= binding_mask;

        if let Some(kind) = supported_buffer_binding_layout_kind(*entry) {
            buffer_entries.push(kind);
            continue;
        }

        let kind = supported_storage_texture_binding_layout_kind(*entry)?;
        storage_textures.push(kind);
    }

    if buffer_entries.is_empty() || storage_textures.is_empty() {
        return None;
    }

    Some(BindGroupLayoutKind::BufferStorageTextureGroup {
        buffers: buffer_entries,
        storage_textures,
    })
}

fn supported_buffer_bind_group_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<BindGroupLayoutKind> {
    let entry = supported_buffer_binding_layout_kind(entry)?;
    match entry {
        BufferBindGroupLayoutKind::Uniform {
            binding,
            visibility,
            has_dynamic_offset,
        } => Some(BindGroupLayoutKind::UniformBuffer {
            binding,
            visibility,
            has_dynamic_offset,
        }),
        BufferBindGroupLayoutKind::Storage {
            binding,
            visibility,
            read_only,
            has_dynamic_offset,
        } => Some(BindGroupLayoutKind::StorageBuffer {
            binding,
            visibility,
            read_only,
            has_dynamic_offset,
        }),
    }
}

fn supported_buffer_bind_group_layout_kind_group(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut group_entries = Vec::with_capacity(entries.len());
    let mut bindings = 0u32;
    for entry in entries {
        let kind = supported_buffer_binding_layout_kind(*entry)?;
        let binding = kind.binding();
        let binding_mask = 1u32.checked_shl(binding)?;
        if bindings & binding_mask != 0 {
            return None;
        }
        bindings |= binding_mask;
        group_entries.push(kind);
    }

    Some(BindGroupLayoutKind::BufferGroup(group_entries))
}

fn supported_buffer_binding_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<BufferBindGroupLayoutKind> {
    if entry.count.is_some() || entry.visibility.is_empty() {
        return None;
    }

    match entry.ty {
        wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Uniform,
            has_dynamic_offset,
            min_binding_size,
        } if entry.binding < DEKO_UNIFORM_BUFFER_COUNT
            && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility) =>
        {
            if min_binding_size.is_some_and(|size| size.get() > DEKO_UNIFORM_BUF_MAX_SIZE) {
                return None;
            }
            Some(BufferBindGroupLayoutKind::Uniform {
                binding: entry.binding,
                visibility: entry.visibility,
                has_dynamic_offset,
            })
        }
        wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Storage { read_only },
            has_dynamic_offset,
            min_binding_size: _,
        } if entry.binding < DEKO_STORAGE_BUFFER_COUNT
            && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility) =>
        {
            Some(BufferBindGroupLayoutKind::Storage {
                binding: entry.binding,
                visibility: entry.visibility,
                read_only,
                has_dynamic_offset,
            })
        }
        _ => None,
    }
}

#[cfg(target_os = "horizon")]
fn shader_error(message: &'static str) -> crate::ShaderError {
    crate::ShaderError::Compilation(String::from(message))
}

#[cfg(target_os = "horizon")]
impl Drop for SurfaceStateInner {
    fn drop(&mut self) {
        unsafe {
            if !self.render_queue.is_null() {
                dk::dkQueueWaitIdle(self.render_queue);
                dk::dkQueueDestroy(self.render_queue);
            }
            if !self.swapchain.is_null() {
                dk::dkSwapchainDestroy(self.swapchain);
            }
            if !self.framebuffer_mem_block.is_null() {
                dk::dkMemBlockDestroy(self.framebuffer_mem_block);
            }
        }
    }
}

#[cfg(target_os = "horizon")]
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(target_os = "horizon")]
fn mip_dimension(value: u32, mip_level: u32) -> u32 {
    value.checked_shr(mip_level).unwrap_or(0).max(1)
}

impl Adapter {
    #[cfg(target_os = "horizon")]
    unsafe fn open_horizon(&self) -> DeviceResult<crate::OpenDevice<Api>> {
        let device_maker = dk::DkDeviceMaker::defaults();
        let raw_device = unsafe { dk::dkDeviceCreate(&device_maker) };
        if raw_device.is_null() {
            return Err(crate::DeviceError::Lost);
        }

        let mut queue_maker = dk::DkQueueMaker::defaults(raw_device);
        queue_maker.flags = dk::DkQueueFlags_Graphics;
        let raw_queue = unsafe { dk::dkQueueCreate(&queue_maker) };
        if raw_queue.is_null() {
            unsafe { dk::dkDeviceDestroy(raw_device) };
            return Err(crate::DeviceError::Lost);
        }
        let texture_descriptor_heap = match unsafe { TextureDescriptorHeap::new(raw_device) } {
            Ok(heap) => heap,
            Err(error) => {
                unsafe {
                    dk::dkQueueDestroy(raw_queue);
                    dk::dkDeviceDestroy(raw_device);
                }
                return Err(error);
            }
        };

        let inner = Arc::new(DeviceInner {
            raw: RawDevice(raw_device),
            texture_descriptor_heap,
        });

        Ok(crate::OpenDevice {
            device: Device {
                inner: inner.clone(),
            },
            queue: Queue {
                device: inner,
                raw: RawQueue(raw_queue),
                active_cmdbuf: UnsafeCell::new(None),
            },
        })
    }

    #[cfg(not(target_os = "horizon"))]
    unsafe fn open_horizon(&self) -> DeviceResult<crate::OpenDevice<Api>> {
        Err(crate::DeviceError::Lost)
    }
}

impl crate::Api for Api {
    const VARIANT: wgt::Backend = wgt::Backend::Deko3d;

    type Instance = Instance;
    type Surface = Surface;
    type Adapter = Adapter;
    type Device = Device;

    type Queue = Queue;
    type CommandEncoder = CommandBuffer;
    type CommandBuffer = CommandBuffer;

    type Buffer = Buffer;
    type Texture = Resource;
    type SurfaceTexture = Resource;
    type TextureView = Resource;
    type Sampler = Resource;
    type QuerySet = Resource;
    type Fence = Fence;
    type AccelerationStructure = Resource;
    type PipelineCache = Resource;

    type BindGroupLayout = Resource;
    type BindGroup = Resource;
    type PipelineLayout = Resource;
    type ShaderModule = Resource;
    type RenderPipeline = Resource;
    type ComputePipeline = Resource;
    type RayTracingPipeline = Resource;
}

crate::impl_dyn_resource!(
    Adapter,
    Buffer,
    CommandBuffer,
    Device,
    Fence,
    Instance,
    Queue,
    Resource,
    Surface
);

impl crate::DynAccelerationStructure for Resource {}
impl crate::DynBindGroup for Resource {}
impl crate::DynBindGroupLayout for Resource {}
impl crate::DynBuffer for Buffer {}
impl crate::DynCommandBuffer for CommandBuffer {}
impl crate::DynComputePipeline for Resource {}
impl crate::DynFence for Fence {}
impl crate::DynPipelineCache for Resource {}
impl crate::DynPipelineLayout for Resource {}
impl crate::DynQuerySet for Resource {}
impl crate::DynRayTracingPipeline for Resource {}
impl crate::DynRenderPipeline for Resource {}
impl crate::DynSampler for Resource {}
impl crate::DynShaderModule for Resource {}
impl crate::DynSurfaceTexture for Resource {}
impl crate::DynTexture for Resource {}
impl crate::DynTextureView for Resource {}

impl core::borrow::Borrow<dyn crate::DynTexture> for Resource {
    fn borrow(&self) -> &dyn crate::DynTexture {
        self
    }
}

impl crate::Instance for Instance {
    type A = Api;

    unsafe fn init(_desc: &crate::InstanceDescriptor<'_>) -> Result<Self, crate::InstanceError> {
        if cfg!(target_os = "horizon") {
            Ok(Instance)
        } else {
            Err(crate::InstanceError::new(String::from(
                "deko3d backend is only available on the Horizon/Switch target",
            )))
        }
    }
    unsafe fn create_surface(
        &self,
        _display_handle: raw_window_handle::RawDisplayHandle,
        _window_handle: raw_window_handle::RawWindowHandle,
    ) -> Result<Surface, crate::InstanceError> {
        if cfg!(target_os = "horizon") {
            Ok(Surface::new())
        } else {
            Err(crate::InstanceError::new(String::from(
                "deko3d surfaces require the Horizon/Switch target",
            )))
        }
    }
    unsafe fn enumerate_adapters(
        &self,
        _surface_hint: Option<&Surface>,
    ) -> Vec<crate::ExposedAdapter<Api>> {
        if cfg!(target_os = "horizon") {
            vec![crate::ExposedAdapter {
                adapter: Adapter,
                info: adapter_info(),
                features: supported_features(),
                capabilities: capabilities(),
            }]
        } else {
            Vec::new()
        }
    }
}

/// Returns adapter info for the first Deko3D backend slice.
///
/// Values that would require private hardware IDs are intentionally left at 0.
pub fn adapter_info() -> wgt::AdapterInfo {
    wgt::AdapterInfo {
        name: String::from("Nintendo Switch Deko3D"),
        vendor: 0,
        device: 0,
        device_type: wgt::DeviceType::IntegratedGpu,
        device_pci_bus_id: String::new(),
        driver: String::from("deko3d"),
        driver_info: String::from("homebrew"),
        backend: wgt::Backend::Deko3d,
        subgroup_min_size: wgt::MINIMUM_SUBGROUP_MIN_SIZE,
        subgroup_max_size: wgt::MAXIMUM_SUBGROUP_MAX_SIZE,
        transient_saves_memory: Some(false),
        limit_bucket: None,
    }
}

/// Minimal feature set backed by the current Deko3D proof paths.
///
/// `PASSTHROUGH_SHADERS` carries offline DKSH bytes through the public wgpu
/// unsafe shader path. `MAPPABLE_PRIMARY_BUFFERS` lets the first public triangle
/// proof initialize a vertex buffer directly, matching the direct-HAL smoke app.
/// `MULTI_DRAW_INDIRECT_COUNT` is currently CPU-side count-buffer emulation
/// over this backend's CPU-addressable buffers, not native Deko3D count-buffer
/// execution. `PIPELINE_CACHE` is an in-memory cache token: Deko3D DKSH
/// modules are already compiled, so it enables normal public cache creation and
/// cached pipeline descriptors without claiming persistent cache data.
pub fn supported_features() -> wgt::Features {
    wgt::Features::PASSTHROUGH_SHADERS
        | wgt::Features::MAPPABLE_PRIMARY_BUFFERS
        | wgt::Features::MULTI_DRAW_INDIRECT_COUNT
        | wgt::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
        | wgt::Features::PIPELINE_CACHE
}

/// Conservative capabilities for the first Deko3D adapter slice.
///
/// These are intentionally below the real hardware envelope until each resource
/// path is implemented and validated.
pub fn capabilities() -> crate::Capabilities {
    crate::Capabilities {
        limits: wgt::Limits::downlevel_defaults(),
        alignments: crate::Alignments {
            buffer_copy_offset: wgt::BufferSize::MIN,
            buffer_copy_pitch: wgt::BufferSize::new(256).unwrap(),
            uniform_bounds_check_alignment: wgt::BufferSize::new(256).unwrap(),
            raw_tlas_instance_size: 0,
            ray_tracing_scratch_buffer_alignment: 1,
            ray_tracing_pipeline_group_data_size: 0,
            ray_tracing_pipeline_group_data_alignment: 0,
            ray_tracing_pipeline_data_offset_alignment: 0,
        },
        downlevel: wgt::DownlevelCapabilities {
            flags: wgt::DownlevelFlags::INDIRECT_EXECUTION,
            limits: wgt::DownlevelLimits {},
            shader_model: wgt::ShaderModel::Sm5,
        },
        cooperative_matrix_properties: Vec::new(),
    }
}

impl crate::Surface for Surface {
    type A = Api;

    unsafe fn configure(
        &self,
        device: &Device,
        config: &crate::SurfaceConfiguration,
    ) -> Result<(), crate::SurfaceError> {
        #[cfg(target_os = "horizon")]
        {
            let state = unsafe { self.state_mut() };
            *state = Some(unsafe { SurfaceState::new(device.inner.clone(), config) }?);
            Ok(())
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::SurfaceError::Other(
                "deko3d surfaces require the Horizon/Switch target",
            ))
        }
    }

    unsafe fn unconfigure(&self, device: &Device) {
        let state = unsafe { self.state_mut() };
        *state = None;
    }

    unsafe fn acquire_texture(
        &self,
        _timeout: Option<Duration>,
        _fence: &Fence,
    ) -> Result<crate::AcquiredSurfaceTexture<Api>, crate::SurfaceError> {
        #[cfg(target_os = "horizon")]
        {
            let state = unsafe { self.state_mut() };
            let state = state.as_mut().ok_or(crate::SurfaceError::Other(
                "deko3d surface is not configured",
            ))?;
            if state.inner.acquired {
                return Err(crate::SurfaceError::Timeout);
            }

            let slot =
                unsafe { dk::dkQueueAcquireImage(state.inner.render_queue, state.inner.swapchain) };
            if slot < 0 || slot as usize >= FRAMEBUFFER_COUNT {
                return Err(crate::SurfaceError::Lost);
            }
            state.inner.acquired = true;
            let image: *const dk::DkImage = &state.inner.framebuffers[slot as usize];
            let image = RawImage(image);

            Ok(crate::AcquiredSurfaceTexture {
                texture: Resource::SurfaceTexture {
                    slot,
                    image,
                    queue: RawQueueHandle(state.inner.render_queue),
                    extent: state.inner.extent,
                },
                suboptimal: false,
            })
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::SurfaceError::Other(
                "deko3d surfaces require the Horizon/Switch target",
            ))
        }
    }
    unsafe fn discard_texture(&self, texture: Resource) {
        #[cfg(target_os = "horizon")]
        {
            if let Ok(state) = unsafe { self.configured_state() } {
                state.inner.acquired = false;
            }
        }
    }
}

impl crate::Adapter for Adapter {
    type A = Api;

    unsafe fn open(
        &self,
        requested_features: wgt::Features,
        _limits: &wgt::Limits,
        _memory_hints: &wgt::MemoryHints,
    ) -> DeviceResult<crate::OpenDevice<Api>> {
        if !(requested_features - supported_features()).is_empty() {
            return Err(crate::DeviceError::Lost);
        }
        unsafe { self.open_horizon() }
    }
    unsafe fn texture_format_capabilities(
        &self,
        format: wgt::TextureFormat,
    ) -> crate::TextureFormatCapabilities {
        deko3d_texture_format_capabilities(format)
    }

    unsafe fn surface_capabilities(&self, surface: &Surface) -> Option<crate::SurfaceCapabilities> {
        Some(crate::SurfaceCapabilities {
            formats: vec![
                wgt::SurfaceFormatCapabilities {
                    format: wgt::TextureFormat::Rgba8Unorm,
                    color_spaces: wgt::SurfaceColorSpaces::SRGB,
                },
                wgt::SurfaceFormatCapabilities {
                    format: wgt::TextureFormat::Rgba8UnormSrgb,
                    color_spaces: wgt::SurfaceColorSpaces::SRGB,
                },
                wgt::SurfaceFormatCapabilities {
                    format: wgt::TextureFormat::Bgra8Unorm,
                    color_spaces: wgt::SurfaceColorSpaces::SRGB,
                },
                wgt::SurfaceFormatCapabilities {
                    format: wgt::TextureFormat::Bgra8UnormSrgb,
                    color_spaces: wgt::SurfaceColorSpaces::SRGB,
                },
            ],
            maximum_frame_latency: 1..=2,
            current_extent: Some(wgt::Extent3d {
                width: DEFAULT_WIDTH,
                height: DEFAULT_HEIGHT,
                depth_or_array_layers: 1,
            }),
            usage: wgt::TextureUses::COLOR_TARGET,
            present_modes: vec![wgt::PresentMode::Fifo],
            composite_alpha_modes: vec![wgt::CompositeAlphaMode::Opaque],
        })
    }

    unsafe fn get_presentation_timestamp(&self) -> wgt::PresentationTimestamp {
        wgt::PresentationTimestamp::INVALID_TIMESTAMP
    }

    fn get_ordered_buffer_usages(&self) -> wgt::BufferUses {
        wgt::BufferUses::INCLUSIVE | wgt::BufferUses::MAP_WRITE
    }

    fn get_ordered_texture_usages(&self) -> wgt::TextureUses {
        wgt::TextureUses::INCLUSIVE
            | wgt::TextureUses::COLOR_TARGET
            | wgt::TextureUses::DEPTH_STENCIL_WRITE
    }
}

impl crate::Queue for Queue {
    type A = Api;

    unsafe fn submit(
        &self,
        command_buffers: &[&CommandBuffer],
        surface_textures: &[&Resource],
        (fence, fence_value): (&Fence, crate::FenceValue),
    ) -> DeviceResult<()> {
        #[cfg(target_os = "horizon")]
        {
            let surface_queue = surface_textures.iter().find_map(|texture| match texture {
                Resource::SurfaceTexture { queue, .. } => Some(*queue),
                _ => None,
            });
            let raw_queue = surface_queue.unwrap_or(RawQueueHandle(self.raw.0)).0;
            unsafe { fence.wait_on_previous_queue(raw_queue) };
            let mut recorder = unsafe { CommandRecorder::new(self.device.raw.0) }?;
            unsafe { dk::dkCmdBufClear(recorder.cmdbuf) };
            unsafe { self.begin_recording(recorder.cmdbuf) }?;
            let record_result = (|| {
                for cb in command_buffers {
                    unsafe { cb.execute(self, surface_queue) }?;
                }
                Ok(())
            })();
            unsafe { self.end_recording() };
            record_result?;
            recorder.check_memory()?;
            let mut raw_fence = dk::DkFence::new();
            unsafe { recorder.submit(raw_queue, &mut raw_fence) };
            unsafe { fence.push_submission(fence_value, raw_queue, raw_fence, Some(recorder)) }?;
            return Ok(());
        }
        #[cfg(not(target_os = "horizon"))]
        {
            let surface_queue = None;
            for cb in command_buffers {
                // SAFETY: Caller is responsible for ensuring synchronization between commands
                // and other mutations.
                unsafe {
                    cb.execute(self, surface_queue)?;
                }
            }
            fence.value.store(fence_value, Ordering::Release);
            Ok(())
        }
    }
    unsafe fn present(
        &self,
        surface: &Surface,
        texture: Resource,
    ) -> Result<(), crate::SurfaceError> {
        #[cfg(target_os = "horizon")]
        {
            let state = unsafe { surface.configured_state() }?;
            let Resource::SurfaceTexture { slot, .. } = texture else {
                return Err(crate::SurfaceError::Other(
                    "deko3d present requires a surface texture",
                ));
            };
            unsafe {
                dk::dkQueuePresentImage(state.inner.render_queue, state.inner.swapchain, slot)
            };
            state.inner.acquired = false;
            Ok(())
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::SurfaceError::Other(
                "deko3d present requires the Horizon/Switch target",
            ))
        }
    }

    unsafe fn wait_for_idle(&self) -> Result<(), crate::DeviceError> {
        Ok(())
    }

    unsafe fn get_timestamp_period(&self) -> f32 {
        1.0
    }
}

impl crate::Device for Device {
    type A = Api;

    unsafe fn create_buffer(&self, desc: &crate::BufferDescriptor) -> DeviceResult<Buffer> {
        #[cfg(target_os = "horizon")]
        {
            Buffer::new(desc, self.inner.raw_device())
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Buffer::new(desc)
        }
    }

    unsafe fn destroy_buffer(&self, buffer: Buffer) {}
    unsafe fn add_raw_buffer(&self, _buffer: &Buffer) {}

    unsafe fn map_buffer(
        &self,
        buffer: &Buffer,
        range: crate::MemoryRange,
    ) -> DeviceResult<crate::BufferMapping> {
        #[cfg(target_os = "horizon")]
        unsafe {
            buffer.download_from_gpu()?;
        }
        // Safety: the `wgpu-core` validation layer will prevent any user-accessible aliasing
        // mappings from being created, so we don’t need to perform any checks here, except for
        // bounds checks on the range which are built into `get_slice_ptr()`.
        Ok(crate::BufferMapping {
            ptr: ptr::NonNull::new(buffer.get_slice_ptr(range).cast::<u8>()).unwrap(),
            is_coherent: true,
        })
    }
    unsafe fn unmap_buffer(&self, buffer: &Buffer) {
        #[cfg(target_os = "horizon")]
        {
            let _ = unsafe { buffer.upload_to_gpu() };
        }
    }
    unsafe fn flush_mapped_ranges<I>(&self, buffer: &Buffer, ranges: I) {}
    unsafe fn invalidate_mapped_ranges<I>(&self, buffer: &Buffer, ranges: I) {}

    unsafe fn create_texture(&self, desc: &crate::TextureDescriptor) -> DeviceResult<Resource> {
        #[cfg(target_os = "horizon")]
        {
            Ok(Resource::Texture(Arc::new(unsafe {
                TextureInner::new(self.inner.raw_device(), desc)?
            })))
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::DeviceError::Lost)
        }
    }
    unsafe fn destroy_texture(&self, texture: Resource) {}
    unsafe fn add_raw_texture(&self, _texture: &Resource) {}

    unsafe fn create_texture_view(
        &self,
        texture: &Resource,
        desc: &crate::TextureViewDescriptor,
    ) -> DeviceResult<Resource> {
        match texture {
            Resource::SurfaceTexture { image, extent, .. } => Ok(Resource::TextureView {
                image: *image,
                extent: *extent,
                sample_count: 1,
                view_dimension: wgt::TextureViewDimension::D2,
                base_mip_level: 0,
                mip_level_count: 1,
                base_array_layer: 0,
                array_layer_count: 1,
                owner: None,
            }),
            Resource::Texture(texture) => {
                let mip_level_count = desc.range.mip_level_count.ok_or(crate::DeviceError::Lost)?;
                let array_layer_count = desc
                    .range
                    .array_layer_count
                    .ok_or(crate::DeviceError::Lost)?;
                if !matches!(
                    desc.dimension,
                    wgt::TextureViewDimension::D2
                        | wgt::TextureViewDimension::D3
                        | wgt::TextureViewDimension::D2Array
                        | wgt::TextureViewDimension::Cube
                        | wgt::TextureViewDimension::CubeArray
                ) || (desc.dimension == wgt::TextureViewDimension::D2 && array_layer_count != 1)
                    || (desc.dimension == wgt::TextureViewDimension::D3 && array_layer_count != 1)
                    || (desc.dimension == wgt::TextureViewDimension::Cube && array_layer_count != 6)
                    || (desc.dimension == wgt::TextureViewDimension::CubeArray
                        && array_layer_count % 6 != 0)
                    || !texture_view_dimension_matches(texture.dimension(), desc.dimension)
                    || mip_level_count == 0
                    || desc
                        .range
                        .base_mip_level
                        .checked_add(mip_level_count)
                        .ok_or(crate::DeviceError::Lost)?
                        > texture.mip_level_count()
                {
                    return Err(crate::DeviceError::Lost);
                }
                let extent = texture.mip_extent(desc.range.base_mip_level)?;
                if matches!(
                    desc.dimension,
                    wgt::TextureViewDimension::Cube | wgt::TextureViewDimension::CubeArray
                ) && extent.width != extent.height
                {
                    return Err(crate::DeviceError::Lost);
                }
                if desc
                    .range
                    .base_array_layer
                    .checked_add(array_layer_count)
                    .ok_or(crate::DeviceError::Lost)?
                    > extent.depth_or_array_layers
                {
                    return Err(crate::DeviceError::Lost);
                }
                Ok(Resource::TextureView {
                    image: texture.raw_image(),
                    extent,
                    sample_count: texture.sample_count(),
                    view_dimension: desc.dimension,
                    base_mip_level: u8::try_from(desc.range.base_mip_level)
                        .map_err(|_| crate::DeviceError::Lost)?,
                    mip_level_count: u8::try_from(mip_level_count)
                        .map_err(|_| crate::DeviceError::Lost)?,
                    base_array_layer: u16::try_from(desc.range.base_array_layer)
                        .map_err(|_| crate::DeviceError::Lost)?,
                    array_layer_count: u16::try_from(array_layer_count)
                        .map_err(|_| crate::DeviceError::Lost)?,
                    owner: Some(texture.clone()),
                })
            }
            _ => Err(crate::DeviceError::Lost),
        }
    }
    unsafe fn destroy_texture_view(&self, view: Resource) {}
    unsafe fn create_sampler(&self, desc: &crate::SamplerDescriptor) -> DeviceResult<Resource> {
        #[cfg(target_os = "horizon")]
        {
            Ok(Resource::Sampler(Arc::new(SamplerInner::new(desc)?)))
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::DeviceError::Lost)
        }
    }
    unsafe fn destroy_sampler(&self, sampler: Resource) {}

    unsafe fn create_command_encoder(
        &self,
        desc: &crate::CommandEncoderDescriptor<Queue>,
    ) -> DeviceResult<CommandBuffer> {
        Ok(CommandBuffer::new())
    }

    unsafe fn create_bind_group_layout(
        &self,
        desc: &crate::BindGroupLayoutDescriptor,
    ) -> DeviceResult<Resource> {
        if !desc.flags.is_empty() {
            return Err(crate::DeviceError::Lost);
        }
        let Some(kind) = supported_bind_group_layout_kind(desc.entries) else {
            return Err(crate::DeviceError::Lost);
        };
        Ok(Resource::BindGroupLayout(kind))
    }
    unsafe fn destroy_bind_group_layout(&self, bg_layout: Resource) {}
    unsafe fn create_pipeline_layout(
        &self,
        desc: &crate::PipelineLayoutDescriptor<Resource>,
    ) -> DeviceResult<Resource> {
        if desc.immediate_size != 0 || !supported_pipeline_layout(desc.bind_group_layouts) {
            return Err(crate::DeviceError::Lost);
        }
        Ok(Resource::PipelineLayout)
    }
    unsafe fn destroy_pipeline_layout(&self, pipeline_layout: Resource) {}
    unsafe fn create_bind_group(
        &self,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
    ) -> DeviceResult<Resource> {
        #[cfg(target_os = "horizon")]
        {
            Ok(Resource::BindGroup(Arc::new(unsafe {
                BindGroupInner::new(
                    self.inner.raw_device(),
                    self.inner.texture_descriptor_heap(),
                    desc,
                )?
            })))
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::DeviceError::Lost)
        }
    }
    unsafe fn destroy_bind_group(&self, group: Resource) {}

    unsafe fn create_shader_module(
        &self,
        desc: &crate::ShaderModuleDescriptor,
        shader: crate::ShaderInput,
    ) -> Result<Resource, crate::ShaderError> {
        #[cfg(target_os = "horizon")]
        {
            match shader {
                crate::ShaderInput::Deko3dDksh(bytes) => {
                    Ok(Resource::ShaderModule(Arc::new(unsafe {
                        ShaderModuleInner::new_dksh(self.inner.raw_device(), bytes)?
                    })))
                }
                _ => Err(crate::ShaderError::Compilation(String::from(
                    "deko3d only accepts offline DKSH bytes through the Deko3D shader path",
                ))),
            }
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::ShaderError::Device(crate::DeviceError::Lost))
        }
    }
    unsafe fn destroy_shader_module(&self, module: Resource) {}
    unsafe fn create_render_pipeline(
        &self,
        desc: &crate::RenderPipelineDescriptor<Resource, Resource, Resource>,
    ) -> Result<Resource, crate::PipelineError> {
        #[cfg(target_os = "horizon")]
        {
            Ok(Resource::RenderPipeline(Arc::new(
                RenderPipelineInner::new(desc)?,
            )))
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::PipelineError::Device(crate::DeviceError::Lost))
        }
    }
    unsafe fn destroy_render_pipeline(&self, pipeline: Resource) {}
    unsafe fn create_compute_pipeline(
        &self,
        desc: &crate::ComputePipelineDescriptor<Resource, Resource, Resource>,
    ) -> Result<Resource, crate::PipelineError> {
        #[cfg(target_os = "horizon")]
        {
            Ok(Resource::ComputePipeline(Arc::new(
                ComputePipelineInner::new(desc)?,
            )))
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::PipelineError::Device(crate::DeviceError::Lost))
        }
    }
    unsafe fn destroy_compute_pipeline(&self, pipeline: Resource) {}
    unsafe fn create_ray_tracing_pipeline(
        &self,
        desc: &crate::RayTracingPipelineDescriptor<Resource, Resource, Resource>,
    ) -> Result<Resource, crate::PipelineError> {
        Err(crate::PipelineError::Device(crate::DeviceError::Lost))
    }
    unsafe fn destroy_ray_tracing_pipeline(&self, pipeline: Resource) {}
    unsafe fn get_raytracing_pipeline_group_data(
        &self,
        pipeline: &Resource,
        groups: core::ops::Range<u32>,
    ) -> DeviceResult<Vec<u8>> {
        Err(crate::DeviceError::Lost)
    }
    unsafe fn create_pipeline_cache(
        &self,
        _desc: &crate::PipelineCacheDescriptor<'_>,
    ) -> Result<Resource, crate::PipelineCacheError> {
        Ok(Resource::PipelineCache)
    }
    unsafe fn destroy_pipeline_cache(&self, _cache: Resource) {}

    unsafe fn create_query_set(
        &self,
        desc: &wgt::QuerySetDescriptor<crate::Label>,
    ) -> DeviceResult<Resource> {
        Err(crate::DeviceError::Lost)
    }
    unsafe fn destroy_query_set(&self, set: Resource) {}
    unsafe fn create_fence(&self) -> DeviceResult<Fence> {
        Ok(Fence::new())
    }
    unsafe fn destroy_fence(&self, fence: Fence) {}
    unsafe fn get_fence_value(&self, fence: &Fence) -> DeviceResult<crate::FenceValue> {
        #[cfg(target_os = "horizon")]
        {
            return unsafe { fence.completed_value() };
        }
        #[cfg(not(target_os = "horizon"))]
        Ok(fence.value.load(Ordering::Acquire))
    }
    unsafe fn wait(
        &self,
        fence: &Fence,
        value: crate::FenceValue,
        timeout: Option<Duration>,
    ) -> DeviceResult<bool> {
        #[cfg(target_os = "horizon")]
        {
            return unsafe { fence.wait_for(value, timeout) };
        }
        #[cfg(not(target_os = "horizon"))]
        {
            let _ = timeout;
            Ok(fence.value.load(Ordering::Acquire) >= value)
        }
    }

    unsafe fn start_graphics_debugger_capture(&self) -> bool {
        false
    }
    unsafe fn stop_graphics_debugger_capture(&self) {}
    unsafe fn create_acceleration_structure(
        &self,
        desc: &crate::AccelerationStructureDescriptor,
    ) -> DeviceResult<Resource> {
        Err(crate::DeviceError::Lost)
    }
    unsafe fn get_acceleration_structure_build_sizes<'a>(
        &self,
        _desc: &crate::GetAccelerationStructureBuildSizesDescriptor<'a, Buffer>,
    ) -> crate::AccelerationStructureBuildSizes {
        Default::default()
    }
    unsafe fn get_acceleration_structure_device_address(
        &self,
        _acceleration_structure: &Resource,
    ) -> wgt::BufferAddress {
        Default::default()
    }
    unsafe fn destroy_acceleration_structure(&self, _acceleration_structure: Resource) {}

    fn tlas_instance_to_bytes(&self, instance: TlasInstance) -> Vec<u8> {
        Vec::new()
    }

    fn get_internal_counters(&self) -> wgt::HalCounters {
        Default::default()
    }

    fn check_if_oom(&self) -> DeviceResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_pipeline_cache_support() {
        assert!(supported_features().contains(wgt::Features::PIPELINE_CACHE));
    }

    #[test]
    fn r8_and_rg8_formats_are_sampled_copy_only() {
        for format in [wgt::TextureFormat::R8Unorm, wgt::TextureFormat::Rg8Unorm] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT));
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_ONLY));
        }
    }

    #[test]
    fn bgra8_formats_are_color_copy_formats_without_storage() {
        for format in [
            wgt::TextureFormat::Bgra8Unorm,
            wgt::TextureFormat::Bgra8UnormSrgb,
        ] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_ONLY));
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY));
        }
    }

    #[test]
    fn command_memory_blocks_grow_and_honor_deko3d_alignment() {
        assert_eq!(command_memory_block_size(1, 16 * 1024, 4), Some(16 * 1024));
        assert_eq!(
            command_memory_block_size(16 * 1024 + 1, 16 * 1024, 4),
            Some(16 * 1024 + 4)
        );
        assert_eq!(command_memory_block_size(1, 32 * 1024, 4), Some(32 * 1024));
        assert_eq!(command_memory_block_size(1, 16 * 1024, 0), None);
    }
}
