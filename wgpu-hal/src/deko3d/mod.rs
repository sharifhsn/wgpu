#![allow(unused_variables)]

#[cfg(target_os = "horizon")]
use alloc::boxed::Box;
#[cfg(target_os = "horizon")]
use alloc::collections::VecDeque;
use alloc::{string::String, vec, vec::Vec};
use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    ptr,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[cfg(target_os = "horizon")]
use core::ffi::c_void;
use core::fmt;
use core::mem::size_of;
#[cfg(target_os = "horizon")]
use core::mem::ManuallyDrop;

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
    state_locked: AtomicBool,
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
    PipelineLayout(Arc<PipelineLayoutInner>),
    PipelineCache,
    ShaderModule(Arc<ShaderModuleInner>),
    RenderPipeline(Arc<RenderPipelineInner>),
    ComputePipeline(Arc<ComputePipelineInner>),
    QuerySet(Arc<QuerySetInner>),
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
        format: wgt::TextureFormat,
        aspect: wgt::TextureAspect,
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
    SampledTexture {
        binding: u32,
        visibility: wgt::ShaderStages,
    },
    TextureSampler {
        texture_binding: u32,
        sampler_binding: u32,
        count: u32,
        visibility: wgt::ShaderStages,
    },
    UniformBuffer {
        binding: u32,
        count: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
    },
    StorageBuffer {
        binding: u32,
        count: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
    },
    StorageTexture {
        binding: u32,
        count: u32,
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
    BufferSampledTextureGroup {
        buffers: Vec<BufferBindGroupLayoutKind>,
        sampled_textures: Vec<SampledTextureBindGroupLayoutKind>,
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
        count: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
    },
    Storage {
        binding: u32,
        count: u32,
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
    count: u32,
    visibility: wgt::ShaderStages,
}

#[derive(Clone, Copy, Debug)]
pub struct SampledTextureBindGroupLayoutKind {
    texture_binding: u32,
    visibility: wgt::ShaderStages,
}

#[derive(Clone, Copy, Debug)]
pub struct StorageTextureBindGroupLayoutKind {
    binding: u32,
    count: u32,
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

    fn count(self) -> u32 {
        match self {
            Self::Uniform { count, .. } | Self::Storage { count, .. } => count,
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
    #[cfg(target_os = "horizon")]
    clear_buffer: ManuallyDrop<Buffer>,
}

struct SurfaceState {
    #[allow(dead_code)]
    inner: SurfaceStateInner,
}

pub struct ShaderModuleInner {
    #[cfg_attr(not(target_os = "horizon"), allow(dead_code))]
    artifacts: Vec<Arc<ShaderArtifactInner>>,
}

struct ShaderArtifactInner {
    #[allow(dead_code)]
    inner: ShaderModuleInnerRaw,
    #[cfg_attr(not(target_os = "horizon"), allow(dead_code))]
    metadata: Option<Deko3dShaderMetadata>,
}

#[derive(Clone, Debug)]
struct Deko3dShaderMetadata {
    stage: wgt::ShaderStages,
    entry_point: String,
    bindings: Vec<wgt::Deko3dShaderBinding>,
}

#[derive(Clone, Debug)]
pub(super) struct Deko3dPipelineBindingMap {
    reflected_stages: wgt::ShaderStages,
    bindings: Vec<Deko3dPipelineBinding>,
}

impl Default for Deko3dPipelineBindingMap {
    fn default() -> Self {
        Self {
            reflected_stages: wgt::ShaderStages::empty(),
            bindings: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Deko3dPipelineBinding {
    stage: wgt::ShaderStages,
    binding: wgt::Deko3dShaderBinding,
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

#[derive(Debug)]
pub struct QuerySetInner {
    query_type: wgt::QueryType,
    count: u32,
    #[cfg(target_os = "horizon")]
    report_buffer: Buffer,
}

#[derive(Debug)]
pub struct PipelineLayoutInner {
    immediate_size: u32,
    #[cfg_attr(not(target_os = "horizon"), allow(dead_code))]
    bind_group_layouts: Vec<Option<BindGroupLayoutKind>>,
    #[cfg(target_os = "horizon")]
    immediate_buffer: Option<Buffer>,
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
    layout: Arc<PipelineLayoutInner>,
    vertex_shader: Arc<ShaderArtifactInner>,
    fragment_shader: Option<Arc<ShaderArtifactInner>>,
    active_stages: wgt::ShaderStages,
    binding_map: Deko3dPipelineBindingMap,
    primitive: dk::DkPrimitive,
    strip_index_format: Option<wgt::IndexFormat>,
    vertex_buffers: Vec<dk::DkVtxBufferState>,
    vertex_attributes: Vec<dk::DkVtxAttribState>,
    rasterizer_state: dk::DkRasterizerState,
    depth_bias: wgt::DepthBiasState,
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
    layout: Arc<PipelineLayoutInner>,
    compute_shader: Arc<ShaderArtifactInner>,
    binding_map: Deko3dPipelineBindingMap,
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
    BufferSampledTextureGroup {
        buffers: Vec<BufferBinding>,
        sampled_textures: Vec<TextureSamplerBinding>,
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
    UniformArray(Vec<UniformBufferBinding>),
    Storage(StorageBufferBinding),
    StorageArray(Vec<StorageBufferBinding>),
}

#[cfg(target_os = "horizon")]
pub(super) struct TextureSamplerBinding {
    texture_binding: u32,
    visibility: wgt::ShaderStages,
    #[allow(dead_code)]
    sampler_binding: u32,
    descriptors: Vec<TextureSamplerDescriptor>,
    handles: Vec<dk::DkResHandle>,
    descriptor_count: u32,
}

#[cfg(target_os = "horizon")]
struct TextureSamplerDescriptor {
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
    image_descriptor: dk::DkImageDescriptor,
    sampler_descriptor: dk::DkSamplerDescriptor,
    _lease: Arc<TextureDescriptorLease>,
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
    allocator: Arc<TextureDescriptorAllocator>,
}

#[cfg(any(target_os = "horizon", test))]
#[derive(Debug)]
struct TextureDescriptorAllocator {
    locked: AtomicBool,
    used: UnsafeCell<[bool; DEKO_TEXTURE_DESCRIPTOR_COUNT as usize]>,
}

#[cfg(target_os = "horizon")]
struct TextureDescriptorLease {
    allocator: Arc<TextureDescriptorAllocator>,
    start: u32,
    count: u32,
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
    lease: Arc<TextureDescriptorLease>,
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
    descriptors: Vec<StorageTextureDescriptor>,
    descriptor_count: u32,
}

#[cfg(target_os = "horizon")]
struct StorageTextureDescriptor {
    #[allow(dead_code)]
    texture: Arc<TextureInner>,
    image_descriptor_set_gpu_addr: dk::DkGpuAddr,
    image_descriptor_gpu_addr: dk::DkGpuAddr,
    image_descriptor_index: u32,
    image_descriptor: dk::DkImageDescriptor,
    _lease: Arc<TextureDescriptorLease>,
}

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

const DKSH_MAGIC: u32 = u32::from_le_bytes(*b"DKSH");

fn validate_dksh_header(bytes: &[u8]) -> Result<DkshHeader, crate::ShaderError> {
    if bytes.len() < size_of::<DkshHeader>() {
        return Err(shader_error("deko3d DKSH input is shorter than its header"));
    }

    let header = unsafe { ptr::read_unaligned(bytes.as_ptr().cast::<DkshHeader>()) };
    if header.magic != DKSH_MAGIC {
        return Err(shader_error("deko3d DKSH input has an invalid magic"));
    }
    if header.header_sz != size_of::<DkshHeader>() as u32
        || header.control_sz < header.header_sz
        || header.code_sz == 0
        || header.num_programs == 0
        || header.programs_off < header.header_sz
        || header.programs_off >= header.control_sz
    {
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

    Ok(header)
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
const DEKO_IMMEDIATES_BINDING: u32 = DEKO_UNIFORM_BUFFER_COUNT - 1;
#[cfg(target_os = "horizon")]
const DEKO_PUSH_CONSTANTS_MAX_SIZE: u32 = 0x7FFC;
const DEKO_STORAGE_BUFFER_COUNT: u32 = 16;
#[cfg(target_os = "horizon")]
const DEKO_CLEAR_BUFFER_SIZE: u64 = 64 * 1024;
const DEKO_IMAGE_BINDING_COUNT: u32 = 8;
const DEKO_TEXTURE_BINDING_COUNT: u32 = 32;
const DEKO_COLOR_ATTACHMENT_COUNT: u32 = 8;
#[cfg(any(target_os = "horizon", test))]
const DEKO_TEXTURE_DESCRIPTOR_COUNT: u32 = 256;
#[cfg(target_os = "horizon")]
const DEKO_COUNTER_REPORT_SIZE: wgt::BufferAddress = 16;
#[cfg(target_os = "horizon")]
const DEKO_QUERY_RESULT_SIZE: wgt::BufferAddress = 8;

fn deko_texture_binding_mask(binding: u32, count: u32) -> Option<u32> {
    let end = binding.checked_add(count)?;
    if count == 0 || end > DEKO_TEXTURE_BINDING_COUNT {
        return None;
    }
    let range = if count == u32::BITS {
        u32::MAX
    } else {
        (1u32.checked_shl(count)? - 1) << binding
    };
    Some(range)
}

fn deko_image_binding_mask(binding: u32, count: u32) -> Option<u32> {
    let end = binding.checked_add(count)?;
    if count == 0 || end > DEKO_IMAGE_BINDING_COUNT {
        return None;
    }
    Some((1u32.checked_shl(count)? - 1) << binding)
}

fn deko_buffer_binding_mask(binding: u32, count: u32, limit: u32) -> Option<u32> {
    let end = binding.checked_add(count)?;
    if count == 0 || end > limit {
        return None;
    }
    Some((1u32.checked_shl(count)? - 1) << binding)
}
fn deko_pipeline_statistics() -> wgt::PipelineStatisticsTypes {
    wgt::PipelineStatisticsTypes::VERTEX_SHADER_INVOCATIONS
        | wgt::PipelineStatisticsTypes::CLIPPER_INVOCATIONS
        | wgt::PipelineStatisticsTypes::CLIPPER_PRIMITIVES_OUT
        | wgt::PipelineStatisticsTypes::FRAGMENT_SHADER_INVOCATIONS
}

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
unsafe impl Send for ShaderArtifactInner {}
unsafe impl Sync for ShaderArtifactInner {}
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
            state_locked: AtomicBool::new(false),
        }
    }

    fn state(&self) -> SurfaceStateGuard<'_> {
        while self
            .state_locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SurfaceStateGuard { surface: self }
    }

    #[cfg(target_os = "horizon")]
    fn configured_state(&self) -> Result<SurfaceStateGuard<'_>, crate::SurfaceError> {
        let state = self.state();
        if state.is_none() {
            return Err(crate::SurfaceError::Other(
                "deko3d surface is not configured",
            ));
        }
        Ok(state)
    }
}

struct SurfaceStateGuard<'a> {
    surface: &'a Surface,
}

impl Deref for SurfaceStateGuard<'_> {
    type Target = Option<SurfaceState>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.surface.state.get() }
    }
}

impl DerefMut for SurfaceStateGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.surface.state.get() }
    }
}

impl Drop for SurfaceStateGuard<'_> {
    fn drop(&mut self) {
        self.surface.state_locked.store(false, Ordering::Release);
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
            allocator: Arc::new(TextureDescriptorAllocator {
                locked: AtomicBool::new(false),
                used: UnsafeCell::new([false; DEKO_TEXTURE_DESCRIPTOR_COUNT as usize]),
            }),
        })
    }

    fn allocate_slots(&self, count: u32) -> DeviceResult<Vec<TextureDescriptorSlot>> {
        if count == 0 {
            return Err(crate::DeviceError::Lost);
        }
        let start = self.allocator.allocate(count)?;
        let lease = Arc::new(TextureDescriptorLease {
            allocator: self.allocator.clone(),
            start,
            count,
        });
        Ok((start..start + count)
            .map(|index| TextureDescriptorSlot {
                image_descriptor_set_gpu_addr: self.image_descriptor_set_gpu_addr,
                sampler_descriptor_set_gpu_addr: self.sampler_descriptor_set_gpu_addr,
                image_descriptor_gpu_addr: self.image_descriptor_set_gpu_addr
                    + self.image_descriptor_stride * u64::from(index),
                sampler_descriptor_gpu_addr: self.sampler_descriptor_set_gpu_addr
                    + self.sampler_descriptor_stride * u64::from(index),
                image_descriptor_index: index,
                sampler_descriptor_index: index,
                descriptor_count: self.capacity,
                lease: lease.clone(),
            })
            .collect())
    }

    unsafe fn destroy(&mut self) {
        if !self.mem_block.is_null() {
            unsafe { dk::dkMemBlockDestroy(self.mem_block) };
            self.mem_block = ptr::null_mut();
        }
    }
}

#[cfg(any(target_os = "horizon", test))]
unsafe impl Send for TextureDescriptorAllocator {}
#[cfg(any(target_os = "horizon", test))]
unsafe impl Sync for TextureDescriptorAllocator {}

#[cfg(any(target_os = "horizon", test))]
impl TextureDescriptorAllocator {
    fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    fn allocate(&self, count: u32) -> DeviceResult<u32> {
        let count = usize::try_from(count).map_err(|_| crate::DeviceError::OutOfMemory)?;
        if count == 0 || count > DEKO_TEXTURE_DESCRIPTOR_COUNT as usize {
            return Err(crate::DeviceError::OutOfMemory);
        }
        self.lock();
        let used = unsafe { &mut *self.used.get() };
        let start = used
            .windows(count)
            .position(|range| range.iter().all(|slot| !slot));
        let Some(start) = start else {
            self.unlock();
            return Err(crate::DeviceError::OutOfMemory);
        };
        used[start..start + count].fill(true);
        self.unlock();
        u32::try_from(start).map_err(|_| crate::DeviceError::OutOfMemory)
    }

    fn release(&self, start: u32, count: u32) {
        let start = start as usize;
        let count = count as usize;
        self.lock();
        let used = unsafe { &mut *self.used.get() };
        debug_assert!(used[start..start + count].iter().all(|slot| *slot));
        used[start..start + count].fill(false);
        self.unlock();
    }
}

#[cfg(target_os = "horizon")]
impl Drop for TextureDescriptorLease {
    fn drop(&mut self) {
        self.allocator.release(self.start, self.count);
    }
}

#[cfg(target_os = "horizon")]
impl Drop for TextureDescriptorHeap {
    fn drop(&mut self) {
        unsafe { self.destroy() };
    }
}

impl ShaderArtifactInner {
    #[cfg(target_os = "horizon")]
    pub(super) fn raw_shader(&self) -> *const dk::DkShader {
        &self.inner.shader
    }

    #[cfg(target_os = "horizon")]
    fn validate_stage(
        &self,
        stage: wgt::ShaderStages,
        entry_point: &str,
        layout: &PipelineLayoutInner,
    ) -> Result<(), crate::PipelineError> {
        if let Some(metadata) = &self.metadata {
            if metadata.stage != stage || metadata.entry_point != entry_point {
                return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
            }
            for binding in &metadata.bindings {
                let Some(Some(group_layout)) =
                    layout.bind_group_layouts.get(binding.group as usize)
                else {
                    return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
                };
                if !deko3d_binding_matches_layout(group_layout, binding, stage) {
                    return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
                }
            }
        } else {
            if layout.bind_group_layouts.iter().flatten().count() > 1 {
                return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
            }
            if layout.immediate_size != 0
                && layout
                    .bind_group_layouts
                    .iter()
                    .enumerate()
                    .any(|(group, group_layout)| {
                        group_layout.as_ref().is_some_and(|group_layout| {
                            deko3d_binding_matches_layout(
                                group_layout,
                                &wgt::Deko3dShaderBinding {
                                    group: group as u32,
                                    binding: DEKO_IMMEDIATES_BINDING,
                                    kind: wgt::Deko3dShaderBindingKind::UniformBuffer,
                                    count: 1,
                                    physical_binding: DEKO_IMMEDIATES_BINDING,
                                },
                                stage,
                            )
                        })
                    })
            {
                return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
            }
        }
        Ok(())
    }
}

impl Deko3dPipelineBindingMap {
    #[cfg(target_os = "horizon")]
    fn from_shaders(shaders: &[&ShaderArtifactInner]) -> Self {
        let mut map = Self::default();
        for shader in shaders {
            let Some(metadata) = &shader.metadata else {
                continue;
            };
            map.reflected_stages |= metadata.stage;
            map.bindings
                .extend(
                    metadata
                        .bindings
                        .iter()
                        .copied()
                        .map(|binding| Deko3dPipelineBinding {
                            stage: metadata.stage,
                            binding,
                        }),
                );
        }
        map
    }

    #[cfg(any(target_os = "horizon", test))]
    fn physical_binding(
        &self,
        stage: wgt::ShaderStages,
        group: u32,
        binding: u32,
        kind: wgt::Deko3dShaderBindingKind,
    ) -> Option<u32> {
        if self.reflected_stages.contains(stage) {
            return self.bindings.iter().find_map(|entry| {
                (entry.stage == stage
                    && entry.binding.group == group
                    && entry.binding.binding == binding
                    && entry.binding.kind == kind)
                    .then_some(entry.binding.physical_binding)
            });
        }
        (group == 0).then_some(binding)
    }
}

impl ShaderModuleInner {
    fn select(
        &self,
        stage: wgt::ShaderStages,
        entry_point: &str,
    ) -> Result<Arc<ShaderArtifactInner>, crate::PipelineError> {
        if self.artifacts.len() == 1 && self.artifacts[0].metadata.is_none() {
            return Ok(self.artifacts[0].clone());
        }
        self.artifacts
            .iter()
            .find(|artifact| {
                artifact.metadata.as_ref().is_some_and(|metadata| {
                    metadata.stage == stage && metadata.entry_point == entry_point
                })
            })
            .cloned()
            .ok_or(crate::PipelineError::Device(crate::DeviceError::Lost))
    }
}

#[cfg(target_os = "horizon")]
fn deko3d_binding_matches_layout(
    layout: &BindGroupLayoutKind,
    binding: &wgt::Deko3dShaderBinding,
    stage: wgt::ShaderStages,
) -> bool {
    let matches = |kind, logical, count, visibility: wgt::ShaderStages| {
        binding.kind == kind
            && binding.binding == logical
            && binding.count == count
            && visibility.contains(stage)
    };
    let buffer_matches = |buffers: &[BufferBindGroupLayoutKind]| {
        buffers.iter().any(|layout| match *layout {
            BufferBindGroupLayoutKind::Uniform {
                binding,
                count,
                visibility,
                ..
            } => matches(
                wgt::Deko3dShaderBindingKind::UniformBuffer,
                binding,
                count,
                visibility,
            ),
            BufferBindGroupLayoutKind::Storage {
                binding,
                count,
                visibility,
                ..
            } => matches(
                wgt::Deko3dShaderBindingKind::StorageBuffer,
                binding,
                count,
                visibility,
            ),
        })
    };
    let texture_sampler_matches = |layouts: &[TextureSamplerBindGroupLayoutKind]| {
        layouts.iter().any(|layout| {
            matches(
                wgt::Deko3dShaderBindingKind::SampledTexture,
                layout.texture_binding,
                layout.count,
                layout.visibility,
            ) || matches(
                wgt::Deko3dShaderBindingKind::Sampler,
                layout.sampler_binding,
                layout.count,
                layout.visibility,
            )
        })
    };
    let sampled_texture_matches = |layouts: &[SampledTextureBindGroupLayoutKind]| {
        layouts.iter().any(|layout| {
            matches(
                wgt::Deko3dShaderBindingKind::SampledTexture,
                layout.texture_binding,
                1,
                layout.visibility,
            )
        })
    };
    let storage_texture_matches = |layouts: &[StorageTextureBindGroupLayoutKind]| {
        layouts.iter().any(|layout| {
            matches(
                wgt::Deko3dShaderBindingKind::StorageTexture,
                layout.binding,
                layout.count,
                layout.visibility,
            )
        })
    };

    match layout {
        BindGroupLayoutKind::SampledTexture {
            binding: logical,
            visibility,
        } => matches(
            wgt::Deko3dShaderBindingKind::SampledTexture,
            *logical,
            1,
            *visibility,
        ),
        BindGroupLayoutKind::TextureSampler {
            texture_binding,
            sampler_binding,
            count,
            visibility,
        } => {
            matches(
                wgt::Deko3dShaderBindingKind::SampledTexture,
                *texture_binding,
                *count,
                *visibility,
            ) || matches(
                wgt::Deko3dShaderBindingKind::Sampler,
                *sampler_binding,
                *count,
                *visibility,
            )
        }
        BindGroupLayoutKind::UniformBuffer {
            binding: logical,
            count,
            visibility,
            ..
        } => matches(
            wgt::Deko3dShaderBindingKind::UniformBuffer,
            *logical,
            *count,
            *visibility,
        ),
        BindGroupLayoutKind::StorageBuffer {
            binding: logical,
            count,
            visibility,
            ..
        } => matches(
            wgt::Deko3dShaderBindingKind::StorageBuffer,
            *logical,
            *count,
            *visibility,
        ),
        BindGroupLayoutKind::StorageTexture {
            binding: logical,
            count,
            visibility,
            ..
        } => matches(
            wgt::Deko3dShaderBindingKind::StorageTexture,
            *logical,
            *count,
            *visibility,
        ),
        BindGroupLayoutKind::BufferGroup(buffers) => buffer_matches(buffers),
        BindGroupLayoutKind::BufferStorageTextureGroup {
            buffers,
            storage_textures,
        } => buffer_matches(buffers) || storage_texture_matches(storage_textures),
        BindGroupLayoutKind::BufferTextureSamplerGroup {
            buffers,
            texture_samplers,
        } => buffer_matches(buffers) || texture_sampler_matches(texture_samplers),
        BindGroupLayoutKind::BufferSampledTextureGroup {
            buffers,
            sampled_textures,
        } => buffer_matches(buffers) || sampled_texture_matches(sampled_textures),
        BindGroupLayoutKind::BufferTextureSamplerStorageTextureGroup {
            buffers,
            texture_samplers,
            storage_textures,
        } => {
            buffer_matches(buffers)
                || texture_sampler_matches(texture_samplers)
                || storage_texture_matches(storage_textures)
        }
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

impl PipelineLayoutInner {
    fn bind_group_layouts(
        desc: &crate::PipelineLayoutDescriptor<Resource>,
    ) -> DeviceResult<Vec<Option<BindGroupLayoutKind>>> {
        desc.bind_group_layouts
            .iter()
            .map(|layout| match layout {
                None => Ok(None),
                Some(Resource::BindGroupLayout(kind)) => Ok(Some(kind.clone())),
                Some(_) => Err(crate::DeviceError::Lost),
            })
            .collect()
    }

    #[cfg(target_os = "horizon")]
    fn new(
        desc: &crate::PipelineLayoutDescriptor<Resource>,
        raw_device: dk::DkDevice,
    ) -> DeviceResult<Self> {
        if u64::from(desc.immediate_size) > DEKO_UNIFORM_BUF_MAX_SIZE {
            return Err(crate::DeviceError::Lost);
        }
        let immediate_buffer = if desc.immediate_size == 0 {
            None
        } else {
            let alignment = u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT);
            let size = u64::from(desc.immediate_size)
                .checked_add(alignment - 1)
                .ok_or(crate::DeviceError::OutOfMemory)?
                & !(alignment - 1);
            Some(Buffer::new(
                &crate::BufferDescriptor {
                    label: None,
                    size,
                    usage: wgt::BufferUses::UNIFORM,
                    memory_flags: crate::MemoryFlags::empty(),
                },
                raw_device,
            )?)
        };
        Ok(Self {
            immediate_size: desc.immediate_size,
            bind_group_layouts: Self::bind_group_layouts(desc)?,
            immediate_buffer,
        })
    }

    #[cfg(not(target_os = "horizon"))]
    fn new(desc: &crate::PipelineLayoutDescriptor<Resource>) -> DeviceResult<Self> {
        if u64::from(desc.immediate_size) > DEKO_UNIFORM_BUF_MAX_SIZE {
            return Err(crate::DeviceError::Lost);
        }
        Ok(Self {
            immediate_size: desc.immediate_size,
            bind_group_layouts: Self::bind_group_layouts(desc)?,
        })
    }

    fn contains_immediate_range(&self, offset_bytes: u32, data: &[u32]) -> bool {
        let Some(size_bytes) = u32::try_from(data.len())
            .ok()
            .and_then(|word_count| word_count.checked_mul(wgt::IMMEDIATE_DATA_ALIGNMENT))
        else {
            return false;
        };
        offset_bytes
            .checked_add(size_bytes)
            .is_some_and(|end| end <= self.immediate_size)
    }

    #[cfg(target_os = "horizon")]
    unsafe fn bind_immediates(
        &self,
        cmdbuf: dk::DkCmdBuf,
        visibility: wgt::ShaderStages,
    ) -> DeviceResult<()> {
        let Some(buffer) = self.immediate_buffer.as_ref() else {
            return Ok(());
        };
        let (gpu_addr, gpu_size) = buffer.gpu_binding(0, None)?;
        unsafe {
            if visibility.contains(wgt::ShaderStages::VERTEX) {
                dk::dkCmdBufBindUniformBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Vertex,
                    DEKO_IMMEDIATES_BINDING,
                    gpu_addr,
                    gpu_size,
                );
            }
            if visibility.contains(wgt::ShaderStages::FRAGMENT) {
                dk::dkCmdBufBindUniformBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Fragment,
                    DEKO_IMMEDIATES_BINDING,
                    gpu_addr,
                    gpu_size,
                );
            }
            if visibility.contains(wgt::ShaderStages::COMPUTE) {
                dk::dkCmdBufBindUniformBuffer(
                    cmdbuf,
                    dk::DkStage::DkStage_Compute,
                    DEKO_IMMEDIATES_BINDING,
                    gpu_addr,
                    gpu_size,
                );
            }
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    unsafe fn push_immediates(
        &self,
        cmdbuf: dk::DkCmdBuf,
        offset_bytes: u32,
        data: &[u32],
    ) -> DeviceResult<()> {
        if data.is_empty() {
            return Ok(());
        }
        if !self.contains_immediate_range(offset_bytes, data) {
            return Err(crate::DeviceError::Lost);
        }
        let buffer = self
            .immediate_buffer
            .as_ref()
            .ok_or(crate::DeviceError::Lost)?;
        let (gpu_addr, gpu_size) = buffer.gpu_binding(0, None)?;
        let max_words = (DEKO_PUSH_CONSTANTS_MAX_SIZE / wgt::IMMEDIATE_DATA_ALIGNMENT) as usize;
        for (chunk_index, chunk) in data.chunks(max_words).enumerate() {
            let chunk_offset = u32::try_from(chunk_index)
                .ok()
                .and_then(|index| index.checked_mul(DEKO_PUSH_CONSTANTS_MAX_SIZE))
                .and_then(|offset| offset_bytes.checked_add(offset))
                .ok_or(crate::DeviceError::Lost)?;
            let chunk_size = u32::try_from(chunk.len())
                .ok()
                .and_then(|word_count| word_count.checked_mul(wgt::IMMEDIATE_DATA_ALIGNMENT))
                .ok_or(crate::DeviceError::Lost)?;
            unsafe {
                dk::dkCmdBufPushConstants(
                    cmdbuf,
                    gpu_addr,
                    gpu_size,
                    chunk_offset,
                    chunk_size,
                    chunk.as_ptr().cast(),
                );
            }
        }
        Ok(())
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
        group: u32,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_map: &Deko3dPipelineBindingMap,
    ) -> DeviceResult<()> {
        match &self.inner {
            BindGroupInnerRaw::TextureSampler(binding) => unsafe {
                binding.bind(cmdbuf, group, dynamic_offsets, binding_map)?;
            },
            BindGroupInnerRaw::UniformBuffer(binding) => unsafe {
                binding.bind(cmdbuf, group, dynamic_offsets, binding_map)?;
            },
            BindGroupInnerRaw::StorageBuffer(binding) => unsafe {
                binding.bind(cmdbuf, group, dynamic_offsets, binding_map)?;
            },
            BindGroupInnerRaw::StorageTexture(binding) => unsafe {
                binding.bind(cmdbuf, group, dynamic_offsets, binding_map)?;
            },
            BindGroupInnerRaw::BufferGroup(bindings) => {
                let mut dynamic_offsets = dynamic_offsets.iter();
                for binding in bindings {
                    let dynamic_offset = if binding.has_dynamic_offset() {
                        Some(*dynamic_offsets.next().ok_or(crate::DeviceError::Lost)?)
                    } else {
                        None
                    };
                    unsafe { binding.bind(cmdbuf, group, dynamic_offset, binding_map)? };
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
                    unsafe { binding.bind(cmdbuf, group, dynamic_offset, binding_map)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                for texture_sampler in texture_samplers {
                    unsafe { texture_sampler.bind(cmdbuf, group, &[], binding_map)? };
                }
            }
            BindGroupInnerRaw::BufferSampledTextureGroup {
                buffers,
                sampled_textures,
            } => {
                let mut dynamic_offsets = dynamic_offsets.iter();
                for binding in buffers {
                    let dynamic_offset = if binding.has_dynamic_offset() {
                        Some(*dynamic_offsets.next().ok_or(crate::DeviceError::Lost)?)
                    } else {
                        None
                    };
                    unsafe { binding.bind(cmdbuf, group, dynamic_offset, binding_map)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                for sampled_texture in sampled_textures {
                    unsafe { sampled_texture.bind(cmdbuf, group, &[], binding_map)? };
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
                    unsafe { binding.bind(cmdbuf, group, dynamic_offset, binding_map)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                for storage_texture in storage_textures {
                    unsafe { storage_texture.bind(cmdbuf, group, &[], binding_map)? };
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
                    unsafe { binding.bind(cmdbuf, group, dynamic_offset, binding_map)? };
                }
                if dynamic_offsets.next().is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                unsafe {
                    for texture_sampler in texture_samplers {
                        texture_sampler.bind(cmdbuf, group, &[], binding_map)?;
                    }
                    for storage_texture in storage_textures {
                        storage_texture.bind(cmdbuf, group, &[], binding_map)?;
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
        group: u32,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_map: &Deko3dPipelineBindingMap,
    ) -> DeviceResult<()> {
        if !dynamic_offsets.is_empty() {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            let descriptor = self.descriptors.first().ok_or(crate::DeviceError::Lost)?;
            for descriptor in &self.descriptors {
                dk::dkCmdBufPushData(
                    cmdbuf,
                    descriptor.image_descriptor_gpu_addr,
                    ptr::addr_of!(descriptor.image_descriptor).cast(),
                    size_of::<dk::DkImageDescriptor>() as u32,
                );
                dk::dkCmdBufPushData(
                    cmdbuf,
                    descriptor.sampler_descriptor_gpu_addr,
                    ptr::addr_of!(descriptor.sampler_descriptor).cast(),
                    size_of::<dk::DkSamplerDescriptor>() as u32,
                );
            }
            dk::dkCmdBufBindImageDescriptorSet(
                cmdbuf,
                descriptor.image_descriptor_set_gpu_addr,
                self.descriptor_count,
            );
            dk::dkCmdBufBindSamplerDescriptorSet(
                cmdbuf,
                descriptor.sampler_descriptor_set_gpu_addr,
                self.descriptor_count,
            );
            if self.visibility.contains(wgt::ShaderStages::VERTEX) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::VERTEX,
                    group,
                    self.texture_binding,
                    wgt::Deko3dShaderBindingKind::SampledTexture,
                ) {
                    dk::dkCmdBufBindTextures(
                        cmdbuf,
                        dk::DkStage::DkStage_Vertex,
                        binding,
                        self.handles.as_ptr(),
                        self.handles.len() as u32,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::FRAGMENT,
                    group,
                    self.texture_binding,
                    wgt::Deko3dShaderBindingKind::SampledTexture,
                ) {
                    dk::dkCmdBufBindTextures(
                        cmdbuf,
                        dk::DkStage::DkStage_Fragment,
                        binding,
                        self.handles.as_ptr(),
                        self.handles.len() as u32,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::COMPUTE,
                    group,
                    self.texture_binding,
                    wgt::Deko3dShaderBindingKind::SampledTexture,
                ) {
                    dk::dkCmdBufBindTextures(
                        cmdbuf,
                        dk::DkStage::DkStage_Compute,
                        binding,
                        self.handles.as_ptr(),
                        self.handles.len() as u32,
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "horizon")]
impl UniformBufferBinding {
    unsafe fn bind_array(
        cmdbuf: dk::DkCmdBuf,
        group: u32,
        bindings: &[Self],
        binding_map: &Deko3dPipelineBindingMap,
    ) -> DeviceResult<()> {
        let first = bindings.first().ok_or(crate::DeviceError::Lost)?;
        let mut extents = Vec::with_capacity(bindings.len());
        for (index, binding) in bindings.iter().enumerate() {
            if binding.binding != first.binding + index as u32
                || binding.visibility != first.visibility
                || binding.has_dynamic_offset
            {
                return Err(crate::DeviceError::Lost);
            }
            let (addr, size) = binding.buffer.gpu_binding(binding.offset, binding.size)?;
            if addr % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0
                || size > dk::DK_UNIFORM_BUF_MAX_SIZE
            {
                return Err(crate::DeviceError::Lost);
            }
            extents.push(dk::DkBufExtents { addr, size });
        }
        let count = u32::try_from(extents.len()).map_err(|_| crate::DeviceError::Lost)?;
        unsafe {
            for (stage, dk_stage) in [
                (wgt::ShaderStages::VERTEX, dk::DkStage::DkStage_Vertex),
                (wgt::ShaderStages::FRAGMENT, dk::DkStage::DkStage_Fragment),
                (wgt::ShaderStages::COMPUTE, dk::DkStage::DkStage_Compute),
            ] {
                if first.visibility.contains(stage) {
                    if let Some(binding) = binding_map.physical_binding(
                        stage,
                        group,
                        first.binding,
                        wgt::Deko3dShaderBindingKind::UniformBuffer,
                    ) {
                        dk::dkCmdBufBindUniformBuffers(
                            cmdbuf,
                            dk_stage,
                            binding,
                            extents.as_ptr(),
                            count,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        group: u32,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_map: &Deko3dPipelineBindingMap,
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
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::VERTEX,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::UniformBuffer,
                ) {
                    dk::dkCmdBufBindUniformBuffer(
                        cmdbuf,
                        dk::DkStage::DkStage_Vertex,
                        binding,
                        gpu_addr,
                        gpu_size,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::FRAGMENT,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::UniformBuffer,
                ) {
                    dk::dkCmdBufBindUniformBuffer(
                        cmdbuf,
                        dk::DkStage::DkStage_Fragment,
                        binding,
                        gpu_addr,
                        gpu_size,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::COMPUTE,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::UniformBuffer,
                ) {
                    dk::dkCmdBufBindUniformBuffer(
                        cmdbuf,
                        dk::DkStage::DkStage_Compute,
                        binding,
                        gpu_addr,
                        gpu_size,
                    );
                }
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
            Self::UniformArray(_) => false,
            Self::Storage(binding) => binding.has_dynamic_offset,
            Self::StorageArray(_) => false,
        }
    }

    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        group: u32,
        dynamic_offset: Option<wgt::DynamicOffset>,
        binding_map: &Deko3dPipelineBindingMap,
    ) -> DeviceResult<()> {
        match self {
            Self::Uniform(binding) => match dynamic_offset {
                Some(offset) => unsafe { binding.bind(cmdbuf, group, &[offset], binding_map) },
                None => unsafe { binding.bind(cmdbuf, group, &[], binding_map) },
            },
            Self::UniformArray(bindings) => {
                if dynamic_offset.is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                unsafe { UniformBufferBinding::bind_array(cmdbuf, group, bindings, binding_map) }
            }
            Self::Storage(binding) => match dynamic_offset {
                Some(offset) => unsafe { binding.bind(cmdbuf, group, &[offset], binding_map) },
                None => unsafe { binding.bind(cmdbuf, group, &[], binding_map) },
            },
            Self::StorageArray(bindings) => {
                if dynamic_offset.is_some() {
                    return Err(crate::DeviceError::Lost);
                }
                unsafe { StorageBufferBinding::bind_array(cmdbuf, group, bindings, binding_map) }
            }
        }
    }
}

#[cfg(target_os = "horizon")]
impl StorageBufferBinding {
    unsafe fn bind_array(
        cmdbuf: dk::DkCmdBuf,
        group: u32,
        bindings: &[Self],
        binding_map: &Deko3dPipelineBindingMap,
    ) -> DeviceResult<()> {
        let first = bindings.first().ok_or(crate::DeviceError::Lost)?;
        let mut extents = Vec::with_capacity(bindings.len());
        for (index, binding) in bindings.iter().enumerate() {
            if binding.binding != first.binding + index as u32
                || binding.visibility != first.visibility
                || binding.has_dynamic_offset
            {
                return Err(crate::DeviceError::Lost);
            }
            let (addr, size) = binding.buffer.gpu_binding(binding.offset, binding.size)?;
            extents.push(dk::DkBufExtents { addr, size });
        }
        let count = u32::try_from(extents.len()).map_err(|_| crate::DeviceError::Lost)?;
        unsafe {
            if first.visibility.contains(wgt::ShaderStages::VERTEX) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::VERTEX,
                    group,
                    first.binding,
                    wgt::Deko3dShaderBindingKind::StorageBuffer,
                ) {
                    dk::dkCmdBufBindStorageBuffers(
                        cmdbuf,
                        dk::DkStage::DkStage_Vertex,
                        binding,
                        extents.as_ptr(),
                        count,
                    );
                }
            }
            if first.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::FRAGMENT,
                    group,
                    first.binding,
                    wgt::Deko3dShaderBindingKind::StorageBuffer,
                ) {
                    dk::dkCmdBufBindStorageBuffers(
                        cmdbuf,
                        dk::DkStage::DkStage_Fragment,
                        binding,
                        extents.as_ptr(),
                        count,
                    );
                }
            }
            if first.visibility.contains(wgt::ShaderStages::COMPUTE) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::COMPUTE,
                    group,
                    first.binding,
                    wgt::Deko3dShaderBindingKind::StorageBuffer,
                ) {
                    dk::dkCmdBufBindStorageBuffers(
                        cmdbuf,
                        dk::DkStage::DkStage_Compute,
                        binding,
                        extents.as_ptr(),
                        count,
                    );
                }
            }
        }
        Ok(())
    }

    unsafe fn bind(
        &self,
        cmdbuf: dk::DkCmdBuf,
        group: u32,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_map: &Deko3dPipelineBindingMap,
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
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::VERTEX,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::StorageBuffer,
                ) {
                    dk::dkCmdBufBindStorageBuffer(
                        cmdbuf,
                        dk::DkStage::DkStage_Vertex,
                        binding,
                        gpu_addr,
                        gpu_size,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::FRAGMENT,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::StorageBuffer,
                ) {
                    dk::dkCmdBufBindStorageBuffer(
                        cmdbuf,
                        dk::DkStage::DkStage_Fragment,
                        binding,
                        gpu_addr,
                        gpu_size,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::COMPUTE,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::StorageBuffer,
                ) {
                    dk::dkCmdBufBindStorageBuffer(
                        cmdbuf,
                        dk::DkStage::DkStage_Compute,
                        binding,
                        gpu_addr,
                        gpu_size,
                    );
                }
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
        group: u32,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_map: &Deko3dPipelineBindingMap,
    ) -> DeviceResult<()> {
        if !dynamic_offsets.is_empty() {
            return Err(crate::DeviceError::Lost);
        }
        let descriptor = self.descriptors.first().ok_or(crate::DeviceError::Lost)?;
        let handles = self
            .descriptors
            .iter()
            .map(|descriptor| dk::dkMakeImageHandle(descriptor.image_descriptor_index))
            .collect::<Vec<_>>();
        unsafe {
            for storage_descriptor in &self.descriptors {
                dk::dkCmdBufPushData(
                    cmdbuf,
                    storage_descriptor.image_descriptor_gpu_addr,
                    ptr::addr_of!(storage_descriptor.image_descriptor).cast(),
                    size_of::<dk::DkImageDescriptor>() as u32,
                );
            }
            dk::dkCmdBufBindImageDescriptorSet(
                cmdbuf,
                descriptor.image_descriptor_set_gpu_addr,
                self.descriptor_count,
            );
            if self.visibility.contains(wgt::ShaderStages::VERTEX) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::VERTEX,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::StorageTexture,
                ) {
                    dk::dkCmdBufBindImages(
                        cmdbuf,
                        dk::DkStage::DkStage_Vertex,
                        binding,
                        handles.as_ptr(),
                        handles.len() as u32,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::FRAGMENT) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::FRAGMENT,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::StorageTexture,
                ) {
                    dk::dkCmdBufBindImages(
                        cmdbuf,
                        dk::DkStage::DkStage_Fragment,
                        binding,
                        handles.as_ptr(),
                        handles.len() as u32,
                    );
                }
            }
            if self.visibility.contains(wgt::ShaderStages::COMPUTE) {
                if let Some(binding) = binding_map.physical_binding(
                    wgt::ShaderStages::COMPUTE,
                    group,
                    self.binding,
                    wgt::Deko3dShaderBindingKind::StorageTexture,
                ) {
                    dk::dkCmdBufBindImages(
                        cmdbuf,
                        dk::DkStage::DkStage_Compute,
                        binding,
                        handles.as_ptr(),
                        handles.len() as u32,
                    );
                }
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
        unsafe { ManuallyDrop::drop(&mut self.clear_buffer) };
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
impl ShaderArtifactInner {
    unsafe fn new_dksh(
        raw_device: dk::DkDevice,
        bytes: &[u8],
        metadata: Option<&wgt::Deko3dShaderMetadata<'_>>,
    ) -> Result<Self, crate::ShaderError> {
        let header = validate_dksh_header(bytes)?;
        let metadata = metadata.map(validate_deko3d_shader_metadata).transpose()?;
        let control_len = header.control_sz as usize;
        let code_len = header.code_sz as usize;

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
                        metadata,
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
impl ShaderModuleInner {
    unsafe fn new_dksh(
        raw_device: dk::DkDevice,
        bytes: &[u8],
        metadata: Option<&wgt::Deko3dShaderMetadata<'_>>,
    ) -> Result<Self, crate::ShaderError> {
        Ok(Self {
            artifacts: vec![Arc::new(unsafe {
                ShaderArtifactInner::new_dksh(raw_device, bytes, metadata)?
            })],
        })
    }

    unsafe fn new_artifacts(
        raw_device: dk::DkDevice,
        artifacts: &[wgt::Deko3dShaderArtifact<'_>],
    ) -> Result<Self, crate::ShaderError> {
        if artifacts.is_empty() {
            return Err(shader_error("deko3d shader artifact bundle is empty"));
        }
        let mut shaders = Vec::with_capacity(artifacts.len());
        for artifact in artifacts {
            shaders.push(Arc::new(unsafe {
                ShaderArtifactInner::new_dksh(raw_device, &artifact.dksh, Some(&artifact.metadata))?
            }));
        }
        Ok(Self { artifacts: shaders })
    }
}

#[cfg(target_os = "horizon")]
impl RenderPipelineInner {
    fn new(
        desc: &crate::RenderPipelineDescriptor<Resource, Resource, Resource>,
    ) -> Result<Self, crate::PipelineError> {
        if desc.multiview_mask.is_some()
            || !matches!(desc.cache, None | Some(Resource::PipelineCache))
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let ms_mode = map_sample_count(desc.multisample.count)
            .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
        let sample_mask = u32::try_from(desc.multisample.mask)
            .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
        let primitive = map_primitive_topology(&desc.primitive)?;
        if desc.primitive.unclipped_depth
            || desc.primitive.polygon_mode != wgt::PolygonMode::Fill
            || desc.primitive.conservative
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let (color_state, color_write_state, blend_states) = map_color_targets(desc.color_targets)?;
        let depth_bias = desc
            .depth_stencil
            .as_ref()
            .map_or(wgt::DepthBiasState::default(), |state| state.bias);
        let rasterizer_state = map_rasterizer_state(&desc.primitive, depth_bias.is_enabled());
        let depth_stencil_state = map_depth_stencil_state(desc.depth_stencil.as_ref())?;
        let mut multisample_state = dk::DkMultisampleState::defaults();
        multisample_state.set_mode(ms_mode);
        multisample_state.set_rasterizer_mode(ms_mode);
        multisample_state.set_alpha_to_coverage_enable(desc.multisample.alpha_to_coverage_enabled);

        let crate::VertexProcessor::Standard {
            vertex_buffers,
            vertex_stage,
        } = &desc.vertex_processor
        else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let Resource::PipelineLayout(layout) = desc.layout else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        // Deko3D DKSH blobs are already compiled for a concrete shader program.
        // wgpu-core validates passthrough/Naga entry-point metadata before HAL
        // pipeline creation, and Deko3D does not inspect the source-level name.

        let Resource::ShaderModule(vertex_module) = vertex_stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let vertex_shader =
            vertex_module.select(wgt::ShaderStages::VERTEX, vertex_stage.entry_point)?;
        vertex_shader.validate_stage(
            wgt::ShaderStages::VERTEX,
            vertex_stage.entry_point,
            layout,
        )?;
        let fragment_shader = desc
            .fragment_stage
            .as_ref()
            .map(|fragment_stage| {
                let Resource::ShaderModule(fragment_module) = fragment_stage.module else {
                    return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
                };
                let shader = fragment_module
                    .select(wgt::ShaderStages::FRAGMENT, fragment_stage.entry_point)?;
                shader.validate_stage(
                    wgt::ShaderStages::FRAGMENT,
                    fragment_stage.entry_point,
                    layout,
                )?;
                Ok(shader)
            })
            .transpose()?;
        let active_stages = wgt::ShaderStages::VERTEX
            | fragment_shader
                .as_ref()
                .map_or(wgt::ShaderStages::empty(), |_| wgt::ShaderStages::FRAGMENT);
        let mut reflected_shaders = vec![vertex_shader.as_ref()];
        if let Some(fragment_shader) = &fragment_shader {
            reflected_shaders.push(fragment_shader.as_ref());
        }
        let binding_map = Deko3dPipelineBindingMap::from_shaders(&reflected_shaders);

        let mut vertex_buffer_states = Vec::new();
        let mut vertex_attributes = Vec::new();
        for (buffer_id, layout) in vertex_buffers.iter().enumerate() {
            let Some(layout) = layout else {
                continue;
            };
            let divisor = vertex_step_divisor(layout.step_mode);
            vertex_buffer_states.push(dk::DkVtxBufferState {
                stride: u32::try_from(layout.array_stride)
                    .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                divisor,
            });
            for attribute in layout.attributes {
                let (size, type_, is_bgra) = map_vertex_format(attribute.format)?;
                vertex_attributes.push(dk::DkVtxAttribState::new(
                    u32::try_from(buffer_id)
                        .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                    is_bgra,
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
                layout: layout.clone(),
                vertex_shader: vertex_shader.clone(),
                fragment_shader,
                active_stages,
                binding_map,
                primitive,
                strip_index_format: desc.primitive.strip_index_format,
                vertex_buffers: vertex_buffer_states,
                vertex_attributes,
                rasterizer_state,
                depth_bias,
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
        if !matches!(desc.cache, None | Some(Resource::PipelineCache)) {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }

        let Resource::PipelineLayout(layout) = desc.layout else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };

        let Resource::ShaderModule(compute_module) = desc.stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let compute_shader =
            compute_module.select(wgt::ShaderStages::COMPUTE, desc.stage.entry_point)?;
        compute_shader.validate_stage(
            wgt::ShaderStages::COMPUTE,
            desc.stage.entry_point,
            layout,
        )?;

        Ok(Self {
            inner: ComputePipelineInnerRaw {
                layout: layout.clone(),
                compute_shader: compute_shader.clone(),
                binding_map: Deko3dPipelineBindingMap::from_shaders(&[&compute_shader]),
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

fn vertex_step_divisor(step_mode: wgt::VertexStepMode) -> u32 {
    match step_mode {
        wgt::VertexStepMode::Vertex => 0,
        wgt::VertexStepMode::Instance => 1,
    }
}

#[cfg(target_os = "horizon")]
fn map_sample_count(sample_count: u32) -> DeviceResult<dk::DkMsMode> {
    match sample_count {
        1 => Ok(dk::DkMsMode::DkMsMode_1x),
        2 => Ok(dk::DkMsMode::DkMsMode_2x),
        4 => Ok(dk::DkMsMode::DkMsMode_4x),
        8 => Ok(dk::DkMsMode::DkMsMode_8x),
        _ => Err(crate::DeviceError::Lost),
    }
}

#[cfg(target_os = "horizon")]
fn map_rasterizer_state(
    primitive: &wgt::PrimitiveState,
    depth_bias_enabled: bool,
) -> dk::DkRasterizerState {
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
    state.set_depth_bias_enable(depth_bias_enabled);
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

    if !matches!(
        depth_stencil.format,
        wgt::TextureFormat::Depth16Unorm
            | wgt::TextureFormat::Depth32Float
            | wgt::TextureFormat::Depth24PlusStencil8
            | wgt::TextureFormat::Depth32FloatStencil8
            | wgt::TextureFormat::Stencil8
    ) {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }

    let has_depth = matches!(
        depth_stencil.format,
        wgt::TextureFormat::Depth16Unorm
            | wgt::TextureFormat::Depth32Float
            | wgt::TextureFormat::Depth24PlusStencil8
            | wgt::TextureFormat::Depth32FloatStencil8
    );
    let has_stencil = matches!(
        depth_stencil.format,
        wgt::TextureFormat::Depth24PlusStencil8
            | wgt::TextureFormat::Depth32FloatStencil8
            | wgt::TextureFormat::Stencil8
    );
    if !has_depth
        && (depth_stencil.depth_compare.is_some()
            || depth_stencil.depth_write_enabled.unwrap_or(false))
    {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }
    if !has_stencil && depth_stencil.stencil.is_enabled() {
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
    if color_targets.len() > DEKO_COLOR_ATTACHMENT_COUNT as usize {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }

    let mut color_state = dk::DkColorState::defaults();
    let mut color_write_state = dk::DkColorWriteState::defaults();
    let mut blend_states = Vec::with_capacity(color_targets.len());
    for (index, color_target) in color_targets.iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
        let Some(color_target) = color_target else {
            color_write_state.set_mask(index, 0);
            color_state.set_blend_enable(index, false);
            blend_states.push(dk::DkBlendState::defaults());
            continue;
        };
        if map_color_image_format(color_target.format).is_none() {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
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
            wgt::TextureDimension::D1 | wgt::TextureDimension::D2 | wgt::TextureDimension::D3
        ) || desc.mip_level_count == 0
            || desc.mip_level_count > u32::from(u8::MAX)
            || desc.size.width == 0
            || desc.size.height == 0
            || desc.size.depth_or_array_layers == 0
            || (desc.dimension == wgt::TextureDimension::D1
                && (desc.size.height != 1
                    || desc.size.depth_or_array_layers != 1
                    || desc.sample_count != 1))
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
        match desc.dimension {
            wgt::TextureDimension::D1 => {
                image_layout_maker.type_ = dk::DkImageType::DkImageType_1D;
            }
            wgt::TextureDimension::D2 if desc.size.depth_or_array_layers > 1 => {
                image_layout_maker.type_ = dk::DkImageType::DkImageType_2DArray;
            }
            wgt::TextureDimension::D3 => {
                image_layout_maker.type_ = dk::DkImageType::DkImageType_3D;
            }
            _ => {}
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
                    | wgt::TextureUses::COPY_SRC
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
        wgt::TextureFormat::Rgba8Uint | wgt::TextureFormat::Rgba8Sint => {
            Some(TextureFormatSupport {
                image_format: map_color_image_format(format)?,
                usage: TextureUsageRequirement::Contains(
                    wgt::TextureUses::RESOURCE
                        | wgt::TextureUses::COPY_DST
                        | wgt::TextureUses::COPY_SRC
                        | wgt::TextureUses::COLOR_TARGET,
                ),
            })
        }
        wgt::TextureFormat::R8Unorm | wgt::TextureFormat::Rg8Unorm => Some(TextureFormatSupport {
            image_format: map_texture_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC
                    | wgt::TextureUses::STORAGE_READ_ONLY
                    | wgt::TextureUses::STORAGE_WRITE_ONLY
                    | wgt::TextureUses::STORAGE_READ_WRITE,
            ),
        }),
        wgt::TextureFormat::R8Uint
        | wgt::TextureFormat::R8Sint
        | wgt::TextureFormat::Rg8Uint
        | wgt::TextureFormat::Rg8Sint
        | wgt::TextureFormat::R8Snorm
        | wgt::TextureFormat::Rg8Snorm
        | wgt::TextureFormat::Rgba8Snorm => Some(TextureFormatSupport {
            image_format: map_texture_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC,
            ),
        }),
        wgt::TextureFormat::R16Float | wgt::TextureFormat::Rg16Float => {
            Some(TextureFormatSupport {
                image_format: map_texture_image_format(format)?,
                usage: TextureUsageRequirement::Contains(
                    wgt::TextureUses::RESOURCE
                        | wgt::TextureUses::COPY_DST
                        | wgt::TextureUses::COPY_SRC,
                ),
            })
        }
        wgt::TextureFormat::R16Unorm
        | wgt::TextureFormat::R16Snorm
        | wgt::TextureFormat::Rg16Unorm
        | wgt::TextureFormat::Rg16Snorm => Some(TextureFormatSupport {
            image_format: map_texture_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC,
            ),
        }),
        wgt::TextureFormat::Rgba16Unorm | wgt::TextureFormat::Rgba16Snorm => {
            Some(TextureFormatSupport {
                image_format: map_color_image_format(format)?,
                usage: TextureUsageRequirement::Contains(
                    wgt::TextureUses::RESOURCE
                        | wgt::TextureUses::COPY_DST
                        | wgt::TextureUses::COPY_SRC,
                ),
            })
        }
        wgt::TextureFormat::R16Uint
        | wgt::TextureFormat::R16Sint
        | wgt::TextureFormat::Rg16Uint
        | wgt::TextureFormat::Rg16Sint
        | wgt::TextureFormat::Rgba16Uint
        | wgt::TextureFormat::Rgba16Sint => Some(TextureFormatSupport {
            image_format: map_texture_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC,
            ),
        }),
        wgt::TextureFormat::R32Float
        | wgt::TextureFormat::R32Uint
        | wgt::TextureFormat::R32Sint
        | wgt::TextureFormat::Rg32Float
        | wgt::TextureFormat::Rg32Uint
        | wgt::TextureFormat::Rg32Sint
        | wgt::TextureFormat::Rgba32Float
        | wgt::TextureFormat::Rgba32Uint
        | wgt::TextureFormat::Rgba32Sint => Some(TextureFormatSupport {
            image_format: map_texture_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC,
            ),
        }),
        wgt::TextureFormat::Rgba16Float => Some(TextureFormatSupport {
            image_format: map_color_image_format(format)?,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::COPY_DST
                    | wgt::TextureUses::COPY_SRC
                    | wgt::TextureUses::STORAGE_READ_ONLY
                    | wgt::TextureUses::STORAGE_WRITE_ONLY
                    | wgt::TextureUses::STORAGE_READ_WRITE
                    | wgt::TextureUses::COLOR_TARGET,
            ),
        }),
        wgt::TextureFormat::Depth16Unorm => Some(TextureFormatSupport {
            image_format: dk::DkImageFormat::DkImageFormat_Z16,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::DEPTH_STENCIL_READ
                    | wgt::TextureUses::DEPTH_STENCIL_WRITE
                    | wgt::TextureUses::COPY_SRC
                    | wgt::TextureUses::COPY_DST,
            ),
        }),
        wgt::TextureFormat::Depth32Float => Some(TextureFormatSupport {
            image_format: dk::DkImageFormat::DkImageFormat_ZF32,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::DEPTH_STENCIL_READ
                    | wgt::TextureUses::DEPTH_STENCIL_WRITE
                    | wgt::TextureUses::COPY_SRC
                    | wgt::TextureUses::COPY_DST,
            ),
        }),
        wgt::TextureFormat::Depth24PlusStencil8 => Some(TextureFormatSupport {
            image_format: dk::DkImageFormat::DkImageFormat_Z24S8,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::DEPTH_STENCIL_READ
                    | wgt::TextureUses::DEPTH_STENCIL_WRITE,
            ),
        }),
        wgt::TextureFormat::Depth32FloatStencil8 => Some(TextureFormatSupport {
            image_format: dk::DkImageFormat::DkImageFormat_ZF32_X24S8,
            usage: TextureUsageRequirement::Contains(
                wgt::TextureUses::RESOURCE
                    | wgt::TextureUses::DEPTH_STENCIL_READ
                    | wgt::TextureUses::DEPTH_STENCIL_WRITE,
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
    let capabilities = match format {
        wgt::TextureFormat::Rgba8Unorm
        | wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Bgra8Unorm
        | wgt::TextureFormat::Bgra8UnormSrgb
        | wgt::TextureFormat::Rgba16Float => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT_BLEND
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
                | if matches!(
                    format,
                    wgt::TextureFormat::Rgba8Unorm | wgt::TextureFormat::Rgba16Float
                ) {
                    crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                        | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                        | crate::TextureFormatCapabilities::STORAGE_READ_WRITE
                } else {
                    crate::TextureFormatCapabilities::empty()
                }
        }
        wgt::TextureFormat::Rgba8Uint | wgt::TextureFormat::Rgba8Sint => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::Depth16Unorm | wgt::TextureFormat::Depth32Float => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::Depth24PlusStencil8 => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT
        }
        wgt::TextureFormat::Depth32FloatStencil8 => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT
        }
        wgt::TextureFormat::Stencil8 => {
            crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::R8Unorm | wgt::TextureFormat::Rg8Unorm => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
                | crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                | crate::TextureFormatCapabilities::STORAGE_READ_WRITE
        }
        wgt::TextureFormat::R8Uint
        | wgt::TextureFormat::R8Sint
        | wgt::TextureFormat::Rg8Uint
        | wgt::TextureFormat::Rg8Sint
        | wgt::TextureFormat::R8Snorm
        | wgt::TextureFormat::Rg8Snorm
        | wgt::TextureFormat::Rgba8Snorm => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::R16Float | wgt::TextureFormat::Rg16Float => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::R16Unorm
        | wgt::TextureFormat::R16Snorm
        | wgt::TextureFormat::Rg16Unorm
        | wgt::TextureFormat::Rg16Snorm => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::Rgba16Unorm | wgt::TextureFormat::Rgba16Snorm => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::R16Uint
        | wgt::TextureFormat::R16Sint
        | wgt::TextureFormat::Rg16Uint
        | wgt::TextureFormat::Rg16Sint
        | wgt::TextureFormat::Rgba16Uint
        | wgt::TextureFormat::Rgba16Sint => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        wgt::TextureFormat::R32Float
        | wgt::TextureFormat::R32Uint
        | wgt::TextureFormat::R32Sint
        | wgt::TextureFormat::Rg32Float
        | wgt::TextureFormat::Rg32Uint
        | wgt::TextureFormat::Rg32Sint
        | wgt::TextureFormat::Rgba32Float
        | wgt::TextureFormat::Rgba32Uint
        | wgt::TextureFormat::Rgba32Sint => {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        }
        _ => crate::TextureFormatCapabilities::empty(),
    };
    let attachment = capabilities.intersects(
        crate::TextureFormatCapabilities::COLOR_ATTACHMENT
            | crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT,
    );
    let resolve = capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT);
    let mut capabilities = capabilities;
    if attachment {
        capabilities |= crate::TextureFormatCapabilities::MULTISAMPLE_X2
            | crate::TextureFormatCapabilities::MULTISAMPLE_X4
            | crate::TextureFormatCapabilities::MULTISAMPLE_X8;
    }
    if resolve {
        capabilities |= crate::TextureFormatCapabilities::MULTISAMPLE_RESOLVE;
    }
    capabilities
}

#[cfg(target_os = "horizon")]
fn map_texture_image_format(format: wgt::TextureFormat) -> Option<dk::DkImageFormat> {
    match format {
        wgt::TextureFormat::R8Unorm => Some(dk::DkImageFormat::DkImageFormat_R8_Unorm),
        wgt::TextureFormat::R8Uint => Some(dk::DkImageFormat::DkImageFormat_R8_Uint),
        wgt::TextureFormat::R8Sint => Some(dk::DkImageFormat::DkImageFormat_R8_Sint),
        wgt::TextureFormat::R8Snorm => Some(dk::DkImageFormat::DkImageFormat_R8_Snorm),
        wgt::TextureFormat::Rg8Unorm => Some(dk::DkImageFormat::DkImageFormat_RG8_Unorm),
        wgt::TextureFormat::Rg8Uint => Some(dk::DkImageFormat::DkImageFormat_RG8_Uint),
        wgt::TextureFormat::Rg8Sint => Some(dk::DkImageFormat::DkImageFormat_RG8_Sint),
        wgt::TextureFormat::Rg8Snorm => Some(dk::DkImageFormat::DkImageFormat_RG8_Snorm),
        wgt::TextureFormat::R16Float => Some(dk::DkImageFormat::DkImageFormat_R16_Float),
        wgt::TextureFormat::Rg16Float => Some(dk::DkImageFormat::DkImageFormat_RG16_Float),
        wgt::TextureFormat::R16Unorm => Some(dk::DkImageFormat::DkImageFormat_R16_Unorm),
        wgt::TextureFormat::R16Snorm => Some(dk::DkImageFormat::DkImageFormat_R16_Snorm),
        wgt::TextureFormat::R16Uint => Some(dk::DkImageFormat::DkImageFormat_R16_Uint),
        wgt::TextureFormat::R16Sint => Some(dk::DkImageFormat::DkImageFormat_R16_Sint),
        wgt::TextureFormat::R32Float => Some(dk::DkImageFormat::DkImageFormat_R32_Float),
        wgt::TextureFormat::R32Uint => Some(dk::DkImageFormat::DkImageFormat_R32_Uint),
        wgt::TextureFormat::R32Sint => Some(dk::DkImageFormat::DkImageFormat_R32_Sint),
        wgt::TextureFormat::Rg16Unorm => Some(dk::DkImageFormat::DkImageFormat_RG16_Unorm),
        wgt::TextureFormat::Rg16Snorm => Some(dk::DkImageFormat::DkImageFormat_RG16_Snorm),
        wgt::TextureFormat::Rg16Uint => Some(dk::DkImageFormat::DkImageFormat_RG16_Uint),
        wgt::TextureFormat::Rg16Sint => Some(dk::DkImageFormat::DkImageFormat_RG16_Sint),
        wgt::TextureFormat::Rg32Float => Some(dk::DkImageFormat::DkImageFormat_RG32_Float),
        wgt::TextureFormat::Rg32Uint => Some(dk::DkImageFormat::DkImageFormat_RG32_Uint),
        wgt::TextureFormat::Rg32Sint => Some(dk::DkImageFormat::DkImageFormat_RG32_Sint),
        wgt::TextureFormat::Depth16Unorm => Some(dk::DkImageFormat::DkImageFormat_Z16),
        wgt::TextureFormat::Depth32Float => Some(dk::DkImageFormat::DkImageFormat_ZF32),
        wgt::TextureFormat::Depth24PlusStencil8 => Some(dk::DkImageFormat::DkImageFormat_Z24S8),
        wgt::TextureFormat::Depth32FloatStencil8 => {
            Some(dk::DkImageFormat::DkImageFormat_ZF32_X24S8)
        }
        wgt::TextureFormat::Stencil8 => Some(dk::DkImageFormat::DkImageFormat_S8),
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
        wgt::TextureFormat::Rgba8Uint => Some(dk::DkImageFormat::DkImageFormat_RGBA8_Uint),
        wgt::TextureFormat::Rgba8Sint => Some(dk::DkImageFormat::DkImageFormat_RGBA8_Sint),
        wgt::TextureFormat::Rgba8Snorm => Some(dk::DkImageFormat::DkImageFormat_RGBA8_Snorm),
        wgt::TextureFormat::Rgba16Unorm => Some(dk::DkImageFormat::DkImageFormat_RGBA16_Unorm),
        wgt::TextureFormat::Rgba16Snorm => Some(dk::DkImageFormat::DkImageFormat_RGBA16_Snorm),
        wgt::TextureFormat::Rgba16Uint => Some(dk::DkImageFormat::DkImageFormat_RGBA16_Uint),
        wgt::TextureFormat::Rgba16Sint => Some(dk::DkImageFormat::DkImageFormat_RGBA16_Sint),
        wgt::TextureFormat::Rgba32Float => Some(dk::DkImageFormat::DkImageFormat_RGBA32_Float),
        wgt::TextureFormat::Rgba32Uint => Some(dk::DkImageFormat::DkImageFormat_RGBA32_Uint),
        wgt::TextureFormat::Rgba32Sint => Some(dk::DkImageFormat::DkImageFormat_RGBA32_Sint),
        wgt::TextureFormat::Rgba16Float => Some(dk::DkImageFormat::DkImageFormat_RGBA16_Float),
        _ => None,
    }
}

impl QuerySetInner {
    fn is_timestamp(&self) -> bool {
        matches!(self.query_type, wgt::QueryType::Timestamp)
    }

    fn is_occlusion(&self) -> bool {
        matches!(self.query_type, wgt::QueryType::Occlusion)
    }

    fn pipeline_statistics(&self) -> Option<wgt::PipelineStatisticsTypes> {
        match self.query_type {
            wgt::QueryType::PipelineStatistics(statistics) => Some(statistics),
            _ => None,
        }
    }

    fn is_pipeline_statistics(&self) -> bool {
        self.pipeline_statistics()
            .is_some_and(Self::supports_pipeline_statistics)
    }

    fn supports_pipeline_statistics(statistics: wgt::PipelineStatisticsTypes) -> bool {
        !statistics.is_empty() && deko_pipeline_statistics().contains(statistics)
    }

    fn is_supported_type(query_type: &wgt::QueryType) -> bool {
        match query_type {
            wgt::QueryType::Timestamp | wgt::QueryType::Occlusion => true,
            wgt::QueryType::PipelineStatistics(statistics) => {
                Self::supports_pipeline_statistics(*statistics)
            }
        }
    }

    #[cfg(any(target_os = "horizon", test))]
    fn result_count_for(query_type: &wgt::QueryType) -> u32 {
        match query_type {
            wgt::QueryType::PipelineStatistics(statistics) => statistics.bits().count_ones(),
            _ => 1,
        }
    }

    #[cfg(any(target_os = "horizon", test))]
    fn result_count(&self) -> u32 {
        Self::result_count_for(&self.query_type)
    }

    fn is_supported(&self) -> bool {
        Self::is_supported_type(&self.query_type)
    }

    fn validate_range(&self, range: core::ops::Range<u32>) -> DeviceResult<()> {
        if !self.is_supported() || range.start > range.end || range.end > self.count {
            return Err(crate::DeviceError::Lost);
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    fn new(
        raw_device: dk::DkDevice,
        desc: &wgt::QuerySetDescriptor<crate::Label>,
    ) -> DeviceResult<Self> {
        if !Self::is_supported_type(&desc.ty) || desc.count == 0 {
            return Err(crate::DeviceError::Lost);
        }
        let size = u64::from(desc.count)
            .checked_mul(u64::from(Self::result_count_for(&desc.ty)))
            .and_then(|size| size.checked_mul(DEKO_COUNTER_REPORT_SIZE))
            .ok_or(crate::DeviceError::OutOfMemory)?;
        let report_buffer = Buffer::new(
            &crate::BufferDescriptor {
                label: None,
                size,
                usage: wgt::BufferUses::COPY_SRC,
                memory_flags: crate::MemoryFlags::empty(),
            },
            raw_device,
        )?;
        Ok(Self {
            query_type: desc.ty,
            count: desc.count,
            report_buffer,
        })
    }

    #[cfg(target_os = "horizon")]
    fn report_address(&self, index: u32, component: u32) -> DeviceResult<dk::DkGpuAddr> {
        self.validate_range(index..index.checked_add(1).ok_or(crate::DeviceError::Lost)?)?;
        if component >= self.result_count() {
            return Err(crate::DeviceError::Lost);
        }
        let report_index = u64::from(index)
            .checked_mul(u64::from(self.result_count()))
            .and_then(|offset| offset.checked_add(u64::from(component)))
            .ok_or(crate::DeviceError::Lost)?;
        let offset = report_index
            .checked_mul(DEKO_COUNTER_REPORT_SIZE)
            .ok_or(crate::DeviceError::Lost)?;
        let size =
            wgt::BufferSize::new(DEKO_COUNTER_REPORT_SIZE).ok_or(crate::DeviceError::Lost)?;
        self.report_buffer
            .gpu_binding(offset, Some(size))
            .map(|(addr, _)| addr)
    }
}

#[cfg(target_os = "horizon")]
fn map_texture_view_dimension(
    dimension: wgt::TextureViewDimension,
) -> DeviceResult<Option<dk::DkImageType>> {
    match dimension {
        wgt::TextureViewDimension::D1 => Ok(Some(dk::DkImageType::DkImageType_1D)),
        wgt::TextureViewDimension::D2 => Ok(None),
        wgt::TextureViewDimension::D3 => Ok(Some(dk::DkImageType::DkImageType_3D)),
        wgt::TextureViewDimension::D2Array => Ok(Some(dk::DkImageType::DkImageType_2DArray)),
        wgt::TextureViewDimension::Cube => Ok(Some(dk::DkImageType::DkImageType_Cubemap)),
        wgt::TextureViewDimension::CubeArray => Ok(Some(dk::DkImageType::DkImageType_CubemapArray)),
    }
}

fn texture_view_dimension_matches(
    texture: wgt::TextureDimension,
    view: wgt::TextureViewDimension,
) -> bool {
    match texture {
        wgt::TextureDimension::D1 => view == wgt::TextureViewDimension::D1,
        wgt::TextureDimension::D2 => matches!(
            view,
            wgt::TextureViewDimension::D2
                | wgt::TextureViewDimension::D2Array
                | wgt::TextureViewDimension::Cube
                | wgt::TextureViewDimension::CubeArray
        ),
        wgt::TextureDimension::D3 => view == wgt::TextureViewDimension::D3,
    }
}

#[cfg(target_os = "horizon")]
impl SamplerInner {
    fn new(desc: &crate::SamplerDescriptor) -> DeviceResult<Self> {
        let mut sampler = dk::DkSampler::defaults();
        sampler.minFilter = map_filter_mode(desc.min_filter);
        sampler.magFilter = map_filter_mode(desc.mag_filter);
        sampler.mipFilter = map_mipmap_filter_mode(desc.mipmap_filter);
        sampler.wrapMode = [
            map_address_mode(desc.address_modes[0])?,
            map_address_mode(desc.address_modes[1])?,
            map_address_mode(desc.address_modes[2])?,
        ];
        sampler.lodClampMin = desc.lod_clamp.start;
        sampler.lodClampMax = desc.lod_clamp.end;
        sampler.maxAnisotropy = f32::from(desc.anisotropy_clamp);
        if let Some(border_color) = desc.border_color {
            sampler.borderColor = map_sampler_border_color(border_color);
        }
        if let Some(compare) = desc.compare {
            sampler.compareEnable = true;
            sampler.compareOp = map_compare_function(compare);
        }

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
            BindGroupLayoutKind::SampledTexture {
                binding,
                visibility,
            } => unsafe {
                Self::new_sampled_texture(texture_descriptor_heap, desc, *binding, *visibility)
            },
            BindGroupLayoutKind::TextureSampler {
                texture_binding,
                sampler_binding,
                count,
                visibility,
            } => unsafe {
                Self::new_texture_sampler(
                    texture_descriptor_heap,
                    desc,
                    *texture_binding,
                    *sampler_binding,
                    *count,
                    *visibility,
                )
            },
            BindGroupLayoutKind::UniformBuffer {
                binding,
                count,
                visibility,
                has_dynamic_offset,
            } => Self::new_uniform_buffer(desc, *binding, *count, *visibility, *has_dynamic_offset),
            BindGroupLayoutKind::StorageBuffer {
                binding,
                count,
                visibility,
                read_only,
                has_dynamic_offset,
            } => Self::new_storage_buffer(
                desc,
                *binding,
                *count,
                *visibility,
                *read_only,
                *has_dynamic_offset,
            ),
            BindGroupLayoutKind::StorageTexture {
                binding,
                count,
                visibility,
                access,
                format,
            } => unsafe {
                Self::new_storage_texture(
                    texture_descriptor_heap,
                    desc,
                    *binding,
                    *count,
                    *visibility,
                    *access,
                    *format,
                )
            },
            BindGroupLayoutKind::BufferGroup(entries) => Self::new_buffer_group(desc, entries),
            BindGroupLayoutKind::BufferSampledTextureGroup {
                buffers,
                sampled_textures,
            } => unsafe {
                Self::new_buffer_sampled_texture_group(
                    texture_descriptor_heap,
                    desc,
                    buffers,
                    sampled_textures,
                )
            },
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

    unsafe fn new_sampled_texture(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
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

        let sampled_texture = unsafe {
            Self::make_texture_sampler_binding(
                texture_descriptor_heap,
                desc,
                binding,
                None,
                1,
                visibility,
            )?
        };

        Ok(Self {
            inner: BindGroupInnerRaw::TextureSampler(sampled_texture),
        })
    }

    unsafe fn new_texture_sampler(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        texture_binding: u32,
        sampler_binding: u32,
        count: u32,
        visibility: wgt::ShaderStages,
    ) -> DeviceResult<Self> {
        let count = usize::try_from(count).map_err(|_| crate::DeviceError::Lost)?;
        if desc.buffers.len() != 0
            || desc.samplers.len() != count
            || desc.textures.len() != count
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
                Some(sampler_binding),
                count as u32,
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
        sampler_binding: Option<u32>,
        count: u32,
        visibility: wgt::ShaderStages,
    ) -> DeviceResult<TextureSamplerBinding> {
        deko_texture_binding_mask(texture_binding, count).ok_or(crate::DeviceError::Lost)?;
        let Some(texture_entry) = desc
            .entries
            .iter()
            .find(|entry| entry.binding == texture_binding)
        else {
            return Err(crate::DeviceError::Lost);
        };
        if texture_entry.count != count {
            return Err(crate::DeviceError::Lost);
        }
        let count = usize::try_from(count).map_err(|_| crate::DeviceError::Lost)?;
        let texture_start =
            usize::try_from(texture_entry.resource_index).map_err(|_| crate::DeviceError::Lost)?;
        let textures = desc
            .textures
            .get(
                texture_start
                    ..texture_start
                        .checked_add(count)
                        .ok_or(crate::DeviceError::Lost)?,
            )
            .ok_or(crate::DeviceError::Lost)?;
        let samplers = if let Some(sampler_binding) = sampler_binding {
            let Some(sampler_entry) = desc
                .entries
                .iter()
                .find(|entry| entry.binding == sampler_binding)
            else {
                return Err(crate::DeviceError::Lost);
            };
            if sampler_entry.count != count as u32 {
                return Err(crate::DeviceError::Lost);
            }
            let sampler_start = usize::try_from(sampler_entry.resource_index)
                .map_err(|_| crate::DeviceError::Lost)?;
            Some(
                desc.samplers
                    .get(
                        sampler_start
                            ..sampler_start
                                .checked_add(count)
                                .ok_or(crate::DeviceError::Lost)?,
                    )
                    .ok_or(crate::DeviceError::Lost)?,
            )
        } else {
            None
        };
        let descriptor_slots = texture_descriptor_heap.allocate_slots(count as u32)?;
        let descriptor_count = descriptor_slots
            .first()
            .ok_or(crate::DeviceError::Lost)?
            .descriptor_count;
        let mut descriptors = Vec::with_capacity(count);
        let default_sampler = Arc::new(SamplerInner {
            inner: SamplerInnerRaw {
                sampler: dk::DkSampler::defaults(),
            },
        });

        for (index, (texture_binding_resource, descriptor_slot)) in
            textures.iter().zip(descriptor_slots).enumerate()
        {
            let sampler = if let Some(samplers) = samplers {
                let Resource::Sampler(ref sampler) = *samplers[index] else {
                    return Err(crate::DeviceError::Lost);
                };
                sampler.clone()
            } else {
                default_sampler.clone()
            };
            if !texture_binding_resource
                .usage
                .contains(wgt::TextureUses::RESOURCE)
            {
                return Err(crate::DeviceError::Lost);
            }
            let Resource::TextureView {
                image,
                format,
                aspect,
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

            let mut image_descriptor = dk::DkImageDescriptor::zeroed();
            let mut image_view = dk::DkImageView::defaults(image.0);
            image_view.format =
                map_texture_image_format(*format).ok_or(crate::DeviceError::Lost)?;
            if let Some(view_type) = map_texture_view_dimension(*view_dimension)? {
                image_view.type_ = view_type;
            }
            if *aspect == wgt::TextureAspect::StencilOnly || *format == wgt::TextureFormat::Stencil8
            {
                image_view.dsSource = dk::DkDsSource::DkDsSource_Stencil;
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
            descriptors.push(TextureSamplerDescriptor {
                texture: texture.clone(),
                sampler,
                image_descriptor_set_gpu_addr: descriptor_slot.image_descriptor_set_gpu_addr,
                sampler_descriptor_set_gpu_addr: descriptor_slot.sampler_descriptor_set_gpu_addr,
                image_descriptor_gpu_addr: descriptor_slot.image_descriptor_gpu_addr,
                sampler_descriptor_gpu_addr: descriptor_slot.sampler_descriptor_gpu_addr,
                image_descriptor_index: descriptor_slot.image_descriptor_index,
                sampler_descriptor_index: descriptor_slot.sampler_descriptor_index,
                image_descriptor,
                sampler_descriptor,
                _lease: descriptor_slot.lease,
            });
        }
        let handles = descriptors
            .iter()
            .map(|descriptor| {
                dk::dkMakeTextureHandle(
                    descriptor.image_descriptor_index,
                    descriptor.sampler_descriptor_index,
                )
            })
            .collect();

        Ok(TextureSamplerBinding {
            texture_binding,
            visibility,
            sampler_binding: sampler_binding.unwrap_or(texture_binding),
            descriptors,
            handles,
            descriptor_count,
        })
    }

    unsafe fn new_storage_texture(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        count: u32,
        visibility: wgt::ShaderStages,
        access: wgt::StorageTextureAccess,
        format: wgt::TextureFormat,
    ) -> DeviceResult<Self> {
        let count = usize::try_from(count).map_err(|_| crate::DeviceError::Lost)?;
        if !desc.buffers.is_empty()
            || !desc.samplers.is_empty()
            || desc.textures.len() != count
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 1
        {
            return Err(crate::DeviceError::Lost);
        }

        let entry = &desc.entries[0];
        if entry.binding != binding || entry.resource_index != 0 || entry.count != count as u32 {
            return Err(crate::DeviceError::Lost);
        }

        let storage_texture = unsafe {
            Self::make_storage_texture_binding(
                texture_descriptor_heap,
                desc,
                binding,
                visibility,
                access,
                format,
                count as u32,
            )?
        };

        Ok(Self {
            inner: BindGroupInnerRaw::StorageTexture(storage_texture),
        })
    }

    unsafe fn make_storage_texture_binding(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
        access: wgt::StorageTextureAccess,
        format: wgt::TextureFormat,
        count: u32,
    ) -> DeviceResult<StorageTextureBinding> {
        deko_image_binding_mask(binding, count).ok_or(crate::DeviceError::Lost)?;
        let count = usize::try_from(count).map_err(|_| crate::DeviceError::Lost)?;
        let entry = desc
            .entries
            .iter()
            .find(|entry| entry.binding == binding)
            .ok_or(crate::DeviceError::Lost)?;
        if entry.count != count as u32 {
            return Err(crate::DeviceError::Lost);
        }
        let texture_start =
            usize::try_from(entry.resource_index).map_err(|_| crate::DeviceError::Lost)?;
        let texture_bindings = desc
            .textures
            .get(
                texture_start
                    ..texture_start
                        .checked_add(count)
                        .ok_or(crate::DeviceError::Lost)?,
            )
            .ok_or(crate::DeviceError::Lost)?;
        let required_usage = match access {
            wgt::StorageTextureAccess::ReadOnly => wgt::TextureUses::STORAGE_READ_ONLY,
            wgt::StorageTextureAccess::WriteOnly => wgt::TextureUses::STORAGE_WRITE_ONLY,
            wgt::StorageTextureAccess::ReadWrite => wgt::TextureUses::STORAGE_READ_WRITE,
            wgt::StorageTextureAccess::Atomic => return Err(crate::DeviceError::Lost),
        };
        if !matches!(
            format,
            wgt::TextureFormat::R8Unorm
                | wgt::TextureFormat::Rg8Unorm
                | wgt::TextureFormat::Rgba8Unorm
                | wgt::TextureFormat::Rgba16Float
        ) {
            return Err(crate::DeviceError::Lost);
        }
        let descriptor_slots = texture_descriptor_heap.allocate_slots(count as u32)?;
        let descriptor_count = descriptor_slots
            .first()
            .ok_or(crate::DeviceError::Lost)?
            .descriptor_count;
        let mut descriptors = Vec::with_capacity(count);
        for (texture_binding, descriptor_slot) in texture_bindings.iter().zip(descriptor_slots) {
            if !texture_binding.usage.contains(required_usage) {
                return Err(crate::DeviceError::Lost);
            }
            let Resource::TextureView {
                image,
                format: view_format,
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
            let mut image_descriptor = dk::DkImageDescriptor::zeroed();
            let mut image_view = dk::DkImageView::defaults(image.0);
            image_view.format =
                map_texture_image_format(*view_format).ok_or(crate::DeviceError::Lost)?;
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
            descriptors.push(StorageTextureDescriptor {
                texture: texture.clone(),
                image_descriptor_set_gpu_addr: descriptor_slot.image_descriptor_set_gpu_addr,
                image_descriptor_gpu_addr: descriptor_slot.image_descriptor_gpu_addr,
                image_descriptor_index: descriptor_slot.image_descriptor_index,
                image_descriptor,
                _lease: descriptor_slot.lease,
            });
        }

        Ok(StorageTextureBinding {
            binding,
            visibility,
            access,
            format,
            descriptors,
            descriptor_count,
        })
    }

    fn make_buffer_bindings(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        layout_entries: &[BufferBindGroupLayoutKind],
    ) -> DeviceResult<Vec<BufferBinding>> {
        let expected_count = layout_entries.iter().try_fold(0usize, |total, entry| {
            total
                .checked_add(entry.count() as usize)
                .ok_or(crate::DeviceError::Lost)
        })?;
        if desc.buffers.len() != expected_count || desc.entries.len() != layout_entries.len() {
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
            if entry.count != layout_entry.count() {
                return Err(crate::DeviceError::Lost);
            }
            let count = usize::try_from(entry.count).map_err(|_| crate::DeviceError::Lost)?;
            let start =
                usize::try_from(entry.resource_index).map_err(|_| crate::DeviceError::Lost)?;
            let buffers = desc
                .buffers
                .get(start..start.checked_add(count).ok_or(crate::DeviceError::Lost)?)
                .ok_or(crate::DeviceError::Lost)?;

            match *layout_entry {
                BufferBindGroupLayoutKind::Uniform {
                    binding,
                    count,
                    visibility,
                    has_dynamic_offset,
                } if count == 1 => {
                    bindings.push(BufferBinding::Uniform(Self::make_uniform_buffer_binding(
                        binding,
                        visibility,
                        has_dynamic_offset,
                        &buffers[0],
                    )?));
                }
                BufferBindGroupLayoutKind::Uniform {
                    binding,
                    visibility,
                    has_dynamic_offset: false,
                    ..
                } => {
                    let mut array = Vec::with_capacity(buffers.len());
                    for (index, buffer) in buffers.iter().enumerate() {
                        array.push(Self::make_uniform_buffer_binding(
                            binding
                                .checked_add(index as u32)
                                .ok_or(crate::DeviceError::Lost)?,
                            visibility,
                            false,
                            buffer,
                        )?);
                    }
                    bindings.push(BufferBinding::UniformArray(array));
                }
                BufferBindGroupLayoutKind::Storage {
                    binding,
                    count,
                    visibility,
                    read_only,
                    has_dynamic_offset,
                } if count == 1 => {
                    bindings.push(BufferBinding::Storage(Self::make_storage_buffer_binding(
                        binding,
                        visibility,
                        read_only,
                        has_dynamic_offset,
                        &buffers[0],
                    )?));
                }
                BufferBindGroupLayoutKind::Storage {
                    binding,
                    visibility,
                    read_only,
                    has_dynamic_offset: false,
                    ..
                } => {
                    let mut array = Vec::with_capacity(buffers.len());
                    for (index, buffer) in buffers.iter().enumerate() {
                        array.push(Self::make_storage_buffer_binding(
                            binding
                                .checked_add(index as u32)
                                .ok_or(crate::DeviceError::Lost)?,
                            visibility,
                            read_only,
                            false,
                            buffer,
                        )?);
                    }
                    bindings.push(BufferBinding::StorageArray(array));
                }
                _ => return Err(crate::DeviceError::Lost),
            }
        }

        Ok(bindings)
    }

    fn new_uniform_buffer(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        count: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
    ) -> DeviceResult<Self> {
        let count_usize = usize::try_from(count).map_err(|_| crate::DeviceError::Lost)?;
        if desc.buffers.len() != count_usize
            || !desc.samplers.is_empty()
            || !desc.textures.is_empty()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 1
        {
            return Err(crate::DeviceError::Lost);
        }

        let entry = &desc.entries[0];
        if entry.binding != binding || entry.resource_index != 0 || entry.count != count {
            return Err(crate::DeviceError::Lost);
        }

        let inner = if count == 1 {
            BindGroupInnerRaw::UniformBuffer(Self::make_uniform_buffer_binding(
                binding,
                visibility,
                has_dynamic_offset,
                &desc.buffers[0],
            )?)
        } else {
            if has_dynamic_offset {
                return Err(crate::DeviceError::Lost);
            }
            let mut array = Vec::with_capacity(count_usize);
            for (index, buffer) in desc.buffers.iter().enumerate() {
                array.push(Self::make_uniform_buffer_binding(
                    binding
                        .checked_add(index as u32)
                        .ok_or(crate::DeviceError::Lost)?,
                    visibility,
                    false,
                    buffer,
                )?);
            }
            BindGroupInnerRaw::BufferGroup(vec![BufferBinding::UniformArray(array)])
        };
        Ok(Self { inner })
    }

    fn new_storage_buffer(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        count: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
    ) -> DeviceResult<Self> {
        if !desc.samplers.is_empty()
            || !desc.textures.is_empty()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 1
        {
            return Err(crate::DeviceError::Lost);
        }

        let bindings = Self::make_buffer_bindings(
            desc,
            &[BufferBindGroupLayoutKind::Storage {
                binding,
                count,
                visibility,
                read_only,
                has_dynamic_offset,
            }],
        )?;
        match bindings.as_slice() {
            [BufferBinding::Storage(binding)] => Ok(Self {
                inner: BindGroupInnerRaw::StorageBuffer(binding.clone()),
            }),
            _ => Ok(Self {
                inner: BindGroupInnerRaw::BufferGroup(bindings),
            }),
        }
    }

    fn new_buffer_group(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        layout_entries: &[BufferBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        if !desc.samplers.is_empty()
            || !desc.textures.is_empty()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
        {
            return Err(crate::DeviceError::Lost);
        }
        let bindings = Self::make_buffer_bindings(desc, layout_entries)?;

        Ok(Self {
            inner: BindGroupInnerRaw::BufferGroup(bindings),
        })
    }

    unsafe fn new_buffer_sampled_texture_group(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        buffer_layouts: &[BufferBindGroupLayoutKind],
        sampled_texture_layouts: &[SampledTextureBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        if !desc.samplers.is_empty()
            || desc.textures.len() != sampled_texture_layouts.len()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != buffer_layouts.len() + sampled_texture_layouts.len()
        {
            return Err(crate::DeviceError::Lost);
        }

        let buffers = Self::make_buffer_bindings(desc, buffer_layouts)?;

        let mut sampled_textures = Vec::with_capacity(sampled_texture_layouts.len());
        for layout in sampled_texture_layouts {
            sampled_textures.push(unsafe {
                Self::make_texture_sampler_binding(
                    texture_descriptor_heap,
                    desc,
                    layout.texture_binding,
                    None,
                    1,
                    layout.visibility,
                )?
            });
        }

        Ok(Self {
            inner: BindGroupInnerRaw::BufferSampledTextureGroup {
                buffers,
                sampled_textures,
            },
        })
    }

    unsafe fn new_buffer_texture_sampler_group(
        texture_descriptor_heap: &TextureDescriptorHeap,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        buffer_layouts: &[BufferBindGroupLayoutKind],
        texture_sampler_layouts: &[TextureSamplerBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        let texture_sampler_count =
            texture_sampler_layouts
                .iter()
                .try_fold(0usize, |total, layout| {
                    total
                        .checked_add(
                            usize::try_from(layout.count).map_err(|_| crate::DeviceError::Lost)?,
                        )
                        .ok_or(crate::DeviceError::Lost)
                })?;
        if desc.samplers.len() != texture_sampler_count
            || desc.textures.len() != texture_sampler_count
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != buffer_layouts.len() + texture_sampler_layouts.len() * 2
        {
            return Err(crate::DeviceError::Lost);
        }

        let buffers = Self::make_buffer_bindings(desc, buffer_layouts)?;

        let mut texture_samplers = Vec::with_capacity(texture_sampler_layouts.len());
        for layout in texture_sampler_layouts {
            texture_samplers.push(unsafe {
                Self::make_texture_sampler_binding(
                    texture_descriptor_heap,
                    desc,
                    layout.texture_binding,
                    Some(layout.sampler_binding),
                    layout.count,
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
        let storage_texture_count =
            storage_texture_layouts
                .iter()
                .try_fold(0usize, |count, layout| {
                    count
                        .checked_add(
                            usize::try_from(layout.count).map_err(|_| crate::DeviceError::Lost)?,
                        )
                        .ok_or(crate::DeviceError::Lost)
                })?;
        if !desc.samplers.is_empty()
            || desc.textures.len() != storage_texture_count
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != buffer_layouts.len() + storage_texture_layouts.len()
        {
            return Err(crate::DeviceError::Lost);
        }

        let buffers = Self::make_buffer_bindings(desc, buffer_layouts)?;

        let mut storage_textures = Vec::with_capacity(storage_texture_layouts.len());
        for storage_texture_layout in storage_texture_layouts {
            storage_textures.push(unsafe {
                Self::make_storage_texture_binding(
                    texture_descriptor_heap,
                    desc,
                    storage_texture_layout.binding,
                    storage_texture_layout.visibility,
                    storage_texture_layout.access,
                    storage_texture_layout.format,
                    storage_texture_layout.count,
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
        let texture_sampler_count =
            texture_sampler_layouts
                .iter()
                .try_fold(0usize, |count, layout| {
                    count
                        .checked_add(
                            usize::try_from(layout.count).map_err(|_| crate::DeviceError::Lost)?,
                        )
                        .ok_or(crate::DeviceError::Lost)
                })?;
        let storage_texture_count =
            storage_texture_layouts
                .iter()
                .try_fold(0usize, |count, layout| {
                    count
                        .checked_add(
                            usize::try_from(layout.count).map_err(|_| crate::DeviceError::Lost)?,
                        )
                        .ok_or(crate::DeviceError::Lost)
                })?;
        if desc.samplers.len() != texture_sampler_count
            || desc.textures.len() != texture_sampler_count + storage_texture_count
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len()
                != buffer_layouts.len()
                    + texture_sampler_layouts.len() * 2
                    + storage_texture_layouts.len()
        {
            return Err(crate::DeviceError::Lost);
        }

        let buffers = Self::make_buffer_bindings(desc, buffer_layouts)?;

        let mut texture_samplers = Vec::with_capacity(texture_sampler_layouts.len());
        for layout in texture_sampler_layouts {
            texture_samplers.push(unsafe {
                Self::make_texture_sampler_binding(
                    texture_descriptor_heap,
                    desc,
                    layout.texture_binding,
                    Some(layout.sampler_binding),
                    layout.count,
                    layout.visibility,
                )?
            });
        }

        let mut storage_textures = Vec::with_capacity(storage_texture_layouts.len());
        for storage_texture_layout in storage_texture_layouts {
            storage_textures.push(unsafe {
                Self::make_storage_texture_binding(
                    texture_descriptor_heap,
                    desc,
                    storage_texture_layout.binding,
                    storage_texture_layout.visibility,
                    storage_texture_layout.access,
                    storage_texture_layout.format,
                    storage_texture_layout.count,
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
) -> Result<(dk::DkVtxAttribSize, dk::DkVtxAttribType, bool), crate::PipelineError> {
    use dk::DkVtxAttribSize as Size;
    use dk::DkVtxAttribType as Type;
    use wgt::VertexFormat as Format;

    match format {
        Format::Uint8 => Ok((Size::DkVtxAttribSize_1x8, Type::DkVtxAttribType_Uint, false)),
        Format::Uint8x2 => Ok((Size::DkVtxAttribSize_2x8, Type::DkVtxAttribType_Uint, false)),
        Format::Uint8x4 => Ok((Size::DkVtxAttribSize_4x8, Type::DkVtxAttribType_Uint, false)),
        Format::Sint8 => Ok((Size::DkVtxAttribSize_1x8, Type::DkVtxAttribType_Sint, false)),
        Format::Sint8x2 => Ok((Size::DkVtxAttribSize_2x8, Type::DkVtxAttribType_Sint, false)),
        Format::Sint8x4 => Ok((Size::DkVtxAttribSize_4x8, Type::DkVtxAttribType_Sint, false)),
        Format::Unorm8 => Ok((
            Size::DkVtxAttribSize_1x8,
            Type::DkVtxAttribType_Unorm,
            false,
        )),
        Format::Unorm8x2 => Ok((
            Size::DkVtxAttribSize_2x8,
            Type::DkVtxAttribType_Unorm,
            false,
        )),
        Format::Unorm8x4 => Ok((
            Size::DkVtxAttribSize_4x8,
            Type::DkVtxAttribType_Unorm,
            false,
        )),
        Format::Snorm8 => Ok((
            Size::DkVtxAttribSize_1x8,
            Type::DkVtxAttribType_Snorm,
            false,
        )),
        Format::Snorm8x2 => Ok((
            Size::DkVtxAttribSize_2x8,
            Type::DkVtxAttribType_Snorm,
            false,
        )),
        Format::Snorm8x4 => Ok((
            Size::DkVtxAttribSize_4x8,
            Type::DkVtxAttribType_Snorm,
            false,
        )),
        Format::Uint16 => Ok((
            Size::DkVtxAttribSize_1x16,
            Type::DkVtxAttribType_Uint,
            false,
        )),
        Format::Uint16x2 => Ok((
            Size::DkVtxAttribSize_2x16,
            Type::DkVtxAttribType_Uint,
            false,
        )),
        Format::Uint16x4 => Ok((
            Size::DkVtxAttribSize_4x16,
            Type::DkVtxAttribType_Uint,
            false,
        )),
        Format::Sint16 => Ok((
            Size::DkVtxAttribSize_1x16,
            Type::DkVtxAttribType_Sint,
            false,
        )),
        Format::Sint16x2 => Ok((
            Size::DkVtxAttribSize_2x16,
            Type::DkVtxAttribType_Sint,
            false,
        )),
        Format::Sint16x4 => Ok((
            Size::DkVtxAttribSize_4x16,
            Type::DkVtxAttribType_Sint,
            false,
        )),
        Format::Unorm16 => Ok((
            Size::DkVtxAttribSize_1x16,
            Type::DkVtxAttribType_Unorm,
            false,
        )),
        Format::Unorm16x2 => Ok((
            Size::DkVtxAttribSize_2x16,
            Type::DkVtxAttribType_Unorm,
            false,
        )),
        Format::Unorm16x4 => Ok((
            Size::DkVtxAttribSize_4x16,
            Type::DkVtxAttribType_Unorm,
            false,
        )),
        Format::Snorm16 => Ok((
            Size::DkVtxAttribSize_1x16,
            Type::DkVtxAttribType_Snorm,
            false,
        )),
        Format::Snorm16x2 => Ok((
            Size::DkVtxAttribSize_2x16,
            Type::DkVtxAttribType_Snorm,
            false,
        )),
        Format::Snorm16x4 => Ok((
            Size::DkVtxAttribSize_4x16,
            Type::DkVtxAttribType_Snorm,
            false,
        )),
        Format::Float16 => Ok((
            Size::DkVtxAttribSize_1x16,
            Type::DkVtxAttribType_Float,
            false,
        )),
        Format::Float16x2 => Ok((
            Size::DkVtxAttribSize_2x16,
            Type::DkVtxAttribType_Float,
            false,
        )),
        Format::Float16x4 => Ok((
            Size::DkVtxAttribSize_4x16,
            Type::DkVtxAttribType_Float,
            false,
        )),
        Format::Float32 => Ok((
            Size::DkVtxAttribSize_1x32,
            Type::DkVtxAttribType_Float,
            false,
        )),
        Format::Float32x2 => Ok((
            Size::DkVtxAttribSize_2x32,
            Type::DkVtxAttribType_Float,
            false,
        )),
        Format::Float32x3 => Ok((
            Size::DkVtxAttribSize_3x32,
            Type::DkVtxAttribType_Float,
            false,
        )),
        Format::Float32x4 => Ok((
            Size::DkVtxAttribSize_4x32,
            Type::DkVtxAttribType_Float,
            false,
        )),
        Format::Uint32 => Ok((
            Size::DkVtxAttribSize_1x32,
            Type::DkVtxAttribType_Uint,
            false,
        )),
        Format::Uint32x2 => Ok((
            Size::DkVtxAttribSize_2x32,
            Type::DkVtxAttribType_Uint,
            false,
        )),
        Format::Uint32x3 => Ok((
            Size::DkVtxAttribSize_3x32,
            Type::DkVtxAttribType_Uint,
            false,
        )),
        Format::Uint32x4 => Ok((
            Size::DkVtxAttribSize_4x32,
            Type::DkVtxAttribType_Uint,
            false,
        )),
        Format::Sint32 => Ok((
            Size::DkVtxAttribSize_1x32,
            Type::DkVtxAttribType_Sint,
            false,
        )),
        Format::Sint32x2 => Ok((
            Size::DkVtxAttribSize_2x32,
            Type::DkVtxAttribType_Sint,
            false,
        )),
        Format::Sint32x3 => Ok((
            Size::DkVtxAttribSize_3x32,
            Type::DkVtxAttribType_Sint,
            false,
        )),
        Format::Sint32x4 => Ok((
            Size::DkVtxAttribSize_4x32,
            Type::DkVtxAttribType_Sint,
            false,
        )),
        Format::Unorm10_10_10_2 => Ok((
            Size::DkVtxAttribSize_10_10_10_2,
            Type::DkVtxAttribType_Unorm,
            false,
        )),
        Format::Unorm8x4Bgra => Ok((Size::DkVtxAttribSize_4x8, Type::DkVtxAttribType_Unorm, true)),
        Format::Float64 | Format::Float64x2 | Format::Float64x3 | Format::Float64x4 => {
            Err(crate::PipelineError::Device(crate::DeviceError::Lost))
        }
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
fn map_mipmap_filter_mode(filter: wgt::MipmapFilterMode) -> dk::DkMipFilter {
    match filter {
        wgt::MipmapFilterMode::Nearest => dk::DkMipFilter::DkMipFilter_Nearest,
        wgt::MipmapFilterMode::Linear => dk::DkMipFilter::DkMipFilter_Linear,
    }
}

#[cfg(target_os = "horizon")]
fn map_address_mode(address: wgt::AddressMode) -> DeviceResult<dk::DkWrapMode> {
    match address {
        wgt::AddressMode::ClampToEdge => Ok(dk::DkWrapMode::DkWrapMode_ClampToEdge),
        wgt::AddressMode::Repeat => Ok(dk::DkWrapMode::DkWrapMode_Repeat),
        wgt::AddressMode::MirrorRepeat => Ok(dk::DkWrapMode::DkWrapMode_MirroredRepeat),
        wgt::AddressMode::ClampToBorder => Ok(dk::DkWrapMode::DkWrapMode_ClampToBorder),
    }
}

#[cfg(target_os = "horizon")]
fn map_sampler_border_color(
    border_color: wgt::SamplerBorderColor,
) -> [dk::DkSamplerBorderColor; 4] {
    let color = match border_color {
        wgt::SamplerBorderColor::TransparentBlack | wgt::SamplerBorderColor::Zero => [0.0; 4],
        wgt::SamplerBorderColor::OpaqueBlack => [0.0, 0.0, 0.0, 1.0],
        wgt::SamplerBorderColor::OpaqueWhite => [1.0; 4],
    };
    color.map(|value_f| dk::DkSamplerBorderColor { value_f })
}

fn supported_bind_group_layout_kind(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    if entries.len() == 1 {
        if let Some(kind) = supported_buffer_bind_group_layout_kind(entries[0]) {
            return Some(kind);
        }
        if let Some(kind) = supported_sampled_texture_bind_group_layout_kind(entries[0]) {
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
        if let Some(kind) = supported_buffer_sampled_texture_bind_group_layout_kind_group(entries) {
            return Some(kind);
        }
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

fn supported_sampled_texture_bind_group_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<BindGroupLayoutKind> {
    let sampled_texture = supported_sampled_texture_binding_layout_kind(entry)?;
    Some(BindGroupLayoutKind::SampledTexture {
        binding: sampled_texture.texture_binding,
        visibility: sampled_texture.visibility,
    })
}

fn supported_sampled_texture_binding_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<SampledTextureBindGroupLayoutKind> {
    if entry.count.is_some()
        || entry.visibility.is_empty()
        || !(wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
            .contains(entry.visibility)
        || deko_texture_binding_mask(entry.binding, 1).is_none()
    {
        return None;
    }

    let supported = matches!(
        entry.ty,
        wgt::BindingType::Texture {
            sample_type: wgt::TextureSampleType::Float { .. }
                | wgt::TextureSampleType::Uint
                | wgt::TextureSampleType::Sint,
            view_dimension: wgt::TextureViewDimension::D1
                | wgt::TextureViewDimension::D2
                | wgt::TextureViewDimension::D3
                | wgt::TextureViewDimension::D2Array
                | wgt::TextureViewDimension::Cube
                | wgt::TextureViewDimension::CubeArray,
            multisampled: false,
        } | wgt::BindingType::Texture {
            sample_type: wgt::TextureSampleType::Depth,
            view_dimension: wgt::TextureViewDimension::D2 | wgt::TextureViewDimension::D2Array,
            multisampled: false,
        } | wgt::BindingType::Texture {
            sample_type: wgt::TextureSampleType::Float { .. }
                | wgt::TextureSampleType::Uint
                | wgt::TextureSampleType::Sint
                | wgt::TextureSampleType::Depth,
            view_dimension: wgt::TextureViewDimension::D2,
            multisampled: true,
        }
    );
    supported.then_some(SampledTextureBindGroupLayoutKind {
        texture_binding: entry.binding,
        visibility: entry.visibility,
    })
}

fn supported_buffer_sampled_texture_bind_group_layout_kind_group(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut buffer_entries = Vec::with_capacity(entries.len().saturating_sub(1));
    let mut sampled_textures = Vec::new();
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
        if let Some(kind) = supported_sampled_texture_binding_layout_kind(*entry) {
            sampled_textures.push(kind);
            continue;
        }
        return None;
    }

    if buffer_entries.is_empty() || sampled_textures.is_empty() {
        return None;
    }

    Some(BindGroupLayoutKind::BufferSampledTextureGroup {
        buffers: buffer_entries,
        sampled_textures,
    })
}

fn supported_pipeline_layout(
    bind_group_layouts: &[Option<&Resource>],
    _has_immediates: bool,
) -> bool {
    bind_group_layouts.len() <= crate::MAX_BIND_GROUPS
        && bind_group_layouts
            .iter()
            .flatten()
            .all(|layout| matches!(layout, Resource::BindGroupLayout(_)))
}

#[allow(dead_code)]
fn supported_legacy_pipeline_layout(
    bind_group_layouts: &[Option<&Resource>],
    has_immediates: bool,
) -> bool {
    if bind_group_layouts.len() > crate::MAX_BIND_GROUPS {
        return false;
    }

    let immediate_binding_mask = if has_immediates {
        1u32 << DEKO_IMMEDIATES_BINDING
    } else {
        0
    };
    let mut vertex_uniform_bindings = immediate_binding_mask;
    let mut fragment_uniform_bindings = immediate_binding_mask;
    let mut compute_uniform_bindings = immediate_binding_mask;
    let mut vertex_storage_bindings = 0u32;
    let mut fragment_storage_bindings = 0u32;
    let mut compute_storage_bindings = 0u32;
    let mut vertex_image_bindings = 0u32;
    let mut fragment_image_bindings = 0u32;
    let mut compute_image_bindings = 0u32;
    for layout in bind_group_layouts.iter().flatten() {
        match layout {
            Resource::BindGroupLayout(BindGroupLayoutKind::SampledTexture {
                binding,
                visibility,
            }) => {
                let Some(binding_mask) = deko_texture_binding_mask(*binding, 1) else {
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
            Resource::BindGroupLayout(BindGroupLayoutKind::TextureSampler {
                texture_binding,
                count,
                visibility,
                ..
            }) => {
                let Some(binding_mask) = deko_texture_binding_mask(*texture_binding, *count) else {
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
                count,
                visibility,
                ..
            }) => {
                let Some(binding_mask) =
                    deko_buffer_binding_mask(*binding, *count, DEKO_UNIFORM_BUFFER_COUNT)
                else {
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
                count,
                visibility,
                ..
            }) => {
                let Some(binding_mask) =
                    deko_buffer_binding_mask(*binding, *count, DEKO_STORAGE_BUFFER_COUNT)
                else {
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
                count,
                visibility,
                ..
            }) => {
                let Some(binding_mask) = deko_image_binding_mask(*binding, *count) else {
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
                    let Some(binding_mask) =
                        deko_buffer_binding_mask(binding, entry.count(), DEKO_UNIFORM_BUFFER_COUNT)
                    else {
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
            Resource::BindGroupLayout(BindGroupLayoutKind::BufferSampledTextureGroup {
                buffers,
                sampled_textures,
            }) => {
                for sampled_texture in sampled_textures {
                    let Some(binding_mask) =
                        deko_texture_binding_mask(sampled_texture.texture_binding, 1)
                    else {
                        return false;
                    };
                    if sampled_texture
                        .visibility
                        .contains(wgt::ShaderStages::VERTEX)
                    {
                        if vertex_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        vertex_image_bindings |= binding_mask;
                    }
                    if sampled_texture
                        .visibility
                        .contains(wgt::ShaderStages::FRAGMENT)
                    {
                        if fragment_image_bindings & binding_mask != 0 {
                            return false;
                        }
                        fragment_image_bindings |= binding_mask;
                    }
                    if sampled_texture
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
                    let Some(binding_mask) =
                        deko_buffer_binding_mask(binding, entry.count(), DEKO_UNIFORM_BUFFER_COUNT)
                    else {
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
                    let Some(binding_mask) = deko_texture_binding_mask(
                        texture_sampler.texture_binding,
                        texture_sampler.count,
                    ) else {
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
                    let Some(binding_mask) =
                        deko_buffer_binding_mask(binding, entry.count(), DEKO_UNIFORM_BUFFER_COUNT)
                    else {
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
                    let Some(binding_mask) =
                        deko_image_binding_mask(storage_texture.binding, storage_texture.count)
                    else {
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
                    let Some(binding_mask) =
                        deko_buffer_binding_mask(binding, entry.count(), DEKO_UNIFORM_BUFFER_COUNT)
                    else {
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
                    let Some(binding_mask) = deko_texture_binding_mask(
                        texture_sampler.texture_binding,
                        texture_sampler.count,
                    ) else {
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
                    let Some(binding_mask) =
                        deko_image_binding_mask(storage_texture.binding, storage_texture.count)
                    else {
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
                    let Some(binding_mask) =
                        deko_buffer_binding_mask(binding, entry.count(), DEKO_UNIFORM_BUFFER_COUNT)
                    else {
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
        if entry.visibility.is_empty()
            || !(wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility)
        {
            return None;
        }
        let count = entry.count.map(|count| count.get()).unwrap_or(1);
        match (entry.binding, entry.ty) {
            (
                binding,
                wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Float { .. },
                    view_dimension:
                        wgt::TextureViewDimension::D1
                        | wgt::TextureViewDimension::D2
                        | wgt::TextureViewDimension::D3
                        | wgt::TextureViewDimension::D2Array
                        | wgt::TextureViewDimension::Cube
                        | wgt::TextureViewDimension::CubeArray,
                    multisampled: false,
                },
            ) if deko_texture_binding_mask(binding, count).is_some() => {
                texture_binding = Some((binding, count, false));
                visibility |= entry.visibility;
            }
            (
                binding,
                wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Depth,
                    view_dimension:
                        wgt::TextureViewDimension::D2
                        | wgt::TextureViewDimension::D2Array
                        | wgt::TextureViewDimension::Cube
                        | wgt::TextureViewDimension::CubeArray,
                    multisampled: false,
                },
            ) if deko_texture_binding_mask(binding, count).is_some() => {
                texture_binding = Some((binding, count, true));
                visibility |= entry.visibility;
            }
            (
                binding,
                wgt::BindingType::Sampler(
                    wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
                ),
            ) => {
                sampler_binding = Some((binding, count, false));
                visibility |= entry.visibility;
            }
            (binding, wgt::BindingType::Sampler(wgt::SamplerBindingType::Comparison)) => {
                sampler_binding = Some((binding, count, true));
                visibility |= entry.visibility;
            }
            _ => return None,
        }
    }
    let (texture_binding, texture_count, texture_is_depth) = texture_binding?;
    let (sampler_binding, sampler_count, sampler_is_comparison) = sampler_binding?;
    if texture_count != sampler_count || texture_is_depth != sampler_is_comparison {
        return None;
    }
    Some(BindGroupLayoutKind::TextureSampler {
        texture_binding,
        sampler_binding,
        count: texture_count,
        visibility,
    })
}

fn supported_storage_texture_bind_group_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<BindGroupLayoutKind> {
    let entry = supported_storage_texture_binding_layout_kind(entry)?;
    Some(BindGroupLayoutKind::StorageTexture {
        binding: entry.binding,
        count: entry.count,
        visibility: entry.visibility,
        access: entry.access,
        format: entry.format,
    })
}

fn supported_storage_texture_binding_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<StorageTextureBindGroupLayoutKind> {
    let count = entry.count.map(|count| count.get()).unwrap_or(1);
    if entry.visibility.is_empty() || deko_image_binding_mask(entry.binding, count).is_none() {
        return None;
    }

    match entry.ty {
        wgt::BindingType::StorageTexture {
            access,
            format,
            view_dimension,
        } if matches!(
            format,
            wgt::TextureFormat::R8Unorm
                | wgt::TextureFormat::Rg8Unorm
                | wgt::TextureFormat::Rgba8Unorm
                | wgt::TextureFormat::Rgba16Float
        ) && matches!(
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
                count,
                visibility: entry.visibility,
                access,
                format,
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

        if entry.visibility.is_empty()
            || !(wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility)
        {
            return None;
        }

        let count = entry.count.map(|count| count.get()).unwrap_or(1);
        match entry.ty {
            wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { .. },
                view_dimension:
                    wgt::TextureViewDimension::D1
                    | wgt::TextureViewDimension::D2
                    | wgt::TextureViewDimension::D3
                    | wgt::TextureViewDimension::D2Array
                    | wgt::TextureViewDimension::Cube
                    | wgt::TextureViewDimension::CubeArray,
                multisampled: false,
            } if deko_texture_binding_mask(entry.binding, count).is_some() => {
                textures.push((entry.binding, count, entry.visibility));
            }
            wgt::BindingType::Sampler(
                wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
            ) => {
                samplers.push((entry.binding, count, entry.visibility));
            }
            _ => return None,
        }
    }

    if textures.is_empty() || textures.len() != samplers.len() || storage_textures.is_empty() {
        return None;
    }

    let mut texture_samplers = Vec::with_capacity(textures.len());
    for (texture_binding, texture_count, texture_visibility) in textures {
        let sampler_binding = texture_binding.checked_add(1)?;
        let sampler_index = samplers.iter().position(|(binding, count, _)| {
            *binding == sampler_binding && *count == texture_count
        })?;
        let (_, _, sampler_visibility) = samplers.swap_remove(sampler_index);
        texture_samplers.push(TextureSamplerBindGroupLayoutKind {
            texture_binding,
            sampler_binding,
            count: texture_count,
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

        if entry.visibility.is_empty()
            || !(wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility)
        {
            return None;
        }

        let count = entry.count.map(|count| count.get()).unwrap_or(1);
        match entry.ty {
            wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { .. },
                view_dimension:
                    wgt::TextureViewDimension::D1
                    | wgt::TextureViewDimension::D2
                    | wgt::TextureViewDimension::D3
                    | wgt::TextureViewDimension::D2Array
                    | wgt::TextureViewDimension::Cube
                    | wgt::TextureViewDimension::CubeArray,
                multisampled: false,
            } if deko_texture_binding_mask(entry.binding, count).is_some() => {
                textures.push((entry.binding, count, entry.visibility));
            }
            wgt::BindingType::Sampler(
                wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
            ) => {
                samplers.push((entry.binding, count, entry.visibility));
            }
            _ => return None,
        }
    }

    if textures.is_empty() || textures.len() != samplers.len() {
        return None;
    }

    let mut texture_samplers = Vec::with_capacity(textures.len());
    for (texture_binding, texture_count, texture_visibility) in textures {
        let sampler_binding = texture_binding.checked_add(1)?;
        let sampler_index = samplers.iter().position(|(binding, count, _)| {
            *binding == sampler_binding && *count == texture_count
        })?;
        let (_, _, sampler_visibility) = samplers.swap_remove(sampler_index);
        texture_samplers.push(TextureSamplerBindGroupLayoutKind {
            texture_binding,
            sampler_binding,
            count: texture_count,
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
            count,
            visibility,
            has_dynamic_offset,
        } => Some(BindGroupLayoutKind::UniformBuffer {
            binding,
            count,
            visibility,
            has_dynamic_offset,
        }),
        BufferBindGroupLayoutKind::Storage {
            binding,
            count,
            visibility,
            read_only,
            has_dynamic_offset,
        } => Some(BindGroupLayoutKind::StorageBuffer {
            binding,
            count,
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
    if entry.visibility.is_empty() {
        return None;
    }
    let count = entry.count.map_or(1, core::num::NonZeroU32::get);

    match entry.ty {
        wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Uniform,
            has_dynamic_offset,
            min_binding_size,
        } if (count == 1 || !has_dynamic_offset)
            && deko_buffer_binding_mask(entry.binding, count, DEKO_UNIFORM_BUFFER_COUNT)
                .is_some()
            && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility) =>
        {
            if min_binding_size.is_some_and(|size| size.get() > DEKO_UNIFORM_BUF_MAX_SIZE) {
                return None;
            }
            Some(BufferBindGroupLayoutKind::Uniform {
                binding: entry.binding,
                count,
                visibility: entry.visibility,
                has_dynamic_offset,
            })
        }
        wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Storage { read_only },
            has_dynamic_offset,
            min_binding_size: _,
        } if (count == 1 || !has_dynamic_offset)
            && deko_buffer_binding_mask(entry.binding, count, DEKO_STORAGE_BUFFER_COUNT)
                .is_some()
            && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility) =>
        {
            Some(BufferBindGroupLayoutKind::Storage {
                binding: entry.binding,
                count,
                visibility: entry.visibility,
                read_only,
                has_dynamic_offset,
            })
        }
        _ => None,
    }
}

fn shader_error(message: &'static str) -> crate::ShaderError {
    crate::ShaderError::Compilation(String::from(message))
}

fn validate_deko3d_shader_metadata(
    metadata: &wgt::Deko3dShaderMetadata<'_>,
) -> Result<Deko3dShaderMetadata, crate::ShaderError> {
    if !matches!(
        metadata.stage,
        wgt::ShaderStages::VERTEX | wgt::ShaderStages::FRAGMENT | wgt::ShaderStages::COMPUTE
    ) || metadata.entry_point.is_empty()
    {
        return Err(shader_error(
            "deko3d shader metadata has an invalid stage or entry point",
        ));
    }

    for (index, binding) in metadata.bindings.iter().enumerate() {
        let limit = match binding.kind {
            wgt::Deko3dShaderBindingKind::UniformBuffer => DEKO_UNIFORM_BUFFER_COUNT,
            wgt::Deko3dShaderBindingKind::StorageBuffer => DEKO_STORAGE_BUFFER_COUNT,
            wgt::Deko3dShaderBindingKind::SampledTexture
            | wgt::Deko3dShaderBindingKind::Sampler => DEKO_TEXTURE_BINDING_COUNT,
            wgt::Deko3dShaderBindingKind::StorageTexture => DEKO_IMAGE_BINDING_COUNT,
        };
        let Some(end) = binding.physical_binding.checked_add(binding.count) else {
            return Err(shader_error("deko3d shader binding range overflows"));
        };
        if binding.count == 0 || end > limit {
            return Err(shader_error(
                "deko3d shader binding exceeds the physical slot limit",
            ));
        }
        if binding.kind == wgt::Deko3dShaderBindingKind::UniformBuffer
            && end > DEKO_IMMEDIATES_BINDING
        {
            return Err(shader_error(
                "deko3d shader binding overlaps the immediates slot",
            ));
        }
        for previous in &metadata.bindings[..index] {
            if previous.group == binding.group && previous.binding == binding.binding {
                return Err(shader_error(
                    "deko3d shader metadata repeats a logical binding",
                ));
            }
            if previous.kind == binding.kind {
                let previous_end = previous.physical_binding + previous.count;
                if binding.physical_binding < previous_end && previous.physical_binding < end {
                    return Err(shader_error(
                        "deko3d shader metadata overlaps physical bindings",
                    ));
                }
            }
        }
    }

    Ok(Deko3dShaderMetadata {
        stage: metadata.stage,
        entry_point: String::from(metadata.entry_point.as_ref()),
        bindings: metadata.bindings.to_vec(),
    })
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
        let clear_buffer = match Buffer::new(
            &crate::BufferDescriptor {
                label: Some("Deko3D zero-fill buffer"),
                size: DEKO_CLEAR_BUFFER_SIZE,
                usage: wgt::BufferUses::COPY_SRC,
                memory_flags: crate::MemoryFlags::empty(),
            },
            raw_device,
        ) {
            Ok(buffer) => buffer,
            Err(error) => {
                drop(texture_descriptor_heap);
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
            clear_buffer: ManuallyDrop::new(clear_buffer),
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
/// Pipeline caches remain disabled because Deko3D DKSH modules are already
/// compiled and the public API exposes no reusable native cache data.
pub fn supported_features() -> wgt::Features {
    wgt::Features::PASSTHROUGH_SHADERS
        | wgt::Features::MAPPABLE_PRIMARY_BUFFERS
        | wgt::Features::IMMEDIATES
        | wgt::Features::TEXTURE_BINDING_ARRAY
        | wgt::Features::BUFFER_BINDING_ARRAY
        | wgt::Features::STORAGE_RESOURCE_BINDING_ARRAY
        | wgt::Features::TIMESTAMP_QUERY
        | wgt::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
        | wgt::Features::TIMESTAMP_QUERY_INSIDE_PASSES
        | wgt::Features::TEXTURE_FORMAT_16BIT_NORM
        | wgt::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
        | wgt::Features::ADDRESS_MODE_CLAMP_TO_BORDER
        | wgt::Features::ADDRESS_MODE_CLAMP_TO_ZERO
        | wgt::Features::DEPTH32FLOAT_STENCIL8
}

/// Conservative capabilities for the first Deko3D adapter slice.
///
/// These are intentionally below the real hardware envelope until each resource
/// path is implemented and validated.
pub fn capabilities() -> crate::Capabilities {
    crate::Capabilities {
        limits: wgt::Limits {
            max_color_attachments: DEKO_COLOR_ATTACHMENT_COUNT,
            max_immediate_size: DEKO_UNIFORM_BUF_MAX_SIZE as u32,
            max_sampled_textures_per_shader_stage: DEKO_TEXTURE_BINDING_COUNT,
            max_samplers_per_shader_stage: DEKO_TEXTURE_BINDING_COUNT,
            max_binding_array_elements_per_shader_stage: DEKO_TEXTURE_BINDING_COUNT,
            max_binding_array_sampler_elements_per_shader_stage: DEKO_TEXTURE_BINDING_COUNT,
            ..wgt::Limits::downlevel_defaults()
        },
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
            flags: wgt::DownlevelFlags::INDIRECT_EXECUTION
                | wgt::DownlevelFlags::ANISOTROPIC_FILTERING,
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
            let mut state = self.state();
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
        let mut state = self.state();
        *state = None;
    }

    unsafe fn acquire_texture(
        &self,
        _timeout: Option<Duration>,
        _fence: &Fence,
    ) -> Result<crate::AcquiredSurfaceTexture<Api>, crate::SurfaceError> {
        #[cfg(target_os = "horizon")]
        {
            let mut state = self.state();
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
            if let Ok(mut state) = self.configured_state() {
                let state = state.as_mut().unwrap();
                if matches!(
                    texture,
                    Resource::SurfaceTexture { queue, .. }
                        if queue.0 == state.inner.render_queue
                ) {
                    state.inner.acquired = false;
                }
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
            let mut surface_queue = None;
            for texture in surface_textures {
                let Resource::SurfaceTexture { queue, .. } = texture else {
                    return Err(crate::DeviceError::Lost);
                };
                if surface_queue.is_some_and(|existing: RawQueueHandle| existing.0 != queue.0) {
                    return Err(crate::DeviceError::Lost);
                }
                surface_queue = Some(*queue);
            }
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
            let mut state = surface.configured_state()?;
            let state = state.as_mut().unwrap();
            let Resource::SurfaceTexture { slot, queue, .. } = texture else {
                return Err(crate::SurfaceError::Other(
                    "deko3d present requires a surface texture",
                ));
            };
            if queue.0 != state.inner.render_queue {
                return Err(crate::SurfaceError::Other(
                    "deko3d surface texture belongs to another surface",
                ));
            }
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
        #[cfg(target_os = "horizon")]
        unsafe {
            dk::dkQueueWaitIdle(self.raw.0);
        }
        Ok(())
    }

    unsafe fn get_timestamp_period(&self) -> f32 {
        #[cfg(target_os = "horizon")]
        {
            return 625.0 / 384.0;
        }
        #[cfg(not(target_os = "horizon"))]
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
                format: desc.format,
                aspect: wgt::TextureAspect::All,
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
                    wgt::TextureViewDimension::D1
                        | wgt::TextureViewDimension::D2
                        | wgt::TextureViewDimension::D3
                        | wgt::TextureViewDimension::D2Array
                        | wgt::TextureViewDimension::Cube
                        | wgt::TextureViewDimension::CubeArray
                ) || (matches!(
                    desc.dimension,
                    wgt::TextureViewDimension::D1 | wgt::TextureViewDimension::D2
                ) && array_layer_count != 1)
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
                    format: desc.format,
                    aspect: desc.range.aspect,
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
        if !supported_pipeline_layout(desc.bind_group_layouts, desc.immediate_size != 0) {
            return Err(crate::DeviceError::Lost);
        }
        #[cfg(target_os = "horizon")]
        let layout = PipelineLayoutInner::new(desc, self.inner.raw_device())?;
        #[cfg(not(target_os = "horizon"))]
        let layout = PipelineLayoutInner::new(desc)?;
        Ok(Resource::PipelineLayout(Arc::new(layout)))
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
                crate::ShaderInput::Deko3dDksh { bytes, metadata } => {
                    Ok(Resource::ShaderModule(Arc::new(unsafe {
                        ShaderModuleInner::new_dksh(self.inner.raw_device(), bytes, metadata)?
                    })))
                }
                crate::ShaderInput::Deko3dArtifacts(artifacts) => {
                    Ok(Resource::ShaderModule(Arc::new(unsafe {
                        ShaderModuleInner::new_artifacts(self.inner.raw_device(), artifacts)?
                    })))
                }
                _ => Err(crate::ShaderError::Compilation(String::from(
                    "deko3d requires precompiled DKSH: public Deko3D only initializes DKSH with \
                     dkShaderInitialize and exposes no runtime WGSL, Naga IR, GLSL, or SPIR-V compiler; \
                     use create_shader_module_deko3d_dksh with a verified WGSL-to-DKSH artifact",
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
        Err(crate::PipelineCacheError::Device(crate::DeviceError::Lost))
    }
    unsafe fn destroy_pipeline_cache(&self, _cache: Resource) {}

    unsafe fn create_query_set(
        &self,
        desc: &wgt::QuerySetDescriptor<crate::Label>,
    ) -> DeviceResult<Resource> {
        #[cfg(target_os = "horizon")]
        {
            Ok(Resource::QuerySet(Arc::new(QuerySetInner::new(
                self.inner.raw_device(),
                desc,
            )?)))
        }
        #[cfg(not(target_os = "horizon"))]
        {
            let _ = desc;
            Err(crate::DeviceError::Lost)
        }
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
    use core::num::NonZeroU32;

    fn valid_dksh_bytes() -> Vec<u8> {
        let header = DkshHeader {
            magic: DKSH_MAGIC,
            header_sz: size_of::<DkshHeader>() as u32,
            control_sz: 256,
            code_sz: 256,
            programs_off: size_of::<DkshHeader>() as u32,
            num_programs: 1,
        };
        let mut bytes = vec![0; (header.control_sz + header.code_sz) as usize];
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                core::ptr::addr_of!(header).cast::<u8>(),
                size_of::<DkshHeader>(),
            )
        };
        bytes[..header_bytes.len()].copy_from_slice(header_bytes);
        bytes
    }

    #[test]
    fn dksh_headers_require_a_valid_container_layout() {
        assert!(validate_dksh_header(&valid_dksh_bytes()).is_ok());

        let mut invalid_magic = valid_dksh_bytes();
        invalid_magic[0] = 0;
        assert!(validate_dksh_header(&invalid_magic).is_err());

        let mut invalid_program_offset = valid_dksh_bytes();
        invalid_program_offset[16..20].copy_from_slice(&256u32.to_ne_bytes());
        assert!(validate_dksh_header(&invalid_program_offset).is_err());

        assert!(validate_dksh_header(&valid_dksh_bytes()[..255]).is_err());
    }

    #[test]
    fn texture_descriptor_slots_are_reclaimed_and_reused() {
        let allocator = TextureDescriptorAllocator {
            locked: AtomicBool::new(false),
            used: UnsafeCell::new([false; DEKO_TEXTURE_DESCRIPTOR_COUNT as usize]),
        };

        let first = allocator.allocate(128).unwrap();
        let second = allocator.allocate(128).unwrap();
        assert_eq!((first, second), (0, 128));
        assert!(allocator.allocate(1).is_err());

        allocator.release(first, 128);
        assert_eq!(allocator.allocate(64).unwrap(), 0);
        allocator.release(0, 64);
        allocator.release(second, 128);

        for _ in 0..1024 {
            let slot = allocator.allocate(1).unwrap();
            assert_eq!(slot, 0);
            allocator.release(slot, 1);
        }
    }

    #[test]
    fn validates_reflected_deko3d_binding_assignments() {
        let bindings = [
            wgt::Deko3dShaderBinding {
                group: 0,
                binding: 0,
                kind: wgt::Deko3dShaderBindingKind::UniformBuffer,
                count: 1,
                physical_binding: 0,
            },
            wgt::Deko3dShaderBinding {
                group: 1,
                binding: 0,
                kind: wgt::Deko3dShaderBindingKind::UniformBuffer,
                count: 1,
                physical_binding: 1,
            },
        ];
        let metadata = wgt::Deko3dShaderMetadata {
            stage: wgt::ShaderStages::VERTEX,
            entry_point: "main".into(),
            bindings: bindings.as_slice().into(),
        };
        let validated = validate_deko3d_shader_metadata(&metadata).unwrap();
        assert_eq!(validated.stage, wgt::ShaderStages::VERTEX);
        assert_eq!(validated.entry_point, "main");
        assert_eq!(validated.bindings, bindings);

        let mut overlapping = bindings;
        overlapping[1].physical_binding = 0;
        let metadata = wgt::Deko3dShaderMetadata {
            bindings: overlapping.as_slice().into(),
            ..metadata
        };
        assert!(validate_deko3d_shader_metadata(&metadata).is_err());
    }

    #[cfg(not(target_os = "horizon"))]
    #[test]
    fn shader_artifact_bundle_selects_stage_and_entry_point() {
        let artifact = |stage, entry_point: &str| {
            Arc::new(ShaderArtifactInner {
                inner: ShaderModuleInnerRaw,
                metadata: Some(Deko3dShaderMetadata {
                    stage,
                    entry_point: entry_point.into(),
                    bindings: Vec::new(),
                }),
            })
        };
        let vertex = artifact(wgt::ShaderStages::VERTEX, "main");
        let fragment = artifact(wgt::ShaderStages::FRAGMENT, "main");
        let module = ShaderModuleInner {
            artifacts: vec![vertex.clone(), fragment.clone()],
        };

        assert!(Arc::ptr_eq(
            &module.select(wgt::ShaderStages::VERTEX, "main").unwrap(),
            &vertex
        ));
        assert!(Arc::ptr_eq(
            &module.select(wgt::ShaderStages::FRAGMENT, "main").unwrap(),
            &fragment
        ));
        assert!(module.select(wgt::ShaderStages::COMPUTE, "main").is_err());
    }

    #[test]
    fn vertex_step_modes_map_to_native_divisors() {
        assert_eq!(vertex_step_divisor(wgt::VertexStepMode::Vertex), 0);
        assert_eq!(vertex_step_divisor(wgt::VertexStepMode::Instance), 1);
    }

    #[test]
    fn advertises_native_anisotropic_filtering() {
        assert!(capabilities()
            .downlevel
            .flags
            .contains(wgt::DownlevelFlags::ANISOTROPIC_FILTERING));
    }

    #[test]
    fn rejects_reflection_that_uses_the_immediates_slot() {
        let bindings = [wgt::Deko3dShaderBinding {
            group: 0,
            binding: 0,
            kind: wgt::Deko3dShaderBindingKind::UniformBuffer,
            count: 1,
            physical_binding: DEKO_IMMEDIATES_BINDING,
        }];
        let metadata = wgt::Deko3dShaderMetadata {
            stage: wgt::ShaderStages::COMPUTE,
            entry_point: "main".into(),
            bindings: bindings.as_slice().into(),
        };
        assert!(validate_deko3d_shader_metadata(&metadata).is_err());
    }

    #[test]
    fn reflected_bindings_remap_same_logical_binding_across_groups() {
        let map = Deko3dPipelineBindingMap {
            reflected_stages: wgt::ShaderStages::COMPUTE,
            bindings: vec![
                Deko3dPipelineBinding {
                    stage: wgt::ShaderStages::COMPUTE,
                    binding: wgt::Deko3dShaderBinding {
                        group: 0,
                        binding: 0,
                        kind: wgt::Deko3dShaderBindingKind::UniformBuffer,
                        count: 1,
                        physical_binding: 3,
                    },
                },
                Deko3dPipelineBinding {
                    stage: wgt::ShaderStages::COMPUTE,
                    binding: wgt::Deko3dShaderBinding {
                        group: 1,
                        binding: 0,
                        kind: wgt::Deko3dShaderBindingKind::UniformBuffer,
                        count: 1,
                        physical_binding: 7,
                    },
                },
            ],
        };

        assert_eq!(
            map.physical_binding(
                wgt::ShaderStages::COMPUTE,
                0,
                0,
                wgt::Deko3dShaderBindingKind::UniformBuffer,
            ),
            Some(3)
        );
        assert_eq!(
            map.physical_binding(
                wgt::ShaderStages::COMPUTE,
                1,
                0,
                wgt::Deko3dShaderBindingKind::UniformBuffer,
            ),
            Some(7)
        );
        assert_eq!(
            map.physical_binding(
                wgt::ShaderStages::COMPUTE,
                2,
                0,
                wgt::Deko3dShaderBindingKind::UniformBuffer,
            ),
            None
        );

        let legacy = Deko3dPipelineBindingMap::default();
        assert_eq!(
            legacy.physical_binding(
                wgt::ShaderStages::VERTEX,
                0,
                4,
                wgt::Deko3dShaderBindingKind::UniformBuffer,
            ),
            Some(4)
        );
        assert_eq!(
            legacy.physical_binding(
                wgt::ShaderStages::VERTEX,
                1,
                4,
                wgt::Deko3dShaderBindingKind::UniformBuffer,
            ),
            None
        );
    }

    #[test]
    fn gates_noop_pipeline_cache_support() {
        assert!(!supported_features().contains(wgt::Features::PIPELINE_CACHE));
    }

    #[test]
    fn does_not_advertise_gpu_written_indirect_counts() {
        assert!(!supported_features().contains(wgt::Features::MULTI_DRAW_INDIRECT_COUNT));
    }

    #[test]
    fn advertises_immediate_data_support() {
        assert!(supported_features().contains(wgt::Features::IMMEDIATES));
        assert_eq!(
            capabilities().limits.max_immediate_size,
            DEKO_UNIFORM_BUF_MAX_SIZE as u32
        );
    }

    #[test]
    fn pipeline_layout_defers_immediate_slot_validation_to_shader_mapping() {
        let layout = Resource::BindGroupLayout(BindGroupLayoutKind::UniformBuffer {
            binding: DEKO_IMMEDIATES_BINDING,
            count: 1,
            visibility: wgt::ShaderStages::COMPUTE,
            has_dynamic_offset: false,
        });
        assert!(supported_pipeline_layout(&[Some(&layout)], false));
        assert!(supported_pipeline_layout(&[Some(&layout)], true));
    }

    #[test]
    fn pipeline_layout_accepts_repeated_logical_bindings_across_groups() {
        let layout = Resource::BindGroupLayout(BindGroupLayoutKind::UniformBuffer {
            binding: 0,
            count: 1,
            visibility: wgt::ShaderStages::COMPUTE,
            has_dynamic_offset: false,
        });
        assert!(supported_pipeline_layout(
            &[Some(&layout), Some(&layout)],
            false,
        ));
    }

    #[test]
    fn recognizes_one_dimensional_sampled_texture_layouts() {
        let entry = wgt::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgt::ShaderStages::COMPUTE,
            ty: wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { filterable: true },
                view_dimension: wgt::TextureViewDimension::D1,
                multisampled: false,
            },
            count: None,
        };

        assert!(matches!(
            supported_bind_group_layout_kind(&[entry]),
            Some(BindGroupLayoutKind::SampledTexture { .. })
        ));
        assert!(texture_view_dimension_matches(
            wgt::TextureDimension::D1,
            wgt::TextureViewDimension::D1,
        ));
    }

    #[test]
    fn advertises_clamp_to_border_sampler_support() {
        let features = supported_features();
        assert!(features.contains(wgt::Features::ADDRESS_MODE_CLAMP_TO_BORDER));
        assert!(features.contains(wgt::Features::ADDRESS_MODE_CLAMP_TO_ZERO));
    }

    #[test]
    fn advertises_paired_sampled_texture_binding_arrays() {
        assert!(supported_features().contains(wgt::Features::TEXTURE_BINDING_ARRAY));
        assert_eq!(
            capabilities()
                .limits
                .max_binding_array_elements_per_shader_stage,
            DEKO_TEXTURE_BINDING_COUNT
        );
        assert_eq!(
            capabilities()
                .limits
                .max_binding_array_sampler_elements_per_shader_stage,
            DEKO_TEXTURE_BINDING_COUNT
        );
    }

    #[test]
    fn advertises_storage_texture_binding_arrays() {
        let features = supported_features();
        assert!(features.contains(wgt::Features::TEXTURE_BINDING_ARRAY));
        assert!(features.contains(wgt::Features::STORAGE_RESOURCE_BINDING_ARRAY));
    }

    #[test]
    fn advertises_storage_buffer_binding_arrays() {
        let entry = wgt::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgt::ShaderStages::COMPUTE,
            ty: wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: NonZeroU32::new(2),
        };
        let layout = supported_bind_group_layout_kind(&[entry]).unwrap();
        assert!(supported_features().contains(wgt::Features::BUFFER_BINDING_ARRAY));
        assert!(matches!(
            layout,
            BindGroupLayoutKind::StorageBuffer { count: 2, .. }
        ));
        let resource = Resource::BindGroupLayout(layout);
        assert!(supported_pipeline_layout(&[Some(&resource)], false));

        let out_of_range = wgt::BindGroupLayoutEntry {
            binding: DEKO_STORAGE_BUFFER_COUNT - 1,
            ..entry
        };
        assert!(supported_bind_group_layout_kind(&[out_of_range]).is_none());

        let dynamic = wgt::BindGroupLayoutEntry {
            ty: wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            ..entry
        };
        assert!(supported_bind_group_layout_kind(&[dynamic]).is_none());
    }

    #[test]
    fn advertises_uniform_buffer_binding_arrays() {
        let entry = wgt::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgt::ShaderStages::VERTEX_FRAGMENT,
            ty: wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: NonZeroU32::new(3),
        };
        assert!(matches!(
            supported_bind_group_layout_kind(&[entry]),
            Some(BindGroupLayoutKind::UniformBuffer { count: 3, .. })
        ));
        assert!(supported_features().contains(wgt::Features::BUFFER_BINDING_ARRAY));

        let dynamic = wgt::BindGroupLayoutEntry {
            ty: wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            ..entry
        };
        assert!(supported_bind_group_layout_kind(&[dynamic]).is_none());
    }

    #[test]
    fn advertises_native_color_attachment_count() {
        assert_eq!(
            capabilities().limits.max_color_attachments,
            DEKO_COLOR_ATTACHMENT_COUNT
        );
    }

    #[test]
    fn recognizes_storage_texture_binding_arrays() {
        let storage_texture = wgt::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgt::ShaderStages::COMPUTE,
            ty: wgt::BindingType::StorageTexture {
                access: wgt::StorageTextureAccess::ReadWrite,
                format: wgt::TextureFormat::Rgba8Unorm,
                view_dimension: wgt::TextureViewDimension::D2,
            },
            count: Some(NonZeroU32::new(2).unwrap()),
        };

        assert!(matches!(
            supported_bind_group_layout_kind(&[storage_texture]),
            Some(BindGroupLayoutKind::StorageTexture {
                binding: 0,
                count: 2,
                visibility: wgt::ShaderStages::COMPUTE,
                access: wgt::StorageTextureAccess::ReadWrite,
                format: wgt::TextureFormat::Rgba8Unorm,
            })
        ));
        assert_eq!(deko_image_binding_mask(7, 2), None);
    }

    #[test]
    fn recognizes_r8_and_rg8_storage_texture_layouts() {
        for format in [wgt::TextureFormat::R8Unorm, wgt::TextureFormat::Rg8Unorm] {
            for access in [
                wgt::StorageTextureAccess::ReadOnly,
                wgt::StorageTextureAccess::WriteOnly,
                wgt::StorageTextureAccess::ReadWrite,
            ] {
                for view_dimension in [
                    wgt::TextureViewDimension::D2,
                    wgt::TextureViewDimension::D2Array,
                    wgt::TextureViewDimension::D3,
                ] {
                    for count in [None, Some(NonZeroU32::new(2).unwrap())] {
                        let entry = wgt::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgt::ShaderStages::COMPUTE,
                            ty: wgt::BindingType::StorageTexture {
                                access,
                                format,
                                view_dimension,
                            },
                            count,
                        };
                        assert!(matches!(
                            supported_bind_group_layout_kind(&[entry]),
                            Some(BindGroupLayoutKind::StorageTexture {
                                format: actual,
                                access: actual_access,
                                count: actual_count,
                                ..
                            }) if actual == format
                                && actual_access == access
                                && actual_count == count.map_or(1, NonZeroU32::get)
                        ));
                    }
                }
            }
        }
    }

    #[test]
    fn recognizes_mixed_buffer_and_storage_texture_binding_arrays() {
        let entries = [
            wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::StorageTexture {
                    access: wgt::StorageTextureAccess::ReadWrite,
                    format: wgt::TextureFormat::Rgba8Unorm,
                    view_dimension: wgt::TextureViewDimension::D2,
                },
                count: Some(NonZeroU32::new(2).unwrap()),
            },
            wgt::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Buffer {
                    ty: wgt::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        let Some(BindGroupLayoutKind::BufferStorageTextureGroup {
            buffers,
            storage_textures,
        }) = supported_bind_group_layout_kind(&entries)
        else {
            panic!("expected a mixed storage texture array layout");
        };
        assert_eq!(buffers.len(), 1);
        assert_eq!(storage_textures.len(), 1);
        assert_eq!(storage_textures[0].binding, 0);
        assert_eq!(storage_textures[0].count, 2);
    }

    #[test]
    fn recognizes_mixed_sampled_texture_arrays_and_storage_textures() {
        let entries = [
            wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Float { filterable: true },
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: Some(NonZeroU32::new(2).unwrap()),
            },
            wgt::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Filtering),
                count: Some(NonZeroU32::new(2).unwrap()),
            },
            wgt::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::StorageTexture {
                    access: wgt::StorageTextureAccess::WriteOnly,
                    format: wgt::TextureFormat::Rgba8Unorm,
                    view_dimension: wgt::TextureViewDimension::D2,
                },
                count: None,
            },
            wgt::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Buffer {
                    ty: wgt::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: true,
                    min_binding_size: wgt::BufferSize::new(4),
                },
                count: None,
            },
        ];

        let Some(BindGroupLayoutKind::BufferTextureSamplerStorageTextureGroup {
            buffers,
            texture_samplers,
            storage_textures,
        }) = supported_bind_group_layout_kind(&entries)
        else {
            panic!("expected a mixed sampled texture array and storage texture layout");
        };
        assert_eq!(buffers.len(), 1);
        assert!(matches!(
            buffers[0],
            BufferBindGroupLayoutKind::Storage {
                binding: 3,
                read_only: false,
                has_dynamic_offset: true,
                ..
            }
        ));
        assert_eq!(texture_samplers.len(), 1);
        assert_eq!(texture_samplers[0].texture_binding, 0);
        assert_eq!(texture_samplers[0].sampler_binding, 1);
        assert_eq!(texture_samplers[0].count, 2);
        assert_eq!(storage_textures.len(), 1);
        assert_eq!(storage_textures[0].binding, 2);
    }

    #[test]
    fn recognizes_paired_sampled_texture_binding_arrays() {
        let entries = [
            wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Float { filterable: true },
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: Some(NonZeroU32::new(2).unwrap()),
            },
            wgt::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Filtering),
                count: Some(NonZeroU32::new(2).unwrap()),
            },
        ];

        assert!(matches!(
            supported_texture_bind_group_layout_kind(&entries),
            Some(BindGroupLayoutKind::TextureSampler {
                texture_binding: 0,
                sampler_binding: 1,
                count: 2,
                visibility: wgt::ShaderStages::COMPUTE,
            })
        ));
        assert_eq!(deko_texture_binding_mask(31, 2), None);
    }

    #[test]
    fn recognizes_mixed_buffer_and_sampled_texture_binding_arrays() {
        let entries = [
            wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Float { filterable: true },
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: Some(NonZeroU32::new(2).unwrap()),
            },
            wgt::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Filtering),
                count: Some(NonZeroU32::new(2).unwrap()),
            },
            wgt::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Buffer {
                    ty: wgt::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        let Some(BindGroupLayoutKind::BufferTextureSamplerGroup {
            buffers,
            texture_samplers,
        }) = supported_buffer_texture_sampler_bind_group_layout_kind_group(&entries)
        else {
            panic!("expected a mixed buffer and sampled texture array layout");
        };
        assert_eq!(buffers.len(), 1);
        assert_eq!(texture_samplers.len(), 1);
        assert_eq!(texture_samplers[0].texture_binding, 0);
        assert_eq!(texture_samplers[0].sampler_binding, 1);
        assert_eq!(texture_samplers[0].count, 2);
    }

    #[test]
    fn recognizes_depth_comparison_sampler_layout() {
        for view_dimension in [
            wgt::TextureViewDimension::D2,
            wgt::TextureViewDimension::D2Array,
            wgt::TextureViewDimension::Cube,
            wgt::TextureViewDimension::CubeArray,
        ] {
            let entries = [
                wgt::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgt::ShaderStages::FRAGMENT,
                    ty: wgt::BindingType::Texture {
                        sample_type: wgt::TextureSampleType::Depth,
                        view_dimension,
                        multisampled: false,
                    },
                    count: None,
                },
                wgt::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgt::ShaderStages::FRAGMENT,
                    ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Comparison),
                    count: None,
                },
            ];

            assert!(matches!(
                supported_texture_bind_group_layout_kind(&entries),
                Some(BindGroupLayoutKind::TextureSampler {
                    texture_binding: 4,
                    sampler_binding: 5,
                    count: 1,
                    visibility: wgt::ShaderStages::FRAGMENT,
                })
            ));
        }
    }

    #[test]
    fn recognizes_depth_comparison_sampler_binding_arrays() {
        for view_dimension in [
            wgt::TextureViewDimension::D2,
            wgt::TextureViewDimension::D2Array,
            wgt::TextureViewDimension::Cube,
            wgt::TextureViewDimension::CubeArray,
        ] {
            let entries = [
                wgt::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgt::ShaderStages::FRAGMENT,
                    ty: wgt::BindingType::Texture {
                        sample_type: wgt::TextureSampleType::Depth,
                        view_dimension,
                        multisampled: false,
                    },
                    count: Some(NonZeroU32::new(2).unwrap()),
                },
                wgt::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgt::ShaderStages::FRAGMENT,
                    ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Comparison),
                    count: Some(NonZeroU32::new(2).unwrap()),
                },
            ];

            assert!(matches!(
                supported_texture_bind_group_layout_kind(&entries),
                Some(BindGroupLayoutKind::TextureSampler {
                    texture_binding: 4,
                    sampler_binding: 5,
                    count: 2,
                    visibility: wgt::ShaderStages::FRAGMENT,
                })
            ));
        }
    }

    #[test]
    fn recognizes_standalone_integer_sampled_texture_layouts() {
        for sample_type in [wgt::TextureSampleType::Uint, wgt::TextureSampleType::Sint] {
            let entry = wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Texture {
                    sample_type,
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            };

            assert!(matches!(
                supported_bind_group_layout_kind(&[entry]),
                Some(BindGroupLayoutKind::SampledTexture {
                    binding: 0,
                    visibility: wgt::ShaderStages::COMPUTE,
                })
            ));
        }
    }

    #[test]
    fn recognizes_standalone_depth_sampled_texture_layouts() {
        for view_dimension in [
            wgt::TextureViewDimension::D2,
            wgt::TextureViewDimension::D2Array,
        ] {
            let entry = wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Depth,
                    view_dimension,
                    multisampled: false,
                },
                count: None,
            };
            assert!(matches!(
                supported_bind_group_layout_kind(&[entry]),
                Some(BindGroupLayoutKind::SampledTexture { .. })
            ));
        }
    }

    #[test]
    fn recognizes_standalone_multisampled_texture_layouts() {
        for sample_type in [
            wgt::TextureSampleType::Float { filterable: false },
            wgt::TextureSampleType::Uint,
            wgt::TextureSampleType::Sint,
            wgt::TextureSampleType::Depth,
        ] {
            let entry = wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Texture {
                    sample_type,
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: true,
                },
                count: None,
            };
            assert!(matches!(
                supported_bind_group_layout_kind(&[entry]),
                Some(BindGroupLayoutKind::SampledTexture { .. })
            ));
        }
    }

    #[test]
    fn recognizes_mixed_storage_buffer_and_integer_sampled_texture_layout() {
        let entries = [
            wgt::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Uint,
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgt::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgt::ShaderStages::COMPUTE,
                ty: wgt::BindingType::Buffer {
                    ty: wgt::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        let Some(BindGroupLayoutKind::BufferSampledTextureGroup {
            buffers,
            sampled_textures,
        }) = supported_bind_group_layout_kind(&entries)
        else {
            panic!("expected a mixed sampled texture and storage buffer layout");
        };
        assert!(matches!(
            buffers.as_slice(),
            [BufferBindGroupLayoutKind::Storage { binding: 1, .. }]
        ));
        assert!(matches!(
            sampled_textures.as_slice(),
            [SampledTextureBindGroupLayoutKind {
                texture_binding: 0,
                visibility: wgt::ShaderStages::COMPUTE,
            }]
        ));
    }

    #[test]
    fn depth32float_is_sampleable() {
        assert!(
            deko3d_texture_format_capabilities(wgt::TextureFormat::Depth32Float)
                .contains(crate::TextureFormatCapabilities::SAMPLED)
        );
    }

    #[test]
    fn depth16unorm_is_sampleable_and_copyable() {
        let capabilities = deko3d_texture_format_capabilities(wgt::TextureFormat::Depth16Unorm);
        assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
    }

    #[test]
    fn depth24plusstencil8_is_sampleable_attachment_without_texel_copies() {
        let capabilities =
            deko3d_texture_format_capabilities(wgt::TextureFormat::Depth24PlusStencil8);
        assert!(capabilities.contains(crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
        assert!(!capabilities.intersects(
            crate::TextureFormatCapabilities::COPY_SRC | crate::TextureFormatCapabilities::COPY_DST,
        ));
    }

    #[test]
    fn depth32floatstencil8_is_feature_gated_sampleable_attachment_without_texel_copies() {
        assert!(supported_features().contains(wgt::Features::DEPTH32FLOAT_STENCIL8));
        let capabilities =
            deko3d_texture_format_capabilities(wgt::TextureFormat::Depth32FloatStencil8);
        assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT));
        assert!(!capabilities.intersects(
            crate::TextureFormatCapabilities::COPY_SRC | crate::TextureFormatCapabilities::COPY_DST,
        ));
    }

    #[test]
    fn advertises_all_supported_timestamp_write_positions() {
        let features = supported_features();
        assert!(features.contains(wgt::Features::TIMESTAMP_QUERY));
        assert!(features.contains(wgt::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS));
        assert!(features.contains(wgt::Features::TIMESTAMP_QUERY_INSIDE_PASSES));
    }

    #[test]
    fn gates_incomplete_pipeline_statistics_query_support() {
        assert!(!supported_features().contains(wgt::Features::PIPELINE_STATISTICS_QUERY));
    }

    #[test]
    fn attachment_formats_expose_implemented_multisample_modes() {
        let color = deko3d_texture_format_capabilities(wgt::TextureFormat::Rgba8Unorm);
        assert!(color.contains(crate::TextureFormatCapabilities::MULTISAMPLE_X2));
        assert!(color.contains(crate::TextureFormatCapabilities::MULTISAMPLE_X4));
        assert!(color.contains(crate::TextureFormatCapabilities::MULTISAMPLE_X8));
        assert!(color.contains(crate::TextureFormatCapabilities::MULTISAMPLE_RESOLVE));

        let depth = deko3d_texture_format_capabilities(wgt::TextureFormat::Depth32Float);
        assert!(depth.contains(crate::TextureFormatCapabilities::MULTISAMPLE_X2));
        assert!(depth.contains(crate::TextureFormatCapabilities::MULTISAMPLE_X4));
        assert!(depth.contains(crate::TextureFormatCapabilities::MULTISAMPLE_X8));
        assert!(!depth.contains(crate::TextureFormatCapabilities::MULTISAMPLE_RESOLVE));

        let sampled = deko3d_texture_format_capabilities(wgt::TextureFormat::R8Unorm);
        assert!(!sampled.intersects(
            crate::TextureFormatCapabilities::MULTISAMPLE_X2
                | crate::TextureFormatCapabilities::MULTISAMPLE_X4
                | crate::TextureFormatCapabilities::MULTISAMPLE_X8,
        ));
    }

    #[test]
    fn r8_and_rg8_formats_are_sampled_copy_and_storage_formats() {
        for format in [wgt::TextureFormat::R8Unorm, wgt::TextureFormat::Rg8Unorm] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_ONLY));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_WRITE));
        }
    }

    #[test]
    fn r8_and_rg8_integer_formats_are_sampled_copy_only() {
        for format in [
            wgt::TextureFormat::R8Uint,
            wgt::TextureFormat::R8Sint,
            wgt::TextureFormat::Rg8Uint,
            wgt::TextureFormat::Rg8Sint,
        ] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.intersects(
                crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                    | crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_READ_WRITE,
            ));
        }
    }

    #[test]
    fn snorm_formats_are_sampled_copy_only() {
        for format in [
            wgt::TextureFormat::R8Snorm,
            wgt::TextureFormat::Rg8Snorm,
            wgt::TextureFormat::Rgba8Snorm,
        ] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.intersects(
                crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                    | crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_READ_WRITE,
            ));
        }
    }

    #[test]
    fn r16_and_rg16_formats_are_sampled_copy_only_formats() {
        for format in [wgt::TextureFormat::R16Float, wgt::TextureFormat::Rg16Float] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT));
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_ONLY));
        }
    }

    #[test]
    fn normalized_16bit_formats_are_feature_gated_sampled_copy_only() {
        assert!(supported_features().contains(wgt::Features::TEXTURE_FORMAT_16BIT_NORM));
        for format in [
            wgt::TextureFormat::R16Unorm,
            wgt::TextureFormat::R16Snorm,
            wgt::TextureFormat::Rg16Unorm,
            wgt::TextureFormat::Rg16Snorm,
            wgt::TextureFormat::Rgba16Unorm,
            wgt::TextureFormat::Rgba16Snorm,
        ] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.intersects(
                crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                    | crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_READ_WRITE,
            ));
        }
    }

    #[test]
    fn integer_16bit_formats_are_sampled_copy_only() {
        for format in [
            wgt::TextureFormat::R16Uint,
            wgt::TextureFormat::R16Sint,
            wgt::TextureFormat::Rg16Uint,
            wgt::TextureFormat::Rg16Sint,
            wgt::TextureFormat::Rgba16Uint,
            wgt::TextureFormat::Rgba16Sint,
        ] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.intersects(
                crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                    | crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_READ_WRITE,
            ));
        }
    }

    #[test]
    fn formats_32bit_are_sampled_copy_only() {
        for format in [
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
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(!capabilities.intersects(
                crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                    | crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                    | crate::TextureFormatCapabilities::STORAGE_READ_WRITE,
            ));
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
    fn rgba8_integer_formats_are_color_copy_formats_without_blending() {
        for format in [wgt::TextureFormat::Rgba8Uint, wgt::TextureFormat::Rgba8Sint] {
            let capabilities = deko3d_texture_format_capabilities(format);
            assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
            assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
            assert!(
                !capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT_BLEND)
            );
            assert!(!capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_ONLY));
        }
    }

    #[test]
    fn rgba16float_is_a_blendable_sampled_color_copy_and_storage_format() {
        let capabilities = deko3d_texture_format_capabilities(wgt::TextureFormat::Rgba16Float);
        assert!(capabilities.contains(crate::TextureFormatCapabilities::SAMPLED));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT_BLEND));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_SRC));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::COPY_DST));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_ONLY));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY));
        assert!(capabilities.contains(crate::TextureFormatCapabilities::STORAGE_READ_WRITE));
    }

    #[test]
    fn timestamp_query_ranges_are_checked() {
        let query_set = QuerySetInner {
            query_type: wgt::QueryType::Timestamp,
            count: 2,
        };
        assert!(query_set.validate_range(0..2).is_ok());
        assert!(query_set.validate_range(2..2).is_ok());
        assert!(query_set.validate_range(1..3).is_err());
    }

    #[test]
    fn occlusion_query_ranges_are_checked() {
        let query_set = QuerySetInner {
            query_type: wgt::QueryType::Occlusion,
            count: 2,
        };
        assert!(query_set.validate_range(0..2).is_ok());
        assert!(query_set.validate_range(2..2).is_ok());
        assert!(query_set.validate_range(1..3).is_err());
    }

    #[test]
    fn graphics_pipeline_statistics_query_ranges_are_checked() {
        let query_set = QuerySetInner {
            query_type: wgt::QueryType::PipelineStatistics(deko_pipeline_statistics()),
            count: 2,
        };
        assert!(query_set.validate_range(0..2).is_ok());
        assert_eq!(query_set.result_count(), 4);
    }

    #[test]
    fn compute_pipeline_statistics_are_rejected() {
        let query_set = QuerySetInner {
            query_type: wgt::QueryType::PipelineStatistics(
                wgt::PipelineStatisticsTypes::COMPUTE_SHADER_INVOCATIONS,
            ),
            count: 1,
        };
        assert!(query_set.validate_range(0..1).is_err());
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
