#![allow(unused_variables)]

#[cfg(target_os = "horizon")]
use alloc::boxed::Box;
use alloc::{string::String, vec, vec::Vec};
#[cfg(any(target_os = "horizon", test))]
use core::mem::{align_of, size_of};
#[cfg(all(deko3d, not(target_os = "horizon")))]
use core::sync::atomic::AtomicUsize;
use core::{
    ptr,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use std::sync::Mutex;

use core::fmt;
#[cfg(supports_64bit_atomics)]
use core::sync::atomic::AtomicU64;
#[cfg(not(supports_64bit_atomics))]
use portable_atomic::AtomicU64;

use crate::TlasInstance;

cfg_if::cfg_if! {
    if #[cfg(supports_ptr_atomics)] {
        use alloc::sync::Arc;
    } else if #[cfg(feature = "portable-atomic")] {
        use portable_atomic_util::Arc;
    }
}

// Keep the raw ABI types available on host platforms as well as Horizon. deko3d-sys deliberately
// performs no native linking off-target, which lets pure backend mapping and validation code compile
// and run in ordinary host CI.
use deko3d_sys as dk;

mod buffer;
pub use buffer::Buffer;
#[cfg(target_os = "horizon")]
use buffer::GpuBufferPool;
mod command;
pub use command::CommandBuffer;
#[cfg(target_os = "horizon")]
mod trace;

#[cfg(target_os = "horizon")]
/// Dumps the bounded Deko3D resource and command trace to standard error.
pub fn dump_debug_trace(reason: &str) {
    trace::dump(reason);
}

#[derive(Clone, Debug)]
pub struct Api;
#[derive(Debug)]
pub struct Instance;
#[derive(Clone, Debug)]
pub struct Adapter;
pub struct Surface {
    #[allow(dead_code)]
    state: Mutex<Option<Arc<SurfaceState>>>,
    #[cfg(target_os = "horizon")]
    native_window: DefaultWindow,
    #[allow(dead_code)]
    lease: DefaultSurfaceLease,
    #[allow(dead_code)]
    id: u64,
    #[allow(dead_code)]
    next_configuration: AtomicU64,
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
    state: Mutex<QueueState>,
}
#[derive(Debug)]
pub struct Encoder;
#[derive(Clone, Debug)]
pub enum Resource {
    Placeholder,
    BindGroupLayout(BindGroupLayoutKind),
    PipelineLayout(Arc<PipelineLayoutInner>),
    PipelineCache,
    QuerySet(Arc<QuerySetInner>),
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
        device_id: usize,
        configuration: u64,
        surface_id: u64,
        generation: Arc<SurfaceState>,
    },
    TextureView {
        id: u64,
        label: Option<String>,
        image: RawImage,
        extent: wgt::Extent3d,
        sample_count: u32,
        format: wgt::TextureFormat,
        aspect: wgt::TextureAspect,
        view_dimension: wgt::TextureViewDimension,
        owner: Option<Arc<TextureInner>>,
        surface_generation: Option<Arc<SurfaceState>>,
    },
}

impl Resource {
    fn surface_texture_view(
        image: RawImage,
        extent: wgt::Extent3d,
        generation: Arc<SurfaceState>,
    ) -> Self {
        Self::TextureView {
            id: 0,
            label: Some(String::from("surface")),
            image,
            extent,
            sample_count: 1,
            format: wgt::TextureFormat::Rgba8Unorm,
            aspect: wgt::TextureAspect::All,
            view_dimension: wgt::TextureViewDimension::D2,
            owner: None,
            surface_generation: Some(generation),
        }
    }
}

#[derive(Clone, Debug)]
pub enum BindGroupLayoutKind {
    TextureSamplers {
        bindings: Vec<(u32, Option<u32>, u32, wgt::ShaderStages)>,
        uniforms: Vec<(u32, wgt::ShaderStages, bool)>,
    },
    UniformBuffers(Vec<(u32, wgt::ShaderStages, bool)>),
    Inactive,
    StorageBuffer {
        binding: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
    },
    BufferGroup(Vec<BufferBindGroupLayoutKind>),
    StorageTexture {
        binding: u32,
        visibility: wgt::ShaderStages,
        access: wgt::StorageTextureAccess,
        format: wgt::TextureFormat,
        view_dimension: wgt::TextureViewDimension,
        count: u32,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum BufferBindGroupLayoutKind {
    Uniform {
        binding: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
        count: u32,
    },
    Storage {
        binding: u32,
        visibility: wgt::ShaderStages,
        read_only: bool,
        has_dynamic_offset: bool,
        count: u32,
    },
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
    value: AtomicU64,
    #[allow(dead_code)]
    queue: Mutex<Option<RawQueueHandle>>,
    #[cfg(target_os = "horizon")]
    raw: Mutex<dk::DkFence>,
}

type DeviceResult<T> = Result<T, crate::DeviceError>;

#[derive(Debug)]
struct DeviceInner {
    #[allow(dead_code)]
    raw: RawDevice,
    #[cfg(target_os = "horizon")]
    buffer_pool: Arc<GpuBufferPool>,
}

#[doc(hidden)]
pub struct SurfaceState {
    #[allow(dead_code)]
    inner: Mutex<SurfaceStateInner>,
    #[allow(dead_code)]
    configuration: u64,
}

#[derive(Debug)]
struct QueueState {
    #[allow(dead_code)]
    raw: RawQueue,
    #[cfg(target_os = "horizon")]
    command_recorder: CommandRecorder,
}

#[cfg(target_os = "horizon")]
unsafe impl Send for QueueState {}

#[cfg(target_os = "horizon")]
unsafe impl Send for SurfaceState {}
#[cfg(target_os = "horizon")]
unsafe impl Sync for SurfaceState {}

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

#[derive(Debug)]
pub struct PipelineLayoutInner {
    immediate_size: u32,
    bind_group_layouts: Vec<Option<BindGroupLayoutKind>>,
    #[cfg(target_os = "horizon")]
    immediate_buffer: Option<Buffer>,
}

#[derive(Debug)]
pub struct QuerySetInner {
    query_type: wgt::QueryType,
    count: u32,
    #[cfg(target_os = "horizon")]
    report_buffer: Buffer,
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
    #[cfg(target_os = "horizon")]
    id: u64,
    #[cfg(target_os = "horizon")]
    label: Option<String>,
    #[cfg(target_os = "horizon")]
    traced_texture_targets: AtomicU64,
}

#[cfg(target_os = "horizon")]
struct RawDevice(dk::DkDevice);

#[cfg(target_os = "horizon")]
struct RawQueue(dk::DkQueue);

#[cfg(target_os = "horizon")]
struct CommandRecorder {
    mem_block: dk::DkMemBlock,
    cmdbuf: dk::DkCmdBuf,
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy)]
pub struct RawImage(*const dk::DkImage, dk::DkImageView);

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy)]
pub struct RawQueueHandle(dk::DkQueue);

#[cfg(target_os = "horizon")]
struct SurfaceStateInner {
    #[allow(dead_code)]
    device: Arc<DeviceInner>,
    render_queue: dk::DkQueue,
    framebuffer_mem_block: dk::DkMemBlock,
    framebuffers: Box<[dk::DkImage; FRAMEBUFFER_COUNT]>,
    swapchain: dk::DkSwapchain,
    acquired_slot: Option<i32>,
    extent: wgt::Extent3d,
}

#[cfg(target_os = "horizon")]
struct ShaderModuleInnerRaw {
    code_mem_block: dk::DkMemBlock,
    shader: dk::DkShader,
    bindings: Vec<ShaderBinding>,
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ShaderBindingKind {
    Uniform,
    Texture,
    Sampler,
    Storage,
    StorageTexture,
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy, Debug)]
pub(super) struct ShaderBinding {
    pub(super) group: u32,
    pub(super) binding: u32,
    pub(super) target: u32,
    pub(super) kind: ShaderBindingKind,
}

#[cfg(target_os = "horizon")]
pub(super) struct RenderPipelineInnerRaw {
    pipeline_layout: Arc<PipelineLayoutInner>,
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
    uses_depth_stencil: bool,
    stencil_read_mask: u8,
    stencil_write_mask: u8,
    bind_group_count: usize,
    vertex_bindings: Vec<ShaderBinding>,
    fragment_bindings: Vec<ShaderBinding>,
}

#[cfg(target_os = "horizon")]
pub(super) struct ComputePipelineInnerRaw {
    pipeline_layout: Arc<PipelineLayoutInner>,
    compute_shader: Arc<ShaderModuleInner>,
}

#[cfg(target_os = "horizon")]
struct TextureInnerRaw {
    id: u64,
    label: Option<String>,
    mem_block: dk::DkMemBlock,
    image: dk::DkImage,
    extent: wgt::Extent3d,
    sample_count: u32,
    dimension: wgt::TextureDimension,
    format: wgt::TextureFormat,
}

#[cfg(target_os = "horizon")]
struct SamplerInnerRaw {
    id: u64,
    label: Option<String>,
    sampler: dk::DkSampler,
}

#[cfg(target_os = "horizon")]
pub(super) enum BindGroupInnerRaw {
    TextureSamplers {
        bindings: Vec<TextureSamplerBinding>,
        descriptor_mem_block: dk::DkMemBlock,
        image_descriptor_gpu_addr: dk::DkGpuAddr,
        sampler_descriptor_gpu_addr: dk::DkGpuAddr,
        image_descriptor_stride: u64,
        sampler_descriptor_stride: u64,
        uniforms: Vec<UniformBufferBinding>,
    },
    UniformBuffers(Vec<UniformBufferBinding>),
    StorageBuffer(StorageBufferBinding),
    BufferGroup(Vec<BufferBinding>),
    StorageTexture(StorageTextureBinding),
    Inactive,
}

#[cfg(target_os = "horizon")]
pub(super) struct TextureSamplerBinding {
    texture_binding: u32,
    visibility: wgt::ShaderStages,
    view_id: u64,
    #[allow(dead_code)]
    texture: Arc<TextureInner>,
    #[allow(dead_code)]
    sampler: Option<Arc<SamplerInner>>,
    image_descriptor: dk::DkImageDescriptor,
    sampler_descriptor: dk::DkSamplerDescriptor,
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
    elements: Vec<StorageTextureElement>,
    descriptor_mem_block: dk::DkMemBlock,
    image_descriptor_gpu_addr: dk::DkGpuAddr,
    image_descriptor_stride: u64,
}

#[cfg(target_os = "horizon")]
pub(super) struct StorageTextureElement {
    texture: Arc<TextureInner>,
    image_descriptor: dk::DkImageDescriptor,
}

#[cfg(any(target_os = "horizon", test))]
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

#[cfg(any(target_os = "horizon", test))]
#[repr(C)]
#[derive(Clone, Copy)]
struct DkshProgramHeader {
    type_: u32,
    entrypoint: u32,
    num_gprs: u32,
    constbuf1_off: u32,
    constbuf1_sz: u32,
    per_warp_scratch_sz: u32,
    payload: [u8; 36],
    reserved: u32,
}

#[cfg(any(target_os = "horizon", test))]
const DKSH_MAGIC: u32 = u32::from_le_bytes(*b"DKSH");
#[cfg(any(target_os = "horizon", test))]
const DKSH_SECTION_ALIGNMENT: u32 = 256;
#[cfg(any(target_os = "horizon", test))]
const DKSH_PROGRAM_TYPE_COMPUTE: u32 = 5;

#[cfg(any(target_os = "horizon", test))]
fn validate_dksh(bytes: &[u8]) -> Result<DkshHeader, crate::ShaderError> {
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
        || header.num_programs != 1
        || header.programs_off < header.header_sz
        || header.control_sz % DKSH_SECTION_ALIGNMENT != 0
        || header.code_sz % DKSH_SECTION_ALIGNMENT != 0
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

    let table_offset = usize::try_from(header.programs_off)
        .map_err(|_| shader_error("deko3d DKSH program table offset is too large"))?;
    let table_len = usize::try_from(header.num_programs)
        .ok()
        .and_then(|count| count.checked_mul(size_of::<DkshProgramHeader>()))
        .ok_or_else(|| shader_error("deko3d DKSH program table overflows"))?;
    let table_end = table_offset
        .checked_add(table_len)
        .ok_or_else(|| shader_error("deko3d DKSH program table overflows"))?;
    if table_offset % align_of::<u32>() != 0 || table_end > control_len {
        return Err(shader_error(
            "deko3d DKSH program table exceeds the control section",
        ));
    }

    for program_bytes in bytes[table_offset..table_end].chunks_exact(size_of::<DkshProgramHeader>())
    {
        let program =
            unsafe { ptr::read_unaligned(program_bytes.as_ptr().cast::<DkshProgramHeader>()) };
        if program.type_ > DKSH_PROGRAM_TYPE_COMPUTE
            || program.entrypoint >= header.code_sz
            || program.constbuf1_off > header.code_sz
            || program.constbuf1_sz > header.code_sz - program.constbuf1_off
        {
            return Err(shader_error("deko3d DKSH program entry is invalid"));
        }
    }

    Ok(header)
}

#[cfg(target_os = "horizon")]
fn parse_shader_bindings(
    bytes: &[u8],
    header: DkshHeader,
) -> Result<Vec<ShaderBinding>, crate::ShaderError> {
    const MAGIC: &[u8; 8] = b"DKRBv001";
    const ENTRY_SIZE: usize = 16;
    let offset = usize::try_from(header.control_sz)
        .ok()
        .and_then(|control| {
            usize::try_from(header.code_sz)
                .ok()
                .and_then(|code| control.checked_add(code))
        })
        .ok_or_else(|| shader_error("deko3d binding metadata offset overflows"))?;
    if bytes.len() == offset {
        return Ok(Vec::new());
    }
    let header_end = offset
        .checked_add(MAGIC.len() + size_of::<u32>())
        .ok_or_else(|| shader_error("deko3d binding metadata header overflows"))?;
    if bytes.get(offset..offset + MAGIC.len()) != Some(MAGIC) || header_end > bytes.len() {
        return Err(shader_error(
            "deko3d binding metadata has an invalid header",
        ));
    }
    let count =
        u32::from_le_bytes(bytes[offset + MAGIC.len()..header_end].try_into().unwrap()) as usize;
    let entries_end = count
        .checked_mul(ENTRY_SIZE)
        .and_then(|size| header_end.checked_add(size))
        .ok_or_else(|| shader_error("deko3d binding metadata overflows"))?;
    if entries_end != bytes.len() {
        return Err(shader_error(
            "deko3d binding metadata has an invalid length",
        ));
    }
    bytes[header_end..entries_end]
        .chunks_exact(ENTRY_SIZE)
        .map(|entry| {
            let word = |index: usize| {
                u32::from_le_bytes(entry[index * 4..index * 4 + 4].try_into().unwrap())
            };
            let kind = match word(3) {
                0 => ShaderBindingKind::Uniform,
                1 => ShaderBindingKind::Texture,
                2 => ShaderBindingKind::Sampler,
                3 => ShaderBindingKind::Storage,
                4 => ShaderBindingKind::StorageTexture,
                _ => return Err(shader_error("deko3d binding metadata has an invalid kind")),
            };
            Ok(ShaderBinding {
                group: word(0),
                binding: word(1),
                target: word(2),
                kind,
            })
        })
        .collect()
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
#[cfg(target_os = "horizon")]
const CMDMEMSIZE: u32 = 16 * 1024;
const DEKO_UNIFORM_BUFFER_COUNT: u32 = 16;
const DEKO_IMMEDIATES_BINDING: u32 = DEKO_UNIFORM_BUFFER_COUNT - 1;
const DEKO_PUSH_CONSTANTS_MAX_SIZE: u32 = 0x7FFC;
#[cfg(target_os = "horizon")]
const DEKO_COUNTER_REPORT_SIZE: wgt::BufferAddress = 16;
#[cfg(target_os = "horizon")]
const DEKO_QUERY_RESULT_SIZE: wgt::BufferAddress = 8;
const DEKO_UNIFORM_BUF_MAX_SIZE: u64 = 0x10000;
const DEKO_TEXTURE_SAMPLER_COUNT: usize = 32;
const DEKO_STORAGE_TEXTURE_COUNT: u32 = 8;
const DEKO_STORAGE_BUFFER_COUNT: u32 = 16;

static DEFAULT_SURFACE_LEASED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "horizon")]
static ACQUIRE_TRACED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "horizon")]
static SUBMIT_TRACED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "horizon")]
static PRESENT_TRACED: AtomicBool = AtomicBool::new(false);
#[cfg(any(target_os = "horizon", test))]
static NEXT_DEFAULT_SURFACE_ID: AtomicU64 = AtomicU64::new(1);
#[cfg(all(deko3d, not(target_os = "horizon")))]
static FORCED_HOST_INSTANCE_INIT_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[doc(hidden)]
#[cfg(all(deko3d, not(target_os = "horizon")))]
pub fn forced_host_instance_init_attempts() -> usize {
    FORCED_HOST_INSTANCE_INIT_ATTEMPTS.load(Ordering::Acquire)
}

struct DefaultSurfaceLease;

impl DefaultSurfaceLease {
    #[cfg(any(target_os = "horizon", test))]
    fn acquire() -> Result<Self, crate::InstanceError> {
        if DEFAULT_SURFACE_LEASED.swap(true, Ordering::AcqRel) {
            return Err(crate::InstanceError::new(String::from(
                "deko3d supports only one live default Switch surface",
            )));
        }
        Ok(Self)
    }
}

impl Drop for DefaultSurfaceLease {
    fn drop(&mut self) {
        DEFAULT_SURFACE_LEASED.store(false, Ordering::Release);
    }
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Copy, Debug)]
struct DefaultWindow(ptr::NonNull<dk::NWindow>);

// libnx owns the process-global default window for the application's lifetime. Surface state
// serializes every use of its pointer, so moving this handle does not transfer ownership.
#[cfg(target_os = "horizon")]
unsafe impl Send for DefaultWindow {}
#[cfg(target_os = "horizon")]
unsafe impl Sync for DefaultWindow {}

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
        f.debug_struct("Fence")
            .field("value", &self.value)
            .field("queue", &self.queue)
            .finish_non_exhaustive()
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
            .field("mem_block", &self.mem_block)
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

impl Instance {
    pub fn create_default_surface(&self) -> Result<Surface, crate::InstanceError> {
        #[cfg(target_os = "horizon")]
        {
            let lease = DefaultSurfaceLease::acquire()?;
            let native_window =
                ptr::NonNull::new(unsafe { dk::nwindowGetDefault() }).ok_or_else(|| {
                    crate::InstanceError::new(String::from(
                        "deko3d default Switch window is unavailable",
                    ))
                })?;
            Ok(Surface {
                state: Mutex::new(None),
                native_window: DefaultWindow(native_window),
                lease,
                id: NEXT_DEFAULT_SURFACE_ID.fetch_add(1, Ordering::Relaxed),
                next_configuration: AtomicU64::new(1),
            })
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::InstanceError::new(String::from(
                "deko3d default Switch surfaces require the Horizon/Switch target",
            )))
        }
    }
}

impl DeviceInner {
    #[cfg(target_os = "horizon")]
    fn raw_device(&self) -> dk::DkDevice {
        self.raw.0
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

impl PipelineLayoutInner {
    fn bind_group_layouts(
        desc: &crate::PipelineLayoutDescriptor<Resource>,
    ) -> DeviceResult<Vec<Option<BindGroupLayoutKind>>> {
        desc.bind_group_layouts
            .iter()
            .map(|layout| match layout {
                Some(Resource::BindGroupLayout(kind)) => Ok(Some(kind.clone())),
                Some(_) => Err(crate::DeviceError::Lost),
                None => Ok(None),
            })
            .collect()
    }

    #[cfg(target_os = "horizon")]
    fn new(
        desc: &crate::PipelineLayoutDescriptor<Resource>,
        pool: Arc<GpuBufferPool>,
    ) -> DeviceResult<Self> {
        if u64::from(desc.immediate_size) > u64::from(dk::DK_UNIFORM_BUF_MAX_SIZE) {
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
                pool,
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
        if u64::from(desc.immediate_size) > u64::from(dk::DK_UNIFORM_BUF_MAX_SIZE) {
            return Err(crate::DeviceError::Lost);
        }
        Ok(Self {
            immediate_size: desc.immediate_size,
            bind_group_layouts: Self::bind_group_layouts(desc)?,
        })
    }

    pub(super) fn contains_immediate_range(&self, offset_bytes: u32, data: &[u32]) -> bool {
        let Some(size_bytes) = u32::try_from(data.len())
            .ok()
            .and_then(|words| words.checked_mul(wgt::IMMEDIATE_DATA_ALIGNMENT))
        else {
            return false;
        };
        offset_bytes
            .checked_add(size_bytes)
            .is_some_and(|end| end <= self.immediate_size)
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn bind_immediates(
        &self,
        cmdbuf: dk::DkCmdBuf,
        visibility: wgt::ShaderStages,
    ) -> DeviceResult<()> {
        let Some(buffer) = self.immediate_buffer.as_ref() else {
            return Ok(());
        };
        let (gpu_addr, gpu_size) = buffer.gpu_binding(0, None)?;
        for (flag, stage) in [
            (wgt::ShaderStages::VERTEX, dk::DkStage::DkStage_Vertex),
            (wgt::ShaderStages::FRAGMENT, dk::DkStage::DkStage_Fragment),
            (wgt::ShaderStages::COMPUTE, dk::DkStage::DkStage_Compute),
        ] {
            if visibility.contains(flag) {
                unsafe {
                    dk::dkCmdBufBindUniformBuffer(
                        cmdbuf,
                        stage,
                        DEKO_IMMEDIATES_BINDING,
                        gpu_addr,
                        gpu_size,
                    );
                }
            }
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn push_immediates(
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
                .and_then(|words| words.checked_mul(wgt::IMMEDIATE_DATA_ALIGNMENT))
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

impl QuerySetInner {
    pub(super) fn is_timestamp(&self) -> bool {
        matches!(self.query_type, wgt::QueryType::Timestamp)
    }

    pub(super) fn is_occlusion(&self) -> bool {
        matches!(self.query_type, wgt::QueryType::Occlusion)
    }

    fn is_supported(&self) -> bool {
        self.is_timestamp() || self.is_occlusion()
    }

    pub(super) fn validate_range(&self, range: core::ops::Range<u32>) -> DeviceResult<()> {
        if !self.is_supported() || range.start > range.end || range.end > self.count {
            return Err(crate::DeviceError::Lost);
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    fn new(
        desc: &wgt::QuerySetDescriptor<crate::Label>,
        pool: Arc<GpuBufferPool>,
    ) -> DeviceResult<Self> {
        if !matches!(
            desc.ty,
            wgt::QueryType::Timestamp | wgt::QueryType::Occlusion
        ) || desc.count == 0
        {
            return Err(crate::DeviceError::Lost);
        }
        let size = u64::from(desc.count)
            .checked_mul(DEKO_COUNTER_REPORT_SIZE)
            .ok_or(crate::DeviceError::OutOfMemory)?;
        let report_buffer = Buffer::new(
            &crate::BufferDescriptor {
                label: None,
                size,
                usage: wgt::BufferUses::COPY_SRC,
                memory_flags: crate::MemoryFlags::empty(),
            },
            pool,
        )?;
        Ok(Self {
            query_type: desc.ty,
            count: desc.count,
            report_buffer,
        })
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn report_address(&self, index: u32) -> DeviceResult<dk::DkGpuAddr> {
        self.validate_range(index..index.checked_add(1).ok_or(crate::DeviceError::Lost)?)?;
        let offset = u64::from(index)
            .checked_mul(DEKO_COUNTER_REPORT_SIZE)
            .ok_or(crate::DeviceError::Lost)?;
        let size =
            wgt::BufferSize::new(DEKO_COUNTER_REPORT_SIZE).ok_or(crate::DeviceError::Lost)?;
        self.report_buffer
            .gpu_binding(offset, Some(size))
            .map(|(address, _)| address)
    }
}

impl TextureInner {
    #[cfg(target_os = "horizon")]
    pub(super) fn id(&self) -> u64 {
        self.inner.id
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn raw_image(&self) -> RawImage {
        let image = &self.inner.image;
        RawImage(image, dk::DkImageView::defaults(image))
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn raw_image(&self) -> RawImage {
        RawImage
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn extent(&self) -> wgt::Extent3d {
        self.inner.extent
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn dimension(&self) -> wgt::TextureDimension {
        self.inner.dimension
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn format(&self) -> wgt::TextureFormat {
        self.inner.format
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn sample_count(&self) -> u32 {
        self.inner.sample_count
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn extent(&self) -> wgt::Extent3d {
        wgt::Extent3d {
            width: 0,
            height: 0,
            depth_or_array_layers: 1,
        }
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn format(&self) -> wgt::TextureFormat {
        wgt::TextureFormat::Rgba8Unorm
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn sample_count(&self) -> u32 {
        1
    }
}

impl BindGroupInner {
    #[cfg(target_os = "horizon")]
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn texture_descriptor_set(
        &self,
    ) -> Option<(dk::DkGpuAddr, dk::DkGpuAddr, u64, u64)> {
        match &self.inner {
            BindGroupInnerRaw::TextureSamplers {
                image_descriptor_gpu_addr,
                sampler_descriptor_gpu_addr,
                image_descriptor_stride,
                sampler_descriptor_stride,
                ..
            } => Some((
                *image_descriptor_gpu_addr,
                *sampler_descriptor_gpu_addr,
                *image_descriptor_stride,
                *sampler_descriptor_stride,
            )),
            _ => None,
        }
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn push_texture_binding(
        &self,
        cmdbuf: dk::DkCmdBuf,
        image_descriptor_gpu_addr: dk::DkGpuAddr,
        sampler_descriptor_gpu_addr: dk::DkGpuAddr,
        image_descriptor_stride: u64,
        sampler_descriptor_stride: u64,
        shader_group: u32,
        binding_number: u32,
        target: u32,
        stage: dk::DkStage,
    ) -> DeviceResult<()> {
        let BindGroupInnerRaw::TextureSamplers { bindings, .. } = &self.inner else {
            return Err(crate::DeviceError::Lost);
        };
        let matching = bindings
            .iter()
            .filter(|binding| binding.texture_binding == binding_number)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(crate::DeviceError::Lost);
        }
        let stage_visibility = match stage {
            dk::DkStage::DkStage_Vertex => wgt::ShaderStages::VERTEX,
            dk::DkStage::DkStage_Fragment => wgt::ShaderStages::FRAGMENT,
            dk::DkStage::DkStage_Compute => wgt::ShaderStages::COMPUTE,
            _ => return Err(crate::DeviceError::Lost),
        };
        if matching
            .iter()
            .any(|binding| !binding.visibility.contains(stage_visibility))
        {
            return Err(crate::DeviceError::Lost);
        }
        for (array_index, binding) in matching.into_iter().enumerate() {
            let target = target
                .checked_add(u32::try_from(array_index).map_err(|_| crate::DeviceError::Lost)?)
                .ok_or(crate::DeviceError::Lost)?;
            if target < 64 {
                let mask = 1_u64 << target;
                if self
                    .traced_texture_targets
                    .fetch_or(mask, Ordering::Relaxed)
                    & mask
                    == 0
                {
                    trace::record(format_args!(
                        "bind bind_group_id={} shader_group={} binding={} native_target={} texture_id={} view_id={} sampler_id={:?}",
                        self.id,
                        shader_group,
                        binding_number,
                        target,
                        binding.texture.id(),
                        binding.view_id,
                        binding.sampler.as_ref().map(|sampler| sampler.inner.id)
                    ));
                }
            }
            let target_offset = u64::from(target);
            unsafe {
                dk::dkCmdBufPushData(
                    cmdbuf,
                    image_descriptor_gpu_addr + target_offset * image_descriptor_stride,
                    ptr::addr_of!(binding.image_descriptor).cast(),
                    size_of::<dk::DkImageDescriptor>() as u32,
                );
                dk::dkCmdBufPushData(
                    cmdbuf,
                    sampler_descriptor_gpu_addr + target_offset * sampler_descriptor_stride,
                    ptr::addr_of!(binding.sampler_descriptor).cast(),
                    size_of::<dk::DkSamplerDescriptor>() as u32,
                );
                dk::dkCmdBufBindTexture(
                    cmdbuf,
                    stage,
                    target,
                    dk::dkMakeTextureHandle(target, target),
                );
            }
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn bind_uniform_binding(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_number: u32,
        target: u32,
        stage: dk::DkStage,
    ) -> DeviceResult<()> {
        let bindings = match &self.inner {
            BindGroupInnerRaw::TextureSamplers { uniforms, .. } => uniforms,
            BindGroupInnerRaw::UniformBuffers(bindings) => bindings,
            BindGroupInnerRaw::StorageBuffer(_) => return Err(crate::DeviceError::Lost),
            BindGroupInnerRaw::BufferGroup(bindings) => {
                let index = bindings
                    .iter()
                    .position(|binding| {
                        matches!(binding,
                            BufferBinding::Uniform(uniform) if uniform.binding == binding_number
                        ) || matches!(binding,
                            BufferBinding::UniformArray(array) if array.first().is_some_and(|uniform| uniform.binding == binding_number)
                        )
                    })
                    .ok_or(crate::DeviceError::Lost)?;
                return match &bindings[index] {
                    BufferBinding::Uniform(binding) => {
                        let dynamic_offset =
                            grouped_dynamic_offset(bindings, index, dynamic_offsets)?;
                        unsafe {
                            bind_uniform_buffer(cmdbuf, binding, dynamic_offset, target, stage)
                        }
                    }
                    BufferBinding::UniformArray(array) if dynamic_offsets.is_empty() => {
                        for (array_index, binding) in array.iter().enumerate() {
                            let target = target
                                .checked_add(
                                    u32::try_from(array_index)
                                        .map_err(|_| crate::DeviceError::Lost)?,
                                )
                                .ok_or(crate::DeviceError::Lost)?;
                            unsafe { bind_uniform_buffer(cmdbuf, binding, 0, target, stage)? };
                        }
                        Ok(())
                    }
                    _ => Err(crate::DeviceError::Lost),
                };
            }
            BindGroupInnerRaw::StorageTexture(_) => return Err(crate::DeviceError::Lost),
            BindGroupInnerRaw::Inactive => return Ok(()),
        };
        let index = bindings
            .iter()
            .position(|binding| binding.binding == binding_number)
            .ok_or(crate::DeviceError::Lost)?;
        let binding = &bindings[index];
        let dynamic_offset = if binding.has_dynamic_offset {
            let dynamic_index = bindings[..index]
                .iter()
                .filter(|binding| binding.has_dynamic_offset)
                .count();
            u64::from(
                *dynamic_offsets
                    .get(dynamic_index)
                    .ok_or(crate::DeviceError::Lost)?,
            )
        } else {
            0
        };
        unsafe { bind_uniform_buffer(cmdbuf, binding, dynamic_offset, target, stage) }
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn bind_storage_binding(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_number: u32,
        target: u32,
        stage: dk::DkStage,
    ) -> DeviceResult<()> {
        let (binding, dynamic_offset) = match &self.inner {
            BindGroupInnerRaw::StorageBuffer(binding) => {
                let dynamic_offset = match (binding.has_dynamic_offset, dynamic_offsets) {
                    (true, [offset]) => u64::from(*offset),
                    (false, []) => 0,
                    _ => return Err(crate::DeviceError::Lost),
                };
                (binding, dynamic_offset)
            }
            BindGroupInnerRaw::BufferGroup(bindings) => {
                let index = bindings
                    .iter()
                    .position(|binding| {
                        matches!(binding,
                            BufferBinding::Storage(storage) if storage.binding == binding_number
                        ) || matches!(binding,
                            BufferBinding::StorageArray(array) if array.first().is_some_and(|storage| storage.binding == binding_number)
                        )
                    })
                    .ok_or(crate::DeviceError::Lost)?;
                match &bindings[index] {
                    BufferBinding::Storage(binding) => (
                        binding,
                        grouped_dynamic_offset(bindings, index, dynamic_offsets)?,
                    ),
                    BufferBinding::StorageArray(array) if dynamic_offsets.is_empty() => {
                        for (array_index, binding) in array.iter().enumerate() {
                            let target = target
                                .checked_add(
                                    u32::try_from(array_index)
                                        .map_err(|_| crate::DeviceError::Lost)?,
                                )
                                .ok_or(crate::DeviceError::Lost)?;
                            unsafe {
                                bind_storage_buffer(
                                    cmdbuf,
                                    binding,
                                    binding_number,
                                    0,
                                    target,
                                    stage,
                                )?
                            };
                        }
                        return Ok(());
                    }
                    _ => return Err(crate::DeviceError::Lost),
                }
            }
            _ => return Err(crate::DeviceError::Lost),
        };
        unsafe {
            bind_storage_buffer(
                cmdbuf,
                binding,
                binding_number,
                dynamic_offset,
                target,
                stage,
            )
        }
    }
}

#[cfg(target_os = "horizon")]
unsafe fn bind_storage_buffer(
    cmdbuf: dk::DkCmdBuf,
    binding: &StorageBufferBinding,
    binding_number: u32,
    dynamic_offset: u64,
    target: u32,
    stage: dk::DkStage,
) -> DeviceResult<()> {
    if dynamic_offset % u64::from(wgt::STORAGE_BINDING_SIZE_ALIGNMENT) != 0
        || binding.binding != binding_number
        || !binding.visibility.contains(match stage {
            dk::DkStage::DkStage_Vertex => wgt::ShaderStages::VERTEX,
            dk::DkStage::DkStage_Fragment => wgt::ShaderStages::FRAGMENT,
            dk::DkStage::DkStage_Compute => wgt::ShaderStages::COMPUTE,
            _ => return Err(crate::DeviceError::Lost),
        })
    {
        return Err(crate::DeviceError::Lost);
    }
    let offset = binding
        .offset
        .checked_add(dynamic_offset)
        .ok_or(crate::DeviceError::Lost)?;
    let (gpu_addr, gpu_size) = binding.buffer.gpu_binding(offset, binding.size)?;
    unsafe { dk::dkCmdBufBindStorageBuffer(cmdbuf, stage, target, gpu_addr, gpu_size) };
    Ok(())
}

impl BindGroupInner {
    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn bind_storage_texture(
        &self,
        cmdbuf: dk::DkCmdBuf,
        dynamic_offsets: &[wgt::DynamicOffset],
        binding_number: u32,
        target: u32,
        stage: dk::DkStage,
    ) -> DeviceResult<()> {
        let BindGroupInnerRaw::StorageTexture(binding) = &self.inner else {
            return Err(crate::DeviceError::Lost);
        };
        let visibility = match stage {
            dk::DkStage::DkStage_Vertex => wgt::ShaderStages::VERTEX,
            dk::DkStage::DkStage_Fragment => wgt::ShaderStages::FRAGMENT,
            dk::DkStage::DkStage_Compute => wgt::ShaderStages::COMPUTE,
            _ => return Err(crate::DeviceError::Lost),
        };
        if !dynamic_offsets.is_empty()
            || binding.binding != binding_number
            || !binding.visibility.contains(visibility)
        {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            for (array_index, element) in binding.elements.iter().enumerate() {
                let index = u32::try_from(array_index).map_err(|_| crate::DeviceError::Lost)?;
                let descriptor_address = binding
                    .image_descriptor_gpu_addr
                    .checked_add(u64::from(index) * binding.image_descriptor_stride)
                    .ok_or(crate::DeviceError::Lost)?;
                dk::dkCmdBufPushData(
                    cmdbuf,
                    descriptor_address,
                    ptr::addr_of!(element.image_descriptor).cast(),
                    size_of::<dk::DkImageDescriptor>() as u32,
                );
            }
            dk::dkCmdBufBindImageDescriptorSet(
                cmdbuf,
                binding.image_descriptor_gpu_addr,
                binding.elements.len() as u32,
            );
            for index in 0..binding.elements.len() as u32 {
                dk::dkCmdBufBindImage(
                    cmdbuf,
                    stage,
                    target.checked_add(index).ok_or(crate::DeviceError::Lost)?,
                    dk::dkMakeImageHandle(index),
                );
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "horizon")]
fn grouped_dynamic_offset(
    bindings: &[BufferBinding],
    binding_index: usize,
    dynamic_offsets: &[wgt::DynamicOffset],
) -> DeviceResult<u64> {
    let has_dynamic_offset = match &bindings[binding_index] {
        BufferBinding::Uniform(binding) => binding.has_dynamic_offset,
        BufferBinding::UniformArray(_) => false,
        BufferBinding::Storage(binding) => binding.has_dynamic_offset,
        BufferBinding::StorageArray(_) => false,
    };
    let dynamic_count = bindings
        .iter()
        .filter(|binding| match binding {
            BufferBinding::Uniform(binding) => binding.has_dynamic_offset,
            BufferBinding::UniformArray(_) => false,
            BufferBinding::Storage(binding) => binding.has_dynamic_offset,
            BufferBinding::StorageArray(_) => false,
        })
        .count();
    if dynamic_count != dynamic_offsets.len() {
        return Err(crate::DeviceError::Lost);
    }
    if !has_dynamic_offset {
        return Ok(0);
    }
    let dynamic_index = bindings[..binding_index]
        .iter()
        .filter(|binding| match binding {
            BufferBinding::Uniform(binding) => binding.has_dynamic_offset,
            BufferBinding::UniformArray(_) => false,
            BufferBinding::Storage(binding) => binding.has_dynamic_offset,
            BufferBinding::StorageArray(_) => false,
        })
        .count();
    dynamic_offsets
        .get(dynamic_index)
        .copied()
        .map(u64::from)
        .ok_or(crate::DeviceError::Lost)
}

#[cfg(target_os = "horizon")]
unsafe fn bind_uniform_buffer(
    cmdbuf: dk::DkCmdBuf,
    binding: &UniformBufferBinding,
    dynamic_offset: u64,
    target: u32,
    stage: dk::DkStage,
) -> DeviceResult<()> {
    let required_visibility = match stage {
        dk::DkStage::DkStage_Vertex => wgt::ShaderStages::VERTEX,
        dk::DkStage::DkStage_Fragment => wgt::ShaderStages::FRAGMENT,
        dk::DkStage::DkStage_Compute => wgt::ShaderStages::COMPUTE,
        _ => return Err(crate::DeviceError::Lost),
    };
    if !binding.visibility.contains(required_visibility) {
        return Err(crate::DeviceError::Lost);
    }
    let offset = binding
        .offset
        .checked_add(dynamic_offset)
        .ok_or(crate::DeviceError::Lost)?;
    let (gpu_addr, gpu_size) = binding.buffer.gpu_binding(offset, binding.size)?;
    if gpu_addr % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0
        || gpu_size > dk::DK_UNIFORM_BUF_MAX_SIZE
    {
        return Err(crate::DeviceError::Lost);
    }
    unsafe {
        dk::dkCmdBufBindUniformBuffer(cmdbuf, stage, target, gpu_addr, gpu_size);
    }
    Ok(())
}

impl Queue {
    #[cfg(target_os = "horizon")]
    pub(super) fn raw_queue(&self) -> DeviceResult<RawQueueHandle> {
        let state = self.state.lock().map_err(|_| crate::DeviceError::Lost)?;
        Ok(RawQueueHandle(state.raw.0))
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn record_and_submit(
        &self,
        raw_queue: dk::DkQueue,
        label: &'static str,
        record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
    ) -> DeviceResult<()> {
        let mut state = self.state.lock().map_err(|_| crate::DeviceError::Lost)?;
        unsafe {
            state
                .command_recorder
                .record_and_submit(raw_queue, label, record)
        }
    }
}

#[cfg(target_os = "horizon")]
impl CommandRecorder {
    unsafe fn new(raw_device: dk::DkDevice) -> DeviceResult<Self> {
        let mut mem_block_maker = dk::DkMemBlockMaker::defaults(raw_device, CMDMEMSIZE);
        mem_block_maker.flags = dk::DkMemBlockFlags_CpuUncached | dk::DkMemBlockFlags_GpuCached;
        let mem_block = unsafe { dk::dkMemBlockCreate(&mem_block_maker) };
        if mem_block.is_null() {
            return Err(crate::DeviceError::OutOfMemory);
        }

        let cmdbuf_maker = dk::DkCmdBufMaker::defaults(raw_device);
        let cmdbuf = unsafe { dk::dkCmdBufCreate(&cmdbuf_maker) };
        if cmdbuf.is_null() {
            unsafe { dk::dkMemBlockDestroy(mem_block) };
            return Err(crate::DeviceError::Lost);
        }

        unsafe { dk::dkCmdBufAddMemory(cmdbuf, mem_block, 0, CMDMEMSIZE) };
        Ok(Self { mem_block, cmdbuf })
    }

    unsafe fn record_and_submit(
        &mut self,
        raw_queue: dk::DkQueue,
        label: &'static str,
        record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
    ) -> DeviceResult<()> {
        unsafe { dk::dkCmdBufClear(self.cmdbuf) };
        let result = record(self.cmdbuf);
        if result.is_ok() {
            unsafe {
                let cmds = dk::dkCmdBufFinishList(self.cmdbuf);
                let _ = label;
                dk::dkQueueSubmitCommands(raw_queue, cmds);
                dk::dkQueueFlush(raw_queue);
                dk::dkQueueWaitIdle(raw_queue);
            }
        }
        unsafe { dk::dkCmdBufClear(self.cmdbuf) };
        if result.is_err() {
            trace::record(format_args!(
                "failure kind=record_and_submit label={label} error={result:?}"
            ));
            trace::dump("record_and_submit_failed");
        }
        result
    }
}

#[cfg(target_os = "horizon")]
impl Drop for DeviceInner {
    fn drop(&mut self) {
        if !self.raw.0.is_null() {
            unsafe { dk::dkDeviceDestroy(self.raw.0) };
        }
    }
}

#[cfg(target_os = "horizon")]
impl Drop for Queue {
    fn drop(&mut self) {
        let state = self
            .state
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner());
        if !state.raw.0.is_null() {
            unsafe {
                dk::dkQueueWaitIdle(state.raw.0);
                dk::dkQueueDestroy(state.raw.0);
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
            if !self.mem_block.is_null() {
                dk::dkMemBlockDestroy(self.mem_block);
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
impl Drop for BindGroupInnerRaw {
    fn drop(&mut self) {
        match self {
            Self::TextureSamplers {
                descriptor_mem_block,
                ..
            }
            | Self::StorageTexture(StorageTextureBinding {
                descriptor_mem_block,
                ..
            }) => {
                if !descriptor_mem_block.is_null() {
                    unsafe { dk::dkMemBlockDestroy(*descriptor_mem_block) };
                }
            }
            _ => {}
        }
    }
}

#[cfg(target_os = "horizon")]
impl SurfaceState {
    unsafe fn new(
        device: Arc<DeviceInner>,
        native_window: DefaultWindow,
        configuration: u64,
        config: &crate::SurfaceConfiguration,
    ) -> Result<Self, crate::SurfaceError> {
        if config.format != wgt::TextureFormat::Rgba8Unorm {
            return Err(crate::SurfaceError::Other(
                "deko3d surface only supports Rgba8Unorm for now",
            ));
        }
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
        image_layout_maker.format = dk::DkImageFormat::DkImageFormat_RGBA8_Unorm;
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

        let mut framebuffers = Box::new([dk::DkImage::zeroed(), dk::DkImage::zeroed()]);
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
            native_window.0.as_ptr().cast(),
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
            inner: Mutex::new(SurfaceStateInner {
                device,
                render_queue,
                framebuffer_mem_block,
                framebuffers,
                swapchain,
                acquired_slot: None,
                extent: config.extent,
            }),
            configuration,
        })
    }
}

#[cfg(target_os = "horizon")]
impl ShaderModuleInner {
    unsafe fn new_dksh(raw_device: dk::DkDevice, bytes: &[u8]) -> Result<Self, crate::ShaderError> {
        let header = validate_dksh(bytes)?;
        let bindings = parse_shader_bindings(bytes, header)?;
        let control_len = usize::try_from(header.control_sz)
            .map_err(|_| shader_error("deko3d DKSH control section is too large"))?;
        let code_len = usize::try_from(header.code_sz)
            .map_err(|_| shader_error("deko3d DKSH code section is too large"))?;

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
                            bindings,
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
        if desc.primitive.topology != wgt::PrimitiveTopology::TriangleList
            || desc.primitive.strip_index_format.is_some()
            || desc.primitive.unclipped_depth
            || desc.primitive.polygon_mode != wgt::PolygonMode::Fill
            || desc.primitive.conservative
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        if desc.color_targets.len() > 8 {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let mut color_state = dk::DkColorState::defaults();
        let mut color_write_state = dk::DkColorWriteState::defaults();
        let mut blend_states = Vec::with_capacity(desc.color_targets.len());
        for (index, target) in desc.color_targets.iter().enumerate() {
            let Some(target) = target else {
                color_write_state.set_mask(index as u32, 0);
                blend_states.push(dk::DkBlendState::defaults());
                continue;
            };
            if !is_color_attachment_format(target.format) {
                return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
            }
            let (mapped_color_state, blend_state) = map_blend_state(target.blend)?;
            color_state.set_blend_enable(index as u32, mapped_color_state.bits & 1 != 0);
            color_write_state.set_mask(
                index as u32,
                map_color_write_state(target.write_mask).masks & 0xf,
            );
            blend_states.push(blend_state);
        }

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

        let Resource::ShaderModule(vertex_shader) = vertex_stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let Resource::ShaderModule(fragment_shader) = fragment_stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let Resource::PipelineLayout(pipeline_layout) = desc.layout else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };

        let mut vertex_buffer_states = Vec::new();
        let mut vertex_attributes = Vec::new();
        let disabled_attribute = dk::DkVtxAttribState::new(
            0,
            true,
            0,
            dk::DkVtxAttribSize::DkVtxAttribSize_1x32,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
            false,
        );
        for (buffer_id, layout) in vertex_buffers.iter().enumerate() {
            vertex_buffer_states.push(dk::DkVtxBufferState {
                stride: u32::try_from(layout.array_stride)
                    .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                divisor: match layout.step_mode {
                    wgt::VertexStepMode::Vertex => 0,
                    wgt::VertexStepMode::Instance => 1,
                },
            });
            for attribute in layout.attributes {
                let shader_location = usize::try_from(attribute.shader_location)
                    .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
                if shader_location >= 32 {
                    return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
                }
                if vertex_attributes.len() <= shader_location {
                    vertex_attributes.resize(shader_location + 1, disabled_attribute);
                }
                let (size, type_) = map_vertex_format(attribute.format)?;
                vertex_attributes[shader_location] = dk::DkVtxAttribState::new(
                    u32::try_from(buffer_id)
                        .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                    false,
                    u32::try_from(attribute.offset)
                        .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?,
                    size,
                    type_,
                    false,
                );
            }
        }

        let mut rasterizer_state = dk::DkRasterizerState::defaults();
        rasterizer_state.set_cull_mode(match desc.primitive.cull_mode {
            None => dk::DkFace_None,
            Some(wgt::Face::Front) => dk::DkFace_Front,
            Some(wgt::Face::Back) => dk::DkFace_Back,
        });
        rasterizer_state.set_front_face(match desc.primitive.front_face {
            wgt::FrontFace::Ccw => dk::DkFrontFace_CW,
            wgt::FrontFace::Cw => dk::DkFrontFace_CCW,
        });
        let uses_depth_stencil = desc.depth_stencil.is_some();
        let depth_stencil_state = desc
            .depth_stencil
            .as_ref()
            .map(map_depth_stencil_state)
            .transpose()?
            .unwrap_or_else(depth_stencil_disabled_state);
        let mut multisample_state = dk::DkMultisampleState::defaults();
        multisample_state.set_mode(ms_mode);
        multisample_state.set_rasterizer_mode(ms_mode);

        Ok(Self {
            inner: RenderPipelineInnerRaw {
                vertex_shader: vertex_shader.clone(),
                fragment_shader: fragment_shader.clone(),
                primitive: dk::DkPrimitive::DkPrimitive_Triangles,
                vertex_buffers: vertex_buffer_states,
                vertex_attributes,
                rasterizer_state,
                color_state,
                color_write_state,
                blend_states,
                depth_stencil_state,
                multisample_state,
                sample_mask,
                uses_depth_stencil,
                stencil_read_mask: desc
                    .depth_stencil
                    .as_ref()
                    .map_or(0xFF, |depth_stencil| depth_stencil.stencil.read_mask as u8),
                stencil_write_mask: desc
                    .depth_stencil
                    .as_ref()
                    .map_or(0xFF, |depth_stencil| depth_stencil.stencil.write_mask as u8),
                pipeline_layout: pipeline_layout.clone(),
                bind_group_count: pipeline_layout.bind_group_layouts.len(),
                vertex_bindings: vertex_shader.inner.bindings.clone(),
                fragment_bindings: fragment_shader.inner.bindings.clone(),
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

        let Resource::PipelineLayout(pipeline_layout) = desc.layout else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };

        let Resource::ShaderModule(compute_shader) = desc.stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };

        Ok(Self {
            inner: ComputePipelineInnerRaw {
                pipeline_layout: pipeline_layout.clone(),
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
        2 => Ok(dk::DkMsMode::DkMsMode_2x),
        4 => Ok(dk::DkMsMode::DkMsMode_4x),
        8 => Ok(dk::DkMsMode::DkMsMode_8x),
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
fn map_depth_stencil_state_expanded(
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

#[cfg(any(target_os = "horizon", test))]
fn map_stencil_face_state(
    state: &mut dk::DkDepthStencilState,
    front: bool,
    face: wgt::StencilFaceState,
) -> Result<(), crate::PipelineError> {
    let fail = map_stencil_operation(face.fail_op);
    let pass = map_stencil_operation(face.pass_op);
    let depth_fail = map_stencil_operation(face.depth_fail_op);
    let compare = map_compare_function_expanded(face.compare);
    if front {
        state.set_stencil_front(fail, pass, depth_fail, compare);
    } else {
        state.set_stencil_back(fail, pass, depth_fail, compare);
    }
    Ok(())
}

#[cfg(any(target_os = "horizon", test))]
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

#[cfg(any(target_os = "horizon", test))]
fn map_compare_function_expanded(compare: wgt::CompareFunction) -> dk::DkCompareOp {
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
        if map_texture_image_format(color_target.format).is_none() {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let index = u32::try_from(index)
            .map_err(|_| crate::PipelineError::Device(crate::DeviceError::Lost))?;
        color_write_state.set_mask(index, map_color_write_mask(color_target.write_mask));
        let (blend_enable, blend_state) = map_blend_state_expanded(color_target.blend)?;
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
fn map_blend_state_expanded(
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
        if map_texture_image_format(desc.format).is_none()
            || desc.mip_level_count == 0
            || !matches!(desc.sample_count, 1 | 2 | 4 | 8)
            || desc.size.width == 0
            || desc.size.height == 0
            || desc.size.depth_or_array_layers == 0
        {
            return Err(crate::DeviceError::Lost);
        }
        if desc.sample_count > 1
            && (desc.mip_level_count != 1
                || desc.size.depth_or_array_layers != 1
                || desc.dimension != wgt::TextureDimension::D2)
        {
            return Err(crate::DeviceError::Lost);
        }
        let is_depth = matches!(
            desc.format,
            wgt::TextureFormat::Depth16Unorm
                | wgt::TextureFormat::Depth32Float
                | wgt::TextureFormat::Depth24PlusStencil8
                | wgt::TextureFormat::Depth32FloatStencil8
                | wgt::TextureFormat::Stencil8
        );
        if is_depth && !desc.usage.contains(wgt::TextureUses::DEPTH_STENCIL_WRITE) {
            return Err(crate::DeviceError::Lost);
        }

        let mut image_layout_maker = dk::DkImageLayoutMaker::defaults(raw_device);
        image_layout_maker.type_ = match desc.dimension {
            wgt::TextureDimension::D1 if desc.size.depth_or_array_layers == 1 => {
                dk::DkImageType::DkImageType_1D
            }
            wgt::TextureDimension::D1 => dk::DkImageType::DkImageType_1DArray,
            wgt::TextureDimension::D2 if desc.size.depth_or_array_layers == 1 => {
                dk::DkImageType::DkImageType_2D
            }
            wgt::TextureDimension::D2 => dk::DkImageType::DkImageType_2DArray,
            wgt::TextureDimension::D3 => dk::DkImageType::DkImageType_3D,
        };
        image_layout_maker.format =
            map_texture_image_format(desc.format).ok_or(crate::DeviceError::Lost)?;
        image_layout_maker.msMode = map_sample_count(desc.sample_count)?;
        if desc.dimension != wgt::TextureDimension::D3
            && desc
                .usage
                .intersects(wgt::TextureUses::COPY_SRC | wgt::TextureUses::COPY_DST)
        {
            image_layout_maker.flags |= dk::DkImageFlags_Usage2DEngine;
        }
        if desc
            .usage
            .intersects(wgt::TextureUses::COLOR_TARGET | wgt::TextureUses::DEPTH_STENCIL_WRITE)
        {
            image_layout_maker.flags |= dk::DkImageFlags_UsageRender;
        }
        if desc.usage.intersects(
            wgt::TextureUses::STORAGE_READ_ONLY
                | wgt::TextureUses::STORAGE_WRITE_ONLY
                | wgt::TextureUses::STORAGE_READ_WRITE,
        ) {
            image_layout_maker.flags |= dk::DkImageFlags_UsageLoadStore;
        }
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

        let inner = TextureInnerRaw {
            id: trace::resource_id(),
            label: desc.label.map(String::from),
            mem_block,
            image,
            extent: desc.size,
            sample_count: desc.sample_count,
            dimension: desc.dimension,
            format: desc.format,
        };
        trace::record_resource(format_args!(
            "resource kind=texture id={} label={:?} format={:?} dimension={:?} size={}x{}x{} mips={} image_type={:?} allocation={} gpu_addr={:#x}",
            inner.id,
            inner.label,
            inner.format,
            inner.dimension,
            inner.extent.width,
            inner.extent.height,
            inner.extent.depth_or_array_layers,
            desc.mip_level_count,
            image_layout_maker.type_,
            texture_size,
            unsafe { dk::dkMemBlockGetGpuAddr(inner.mem_block) }
        ));
        Ok(Self { inner })
    }
}

#[cfg(target_os = "horizon")]
impl SamplerInner {
    fn new(desc: &crate::SamplerDescriptor) -> DeviceResult<Self> {
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
        if let Some(compare) = desc.compare {
            sampler.compareEnable = true;
            sampler.compareOp = map_compare_function(compare);
        }
        sampler.maxAnisotropy = desc.anisotropy_clamp as f32;
        if let Some(border_color) = desc.border_color {
            let color = match border_color {
                wgt::SamplerBorderColor::TransparentBlack | wgt::SamplerBorderColor::Zero => {
                    [0.0, 0.0, 0.0, 0.0]
                }
                wgt::SamplerBorderColor::OpaqueBlack => [0.0, 0.0, 0.0, 1.0],
                wgt::SamplerBorderColor::OpaqueWhite => [1.0, 1.0, 1.0, 1.0],
            };
            sampler.borderColor = color.map(|value_f| dk::DkSamplerBorderColor { value_f });
        }

        let inner = SamplerInnerRaw {
            id: trace::resource_id(),
            label: desc.label.map(String::from),
            sampler,
        };
        trace::record_resource(format_args!(
            "resource kind=sampler id={} label={:?} address={:?} min={:?} mag={:?} mip={:?}",
            inner.id,
            inner.label,
            desc.address_modes,
            desc.min_filter,
            desc.mag_filter,
            desc.mipmap_filter
        ));
        Ok(Self { inner })
    }
}

#[cfg(target_os = "horizon")]
impl BindGroupInner {
    unsafe fn new(
        raw_device: dk::DkDevice,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
    ) -> DeviceResult<Self> {
        let Resource::BindGroupLayout(kind) = desc.layout else {
            return Err(crate::DeviceError::Lost);
        };
        match kind {
            BindGroupLayoutKind::TextureSamplers { bindings, uniforms } => unsafe {
                Self::new_texture_samplers(raw_device, desc, bindings, uniforms)
            },
            BindGroupLayoutKind::UniformBuffers(bindings) => {
                Self::new_uniform_buffers(desc, bindings)
            }
            BindGroupLayoutKind::Inactive => Ok(Self {
                inner: BindGroupInnerRaw::Inactive,
                id: trace::resource_id(),
                label: desc.label.map(String::from),
                traced_texture_targets: AtomicU64::new(0),
            }),
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
            BindGroupLayoutKind::BufferGroup(entries) => Self::new_buffer_group(desc, entries),
            BindGroupLayoutKind::StorageTexture {
                binding,
                visibility,
                access,
                format,
                view_dimension,
                count,
            } => unsafe {
                Self::new_storage_texture(
                    raw_device,
                    desc,
                    *binding,
                    *visibility,
                    *access,
                    *format,
                    *view_dimension,
                    *count,
                )
            },
        }
    }

    unsafe fn new_texture_samplers(
        raw_device: dk::DkDevice,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        layout_bindings: &[(u32, Option<u32>, u32, wgt::ShaderStages)],
        uniform_layouts: &[(u32, wgt::ShaderStages, bool)],
    ) -> DeviceResult<Self> {
        if layout_bindings.is_empty()
            || layout_bindings
                .iter()
                .map(|(_, _, count, _)| *count as usize)
                .sum::<usize>()
                > DEKO_TEXTURE_SAMPLER_COUNT
            || desc.buffers.len() != uniform_layouts.len()
            || desc.samplers.len()
                != layout_bindings
                    .iter()
                    .filter_map(|(_, sampler, count, _)| sampler.map(|_| *count as usize))
                    .sum::<usize>()
            || desc.textures.len()
                != layout_bindings
                    .iter()
                    .map(|(_, _, count, _)| *count as usize)
                    .sum::<usize>()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len()
                != layout_bindings.len()
                    + layout_bindings
                        .iter()
                        .filter(|(_, sampler, _, _)| sampler.is_some())
                        .count()
                    + uniform_layouts.len()
        {
            return Err(crate::DeviceError::Lost);
        }
        let mut bindings = Vec::with_capacity(desc.textures.len());
        for &(texture_binding, sampler_binding, count, visibility) in layout_bindings {
            let texture_entry = desc
                .entries
                .iter()
                .find(|entry| entry.binding == texture_binding)
                .ok_or(crate::DeviceError::Lost)?;
            if texture_entry.count != count {
                return Err(crate::DeviceError::Lost);
            }
            let texture_resource_index = usize::try_from(texture_entry.resource_index)
                .map_err(|_| crate::DeviceError::Lost)?;
            let sampler_entry = if let Some(sampler_binding) = sampler_binding {
                let sampler_entry = desc
                    .entries
                    .iter()
                    .find(|entry| entry.binding == sampler_binding)
                    .ok_or(crate::DeviceError::Lost)?;
                if sampler_entry.count != count {
                    return Err(crate::DeviceError::Lost);
                }
                Some(sampler_entry)
            } else {
                None
            };
            for array_index in 0..usize::try_from(count).map_err(|_| crate::DeviceError::Lost)? {
                let sampler = if let Some(sampler_entry) = sampler_entry {
                    let sampler_resource_index = usize::try_from(sampler_entry.resource_index)
                        .map_err(|_| crate::DeviceError::Lost)?
                        .checked_add(array_index)
                        .ok_or(crate::DeviceError::Lost)?;
                    let Resource::Sampler(sampler) = desc
                        .samplers
                        .get(sampler_resource_index)
                        .ok_or(crate::DeviceError::Lost)?
                    else {
                        return Err(crate::DeviceError::Lost);
                    };
                    Some(sampler.clone())
                } else {
                    None
                };
                let texture = desc
                    .textures
                    .get(
                        texture_resource_index
                            .checked_add(array_index)
                            .ok_or(crate::DeviceError::Lost)?,
                    )
                    .ok_or(crate::DeviceError::Lost)?;
                if !texture.usage.contains(wgt::TextureUses::RESOURCE) {
                    return Err(crate::DeviceError::Lost);
                }
                let Resource::TextureView {
                    id: view_id,
                    image,
                    format,
                    aspect,
                    view_dimension,
                    owner: Some(owner),
                    ..
                } = texture.view
                else {
                    return Err(crate::DeviceError::Lost);
                };
                if !matches!(
                    aspect,
                    wgt::TextureAspect::All | wgt::TextureAspect::DepthOnly
                ) {
                    return Err(crate::DeviceError::Lost);
                }
                let mut image_descriptor = dk::DkImageDescriptor::zeroed();
                let mut image_view = image.1;
                image_view.format =
                    map_texture_image_format(*format).ok_or(crate::DeviceError::Lost)?;
                if let Some(view_type) = map_texture_view_dimension(*view_dimension) {
                    image_view.type_ = view_type;
                }
                let mut sampler_descriptor = dk::DkSamplerDescriptor::zeroed();
                let default_sampler = dk::DkSampler::defaults();
                let raw_sampler = sampler
                    .as_ref()
                    .map_or(&default_sampler, |sampler| &sampler.inner.sampler);
                unsafe {
                    dk::dkImageDescriptorInitialize(
                        &mut image_descriptor,
                        &image_view,
                        false,
                        false,
                    );
                    dk::dkSamplerDescriptorInitialize(&mut sampler_descriptor, raw_sampler);
                }
                bindings.push(TextureSamplerBinding {
                    texture_binding,
                    visibility,
                    view_id: *view_id,
                    texture: owner.clone(),
                    sampler,
                    image_descriptor,
                    sampler_descriptor,
                });
            }
        }
        let uniforms = Self::uniform_buffer_bindings(desc, uniform_layouts)?;

        let image_descriptor_stride = align_up(
            size_of::<dk::DkImageDescriptor>() as u32,
            dk::DK_IMAGE_DESCRIPTOR_ALIGNMENT,
        );
        let sampler_descriptor_stride = align_up(
            size_of::<dk::DkSamplerDescriptor>() as u32,
            dk::DK_SAMPLER_DESCRIPTOR_ALIGNMENT,
        );
        let descriptor_capacity = DEKO_TEXTURE_SAMPLER_COUNT as u32;
        let descriptor_size = image_descriptor_stride
            .checked_mul(descriptor_capacity)
            .and_then(|size| size.checked_add(sampler_descriptor_stride * descriptor_capacity))
            .ok_or(crate::DeviceError::Lost)?;
        let allocation_size = align_up(descriptor_size, dk::DK_MEMBLOCK_ALIGNMENT);
        let mut mem_block_maker = dk::DkMemBlockMaker::defaults(raw_device, allocation_size);
        mem_block_maker.flags = dk::DkMemBlockFlags_CpuUncached | dk::DkMemBlockFlags_GpuCached;
        let descriptor_mem_block = unsafe { dk::dkMemBlockCreate(&mem_block_maker) };
        if descriptor_mem_block.is_null() {
            return Err(crate::DeviceError::OutOfMemory);
        }
        let descriptor_gpu_addr = unsafe { dk::dkMemBlockGetGpuAddr(descriptor_mem_block) };
        if descriptor_gpu_addr == u64::MAX {
            unsafe { dk::dkMemBlockDestroy(descriptor_mem_block) };
            return Err(crate::DeviceError::Lost);
        }

        let sampler_offset = u64::from(image_descriptor_stride) * u64::from(descriptor_capacity);

        Ok(Self {
            inner: BindGroupInnerRaw::TextureSamplers {
                bindings,
                descriptor_mem_block,
                image_descriptor_gpu_addr: descriptor_gpu_addr,
                sampler_descriptor_gpu_addr: descriptor_gpu_addr + sampler_offset,
                image_descriptor_stride: u64::from(image_descriptor_stride),
                sampler_descriptor_stride: u64::from(sampler_descriptor_stride),
                uniforms,
            },
            id: trace::resource_id(),
            label: desc.label.map(String::from),
            traced_texture_targets: AtomicU64::new(0),
        })
    }

    unsafe fn new_storage_texture(
        raw_device: dk::DkDevice,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
        access: wgt::StorageTextureAccess,
        format: wgt::TextureFormat,
        view_dimension: wgt::TextureViewDimension,
        count: u32,
    ) -> DeviceResult<Self> {
        if !desc.buffers.is_empty()
            || !desc.samplers.is_empty()
            || desc.textures.len() != count as usize
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
        let required_usage = match access {
            wgt::StorageTextureAccess::ReadOnly => wgt::TextureUses::STORAGE_READ_ONLY,
            wgt::StorageTextureAccess::WriteOnly => wgt::TextureUses::STORAGE_WRITE_ONLY,
            wgt::StorageTextureAccess::ReadWrite => wgt::TextureUses::STORAGE_READ_WRITE,
            _ => return Err(crate::DeviceError::Lost),
        };
        let descriptor_stride = align_up(
            size_of::<dk::DkImageDescriptor>() as u32,
            dk::DK_IMAGE_DESCRIPTOR_ALIGNMENT,
        );
        let descriptor_size = descriptor_stride
            .checked_mul(count)
            .ok_or(crate::DeviceError::OutOfMemory)?;
        let allocation_size = align_up(descriptor_size, dk::DK_MEMBLOCK_ALIGNMENT);
        let mut maker = dk::DkMemBlockMaker::defaults(raw_device, allocation_size);
        maker.flags = dk::DkMemBlockFlags_CpuUncached | dk::DkMemBlockFlags_GpuCached;
        let descriptor_mem_block = unsafe { dk::dkMemBlockCreate(&maker) };
        if descriptor_mem_block.is_null() {
            return Err(crate::DeviceError::OutOfMemory);
        }
        let image_descriptor_gpu_addr = unsafe { dk::dkMemBlockGetGpuAddr(descriptor_mem_block) };
        if image_descriptor_gpu_addr == u64::MAX {
            unsafe { dk::dkMemBlockDestroy(descriptor_mem_block) };
            return Err(crate::DeviceError::Lost);
        }
        let mut elements = Vec::with_capacity(count as usize);
        for texture_binding in desc.textures {
            if !texture_binding.usage.contains(required_usage) {
                return Err(crate::DeviceError::Lost);
            }
            let Resource::TextureView {
                image,
                format: view_format,
                view_dimension: actual_dimension,
                owner: Some(texture),
                ..
            } = texture_binding.view
            else {
                return Err(crate::DeviceError::Lost);
            };
            if *view_format != format || *actual_dimension != view_dimension {
                return Err(crate::DeviceError::Lost);
            }
            let mut image_descriptor = dk::DkImageDescriptor::zeroed();
            unsafe {
                dk::dkImageDescriptorInitialize(&mut image_descriptor, &image.1, true, false)
            };
            elements.push(StorageTextureElement {
                texture: texture.clone(),
                image_descriptor,
            });
        }
        Ok(Self {
            inner: BindGroupInnerRaw::StorageTexture(StorageTextureBinding {
                binding,
                visibility,
                elements,
                descriptor_mem_block,
                image_descriptor_gpu_addr,
                image_descriptor_stride: u64::from(descriptor_stride),
            }),
            id: trace::resource_id(),
            label: desc.label.map(String::from),
            traced_texture_targets: AtomicU64::new(0),
        })
    }

    fn new_uniform_buffers(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        layouts: &[(u32, wgt::ShaderStages, bool)],
    ) -> DeviceResult<Self> {
        if layouts.len() > DEKO_UNIFORM_BUFFER_COUNT as usize
            || desc.buffers.len() != layouts.len()
            || !desc.samplers.is_empty()
            || !desc.textures.is_empty()
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != layouts.len()
        {
            return Err(crate::DeviceError::Lost);
        }
        Ok(Self {
            inner: BindGroupInnerRaw::UniformBuffers(Self::uniform_buffer_bindings(desc, layouts)?),
            id: trace::resource_id(),
            label: desc.label.map(String::from),
            traced_texture_targets: AtomicU64::new(0),
        })
    }

    fn uniform_buffer_bindings(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        layouts: &[(u32, wgt::ShaderStages, bool)],
    ) -> DeviceResult<Vec<UniformBufferBinding>> {
        let mut bindings = Vec::with_capacity(layouts.len());
        for &(binding, visibility, has_dynamic_offset) in layouts {
            let Some(entry) = desc.entries.iter().find(|entry| entry.binding == binding) else {
                return Err(crate::DeviceError::Lost);
            };
            if entry.count != 1 {
                return Err(crate::DeviceError::Lost);
            }
            let resource_index =
                usize::try_from(entry.resource_index).map_err(|_| crate::DeviceError::Lost)?;
            let buffer = desc
                .buffers
                .get(resource_index)
                .ok_or(crate::DeviceError::Lost)?;
            if buffer.offset % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0
                || buffer
                    .size
                    .is_some_and(|size| size.get() > u64::from(dk::DK_UNIFORM_BUF_MAX_SIZE))
            {
                return Err(crate::DeviceError::Lost);
            }
            bindings.push(UniformBufferBinding {
                binding,
                visibility,
                has_dynamic_offset,
                buffer: buffer.buffer.clone(),
                offset: buffer.offset,
                size: buffer.size,
            });
        }
        Ok(bindings)
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
            id: trace::resource_id(),
            label: desc.label.map(String::from),
            traced_texture_targets: AtomicU64::new(0),
        })
    }

    fn new_buffer_group(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        layout_entries: &[BufferBindGroupLayoutKind],
    ) -> DeviceResult<Self> {
        let expected_buffers = layout_entries
            .iter()
            .try_fold(0usize, |total, entry| {
                total.checked_add(entry.count() as usize)
            })
            .ok_or(crate::DeviceError::Lost)?;
        if desc.buffers.len() != expected_buffers
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
            if entry.count != layout_entry.count() {
                return Err(crate::DeviceError::Lost);
            }
            let start =
                usize::try_from(entry.resource_index).map_err(|_| crate::DeviceError::Lost)?;
            let count = usize::try_from(entry.count).map_err(|_| crate::DeviceError::Lost)?;
            let Some(resources) = desc
                .buffers
                .get(start..start.checked_add(count).ok_or(crate::DeviceError::Lost)?)
            else {
                return Err(crate::DeviceError::Lost);
            };

            match *layout_entry {
                BufferBindGroupLayoutKind::Uniform {
                    binding,
                    visibility,
                    has_dynamic_offset,
                    count,
                } => {
                    let array = resources
                        .iter()
                        .map(|buffer| {
                            Self::make_uniform_buffer_binding(
                                binding,
                                visibility,
                                has_dynamic_offset,
                                buffer,
                            )
                        })
                        .collect::<DeviceResult<Vec<_>>>()?;
                    if count == 1 {
                        bindings.push(BufferBinding::Uniform(array.into_iter().next().unwrap()));
                    } else {
                        bindings.push(BufferBinding::UniformArray(array));
                    }
                }
                BufferBindGroupLayoutKind::Storage {
                    binding,
                    visibility,
                    read_only,
                    has_dynamic_offset,
                    count,
                } => {
                    let array = resources
                        .iter()
                        .map(|buffer| {
                            Self::make_storage_buffer_binding(
                                binding,
                                visibility,
                                read_only,
                                has_dynamic_offset,
                                buffer,
                            )
                        })
                        .collect::<DeviceResult<Vec<_>>>()?;
                    if count == 1 {
                        bindings.push(BufferBinding::Storage(array.into_iter().next().unwrap()));
                    } else {
                        bindings.push(BufferBinding::StorageArray(array));
                    }
                }
            }
        }

        Ok(Self {
            inner: BindGroupInnerRaw::BufferGroup(bindings),
            id: trace::resource_id(),
            label: desc.label.map(String::from),
            traced_texture_targets: AtomicU64::new(0),
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
        wgt::VertexFormat::Uint32 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_1x32,
            dk::DkVtxAttribType::DkVtxAttribType_Uint,
        )),
        wgt::VertexFormat::Uint32x2 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_2x32,
            dk::DkVtxAttribType::DkVtxAttribType_Uint,
        )),
        wgt::VertexFormat::Uint32x3 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_3x32,
            dk::DkVtxAttribType::DkVtxAttribType_Uint,
        )),
        wgt::VertexFormat::Uint32x4 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_4x32,
            dk::DkVtxAttribType::DkVtxAttribType_Uint,
        )),
        wgt::VertexFormat::Sint32 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_1x32,
            dk::DkVtxAttribType::DkVtxAttribType_Sint,
        )),
        wgt::VertexFormat::Sint32x2 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_2x32,
            dk::DkVtxAttribType::DkVtxAttribType_Sint,
        )),
        wgt::VertexFormat::Sint32x3 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_3x32,
            dk::DkVtxAttribType::DkVtxAttribType_Sint,
        )),
        wgt::VertexFormat::Sint32x4 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_4x32,
            dk::DkVtxAttribType::DkVtxAttribType_Sint,
        )),
        wgt::VertexFormat::Float16 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_1x16,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
        )),
        wgt::VertexFormat::Float16x2 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_2x16,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
        )),
        wgt::VertexFormat::Float16x4 => Ok((
            dk::DkVtxAttribSize::DkVtxAttribSize_4x16,
            dk::DkVtxAttribType::DkVtxAttribType_Float,
        )),
        wgt::VertexFormat::Uint16 | wgt::VertexFormat::Uint16x2 | wgt::VertexFormat::Uint16x4 => {
            Ok((
                match format {
                    wgt::VertexFormat::Uint16 => dk::DkVtxAttribSize::DkVtxAttribSize_1x16,
                    wgt::VertexFormat::Uint16x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x16,
                    _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x16,
                },
                dk::DkVtxAttribType::DkVtxAttribType_Uint,
            ))
        }
        wgt::VertexFormat::Sint16 | wgt::VertexFormat::Sint16x2 | wgt::VertexFormat::Sint16x4 => {
            Ok((
                match format {
                    wgt::VertexFormat::Sint16 => dk::DkVtxAttribSize::DkVtxAttribSize_1x16,
                    wgt::VertexFormat::Sint16x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x16,
                    _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x16,
                },
                dk::DkVtxAttribType::DkVtxAttribType_Sint,
            ))
        }
        wgt::VertexFormat::Unorm16
        | wgt::VertexFormat::Unorm16x2
        | wgt::VertexFormat::Unorm16x4 => Ok((
            match format {
                wgt::VertexFormat::Unorm16 => dk::DkVtxAttribSize::DkVtxAttribSize_1x16,
                wgt::VertexFormat::Unorm16x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x16,
                _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x16,
            },
            dk::DkVtxAttribType::DkVtxAttribType_Unorm,
        )),
        wgt::VertexFormat::Snorm16
        | wgt::VertexFormat::Snorm16x2
        | wgt::VertexFormat::Snorm16x4 => Ok((
            match format {
                wgt::VertexFormat::Snorm16 => dk::DkVtxAttribSize::DkVtxAttribSize_1x16,
                wgt::VertexFormat::Snorm16x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x16,
                _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x16,
            },
            dk::DkVtxAttribType::DkVtxAttribType_Snorm,
        )),
        wgt::VertexFormat::Uint8 | wgt::VertexFormat::Uint8x2 | wgt::VertexFormat::Uint8x4 => Ok((
            match format {
                wgt::VertexFormat::Uint8 => dk::DkVtxAttribSize::DkVtxAttribSize_1x8,
                wgt::VertexFormat::Uint8x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x8,
                _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x8,
            },
            dk::DkVtxAttribType::DkVtxAttribType_Uint,
        )),
        wgt::VertexFormat::Sint8 | wgt::VertexFormat::Sint8x2 | wgt::VertexFormat::Sint8x4 => Ok((
            match format {
                wgt::VertexFormat::Sint8 => dk::DkVtxAttribSize::DkVtxAttribSize_1x8,
                wgt::VertexFormat::Sint8x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x8,
                _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x8,
            },
            dk::DkVtxAttribType::DkVtxAttribType_Sint,
        )),
        wgt::VertexFormat::Unorm8 | wgt::VertexFormat::Unorm8x2 | wgt::VertexFormat::Unorm8x4 => {
            Ok((
                match format {
                    wgt::VertexFormat::Unorm8 => dk::DkVtxAttribSize::DkVtxAttribSize_1x8,
                    wgt::VertexFormat::Unorm8x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x8,
                    _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x8,
                },
                dk::DkVtxAttribType::DkVtxAttribType_Unorm,
            ))
        }
        wgt::VertexFormat::Snorm8 | wgt::VertexFormat::Snorm8x2 | wgt::VertexFormat::Snorm8x4 => {
            Ok((
                match format {
                    wgt::VertexFormat::Snorm8 => dk::DkVtxAttribSize::DkVtxAttribSize_1x8,
                    wgt::VertexFormat::Snorm8x2 => dk::DkVtxAttribSize::DkVtxAttribSize_2x8,
                    _ => dk::DkVtxAttribSize::DkVtxAttribSize_4x8,
                },
                dk::DkVtxAttribType::DkVtxAttribType_Snorm,
            ))
        }
        _ => Err(crate::PipelineError::Device(crate::DeviceError::Lost)),
    }
}

#[cfg(any(target_os = "horizon", test))]
fn map_blend_state(
    blend: Option<wgt::BlendState>,
) -> Result<(dk::DkColorState, dk::DkBlendState), crate::PipelineError> {
    let mut color_state = dk::DkColorState::defaults();
    let mut blend_state = dk::DkBlendState::defaults();
    let Some(blend) = blend else {
        return Ok((color_state, blend_state));
    };
    let (color_src, color_dst) = match blend.color {
        wgt::BlendComponent {
            src_factor: wgt::BlendFactor::SrcAlpha,
            dst_factor: wgt::BlendFactor::OneMinusSrcAlpha,
            operation: wgt::BlendOperation::Add,
        } => (
            dk::DkBlendFactor::DkBlendFactor_SrcAlpha,
            dk::DkBlendFactor::DkBlendFactor_InvSrcAlpha,
        ),
        wgt::BlendComponent {
            src_factor: wgt::BlendFactor::One,
            dst_factor: wgt::BlendFactor::OneMinusSrcAlpha,
            operation: wgt::BlendOperation::Add,
        } => (
            dk::DkBlendFactor::DkBlendFactor_One,
            dk::DkBlendFactor::DkBlendFactor_InvSrcAlpha,
        ),
        wgt::BlendComponent::REPLACE if blend.alpha == wgt::BlendComponent::REPLACE => {
            return Ok((color_state, blend_state));
        }
        _ => return Err(crate::PipelineError::Device(crate::DeviceError::Lost)),
    };
    if blend.alpha
        != (wgt::BlendComponent {
            src_factor: wgt::BlendFactor::One,
            dst_factor: wgt::BlendFactor::OneMinusSrcAlpha,
            operation: wgt::BlendOperation::Add,
        })
    {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }
    color_state.set_blend_enable(0, true);
    blend_state.set_ops(dk::DkBlendOp::DkBlendOp_Add, dk::DkBlendOp::DkBlendOp_Add);
    blend_state.set_factors(
        color_src,
        color_dst,
        dk::DkBlendFactor::DkBlendFactor_One,
        dk::DkBlendFactor::DkBlendFactor_InvSrcAlpha,
    );
    Ok((color_state, blend_state))
}

#[cfg(any(target_os = "horizon", test))]
fn map_color_write_state(mask: wgt::ColorWrites) -> dk::DkColorWriteState {
    let mut result = dk::DkColorWriteState::defaults();
    let mut deko_mask = 0;
    if mask.contains(wgt::ColorWrites::RED) {
        deko_mask |= dk::DkColorMask_R;
    }
    if mask.contains(wgt::ColorWrites::GREEN) {
        deko_mask |= dk::DkColorMask_G;
    }
    if mask.contains(wgt::ColorWrites::BLUE) {
        deko_mask |= dk::DkColorMask_B;
    }
    if mask.contains(wgt::ColorWrites::ALPHA) {
        deko_mask |= dk::DkColorMask_A;
    }
    result.set_mask(0, deko_mask);
    result
}

#[cfg(any(target_os = "horizon", test))]
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
        wgt::TextureFormat::R16Unorm => Some(dk::DkImageFormat::DkImageFormat_R16_Unorm),
        wgt::TextureFormat::R16Snorm => Some(dk::DkImageFormat::DkImageFormat_R16_Snorm),
        wgt::TextureFormat::R16Uint => Some(dk::DkImageFormat::DkImageFormat_R16_Uint),
        wgt::TextureFormat::R16Sint => Some(dk::DkImageFormat::DkImageFormat_R16_Sint),
        wgt::TextureFormat::Rg16Float => Some(dk::DkImageFormat::DkImageFormat_RG16_Float),
        wgt::TextureFormat::Rg16Unorm => Some(dk::DkImageFormat::DkImageFormat_RG16_Unorm),
        wgt::TextureFormat::Rg16Snorm => Some(dk::DkImageFormat::DkImageFormat_RG16_Snorm),
        wgt::TextureFormat::Rg16Uint => Some(dk::DkImageFormat::DkImageFormat_RG16_Uint),
        wgt::TextureFormat::Rg16Sint => Some(dk::DkImageFormat::DkImageFormat_RG16_Sint),
        wgt::TextureFormat::R32Float => Some(dk::DkImageFormat::DkImageFormat_R32_Float),
        wgt::TextureFormat::R32Uint => Some(dk::DkImageFormat::DkImageFormat_R32_Uint),
        wgt::TextureFormat::R32Sint => Some(dk::DkImageFormat::DkImageFormat_R32_Sint),
        wgt::TextureFormat::Rg32Float => Some(dk::DkImageFormat::DkImageFormat_RG32_Float),
        wgt::TextureFormat::Rg32Uint => Some(dk::DkImageFormat::DkImageFormat_RG32_Uint),
        wgt::TextureFormat::Rg32Sint => Some(dk::DkImageFormat::DkImageFormat_RG32_Sint),
        wgt::TextureFormat::Rgb9e5Ufloat => Some(dk::DkImageFormat::DkImageFormat_E5BGR9_Float),
        wgt::TextureFormat::Depth16Unorm => Some(dk::DkImageFormat::DkImageFormat_Z16),
        wgt::TextureFormat::Depth32Float => Some(dk::DkImageFormat::DkImageFormat_ZF32),
        wgt::TextureFormat::Depth24PlusStencil8 => Some(dk::DkImageFormat::DkImageFormat_Z24S8),
        wgt::TextureFormat::Depth32FloatStencil8 => {
            Some(dk::DkImageFormat::DkImageFormat_ZF32_X24S8)
        }
        wgt::TextureFormat::Stencil8 => Some(dk::DkImageFormat::DkImageFormat_S8),
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
        wgt::TextureFormat::Rgba16Float => Some(dk::DkImageFormat::DkImageFormat_RGBA16_Float),
        wgt::TextureFormat::Rgba32Float => Some(dk::DkImageFormat::DkImageFormat_RGBA32_Float),
        wgt::TextureFormat::Rgba32Uint => Some(dk::DkImageFormat::DkImageFormat_RGBA32_Uint),
        wgt::TextureFormat::Rgba32Sint => Some(dk::DkImageFormat::DkImageFormat_RGBA32_Sint),
        _ => None,
    }
}

fn deko3d_texture_format_capabilities(
    format: wgt::TextureFormat,
) -> crate::TextureFormatCapabilities {
    use crate::TextureFormatCapabilities as C;
    let mut capabilities = match format {
        wgt::TextureFormat::Rgba8Unorm
        | wgt::TextureFormat::Rgba8UnormSrgb
        | wgt::TextureFormat::Bgra8Unorm
        | wgt::TextureFormat::Bgra8UnormSrgb
        | wgt::TextureFormat::Rgba16Float => {
            C::SAMPLED | C::COLOR_ATTACHMENT | C::COLOR_ATTACHMENT_BLEND | C::COPY_SRC | C::COPY_DST
        }
        wgt::TextureFormat::Rg16Float
        | wgt::TextureFormat::Rgba8Uint
        | wgt::TextureFormat::Rgba8Sint => {
            C::SAMPLED | C::COLOR_ATTACHMENT | C::COPY_SRC | C::COPY_DST
        }
        wgt::TextureFormat::Depth16Unorm | wgt::TextureFormat::Depth32Float => {
            C::SAMPLED | C::DEPTH_STENCIL_ATTACHMENT | C::COPY_SRC | C::COPY_DST
        }
        wgt::TextureFormat::Depth24PlusStencil8 | wgt::TextureFormat::Depth32FloatStencil8 => {
            C::SAMPLED | C::DEPTH_STENCIL_ATTACHMENT
        }
        wgt::TextureFormat::Stencil8 => C::DEPTH_STENCIL_ATTACHMENT | C::COPY_SRC | C::COPY_DST,
        wgt::TextureFormat::R8Unorm
        | wgt::TextureFormat::Rg8Unorm
        | wgt::TextureFormat::R8Uint
        | wgt::TextureFormat::R8Sint
        | wgt::TextureFormat::Rg8Uint
        | wgt::TextureFormat::Rg8Sint
        | wgt::TextureFormat::R8Snorm
        | wgt::TextureFormat::Rg8Snorm
        | wgt::TextureFormat::Rgba8Snorm
        | wgt::TextureFormat::R16Float
        | wgt::TextureFormat::R16Unorm
        | wgt::TextureFormat::R16Snorm
        | wgt::TextureFormat::R16Uint
        | wgt::TextureFormat::R16Sint
        | wgt::TextureFormat::Rg16Unorm
        | wgt::TextureFormat::Rg16Snorm
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
        | wgt::TextureFormat::Rgb9e5Ufloat => C::SAMPLED | C::COPY_SRC | C::COPY_DST,
        _ => C::empty(),
    };
    if matches!(
        format,
        wgt::TextureFormat::R8Unorm
            | wgt::TextureFormat::Rg8Unorm
            | wgt::TextureFormat::R32Float
            | wgt::TextureFormat::R32Uint
            | wgt::TextureFormat::R32Sint
            | wgt::TextureFormat::Rgba8Unorm
            | wgt::TextureFormat::Rgba8Uint
            | wgt::TextureFormat::Rgba8Sint
            | wgt::TextureFormat::Rgba16Float
            | wgt::TextureFormat::Rgba16Uint
            | wgt::TextureFormat::Rgba16Sint
            | wgt::TextureFormat::Rgba32Float
            | wgt::TextureFormat::Rgba32Uint
            | wgt::TextureFormat::Rgba32Sint
    ) {
        capabilities |= C::STORAGE_READ_ONLY | C::STORAGE_WRITE_ONLY | C::STORAGE_READ_WRITE;
    }
    if capabilities.intersects(C::COLOR_ATTACHMENT | C::DEPTH_STENCIL_ATTACHMENT) {
        capabilities |= C::MULTISAMPLE_X2 | C::MULTISAMPLE_X4 | C::MULTISAMPLE_X8;
    }
    if capabilities.contains(C::COLOR_ATTACHMENT) {
        capabilities |= C::MULTISAMPLE_RESOLVE;
    }
    capabilities
}

pub(super) fn is_color_attachment_format(format: wgt::TextureFormat) -> bool {
    deko3d_texture_format_capabilities(format)
        .contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT)
}

#[cfg(any(target_os = "horizon", test))]
fn map_depth_stencil_state(
    state: &wgt::DepthStencilState,
) -> Result<dk::DkDepthStencilState, crate::PipelineError> {
    if !matches!(
        state.format,
        wgt::TextureFormat::Depth16Unorm
            | wgt::TextureFormat::Depth32Float
            | wgt::TextureFormat::Depth24PlusStencil8
            | wgt::TextureFormat::Depth32FloatStencil8
            | wgt::TextureFormat::Stencil8
    ) || state.bias != wgt::DepthBiasState::default()
    {
        return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
    }
    let mut result = dk::DkDepthStencilState::defaults();
    result.set_depth_test_enable(state.is_depth_enabled());
    result.set_depth_write_enable(state.depth_write_enabled.unwrap_or(false));
    result.set_depth_compare_op(
        match state.depth_compare.unwrap_or(wgt::CompareFunction::Always) {
            wgt::CompareFunction::Never => dk::DkCompareOp::DkCompareOp_Never,
            wgt::CompareFunction::Less => dk::DkCompareOp::DkCompareOp_Less,
            wgt::CompareFunction::Equal => dk::DkCompareOp::DkCompareOp_Equal,
            wgt::CompareFunction::LessEqual => dk::DkCompareOp::DkCompareOp_Lequal,
            wgt::CompareFunction::Greater => dk::DkCompareOp::DkCompareOp_Greater,
            wgt::CompareFunction::NotEqual => dk::DkCompareOp::DkCompareOp_NotEqual,
            wgt::CompareFunction::GreaterEqual => dk::DkCompareOp::DkCompareOp_Gequal,
            wgt::CompareFunction::Always => dk::DkCompareOp::DkCompareOp_Always,
        },
    );
    if state.stencil.is_enabled() {
        result.set_stencil_test_enable(true);
        map_stencil_face_state(&mut result, true, state.stencil.front)?;
        map_stencil_face_state(&mut result, false, state.stencil.back)?;
    } else {
        result.set_stencil_test_enable(false);
    }
    Ok(result)
}

#[cfg(any(target_os = "horizon", test))]
fn depth_stencil_disabled_state() -> dk::DkDepthStencilState {
    let mut state = dk::DkDepthStencilState::defaults();
    state.set_depth_test_enable(false);
    state.set_depth_write_enable(false);
    state
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
        wgt::AddressMode::ClampToBorder => Ok(dk::DkWrapMode::DkWrapMode_ClampToBorder),
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
fn map_texture_view_dimension(dimension: wgt::TextureViewDimension) -> Option<dk::DkImageType> {
    match dimension {
        wgt::TextureViewDimension::D1 => Some(dk::DkImageType::DkImageType_1D),
        wgt::TextureViewDimension::D2 => None,
        wgt::TextureViewDimension::D3 => Some(dk::DkImageType::DkImageType_3D),
        wgt::TextureViewDimension::D2Array => Some(dk::DkImageType::DkImageType_2DArray),
        wgt::TextureViewDimension::Cube => Some(dk::DkImageType::DkImageType_Cubemap),
        wgt::TextureViewDimension::CubeArray => Some(dk::DkImageType::DkImageType_CubemapArray),
    }
}

#[cfg(target_os = "horizon")]
fn configure_raw_image_view(
    mut image: RawImage,
    format: wgt::TextureFormat,
    dimension: wgt::TextureViewDimension,
    range: wgt::ImageSubresourceRange,
) -> DeviceResult<RawImage> {
    image.1.format = map_texture_image_format(format).ok_or(crate::DeviceError::Lost)?;
    if let Some(view_type) = map_texture_view_dimension(dimension) {
        image.1.type_ = view_type;
    }
    if dimension != wgt::TextureViewDimension::D3 {
        image.1.layerOffset =
            u16::try_from(range.base_array_layer).map_err(|_| crate::DeviceError::Lost)?;
        image.1.layerCount = range
            .array_layer_count
            .map(u16::try_from)
            .transpose()
            .map_err(|_| crate::DeviceError::Lost)?
            .unwrap_or(0);
    }
    image.1.mipLevelOffset =
        u8::try_from(range.base_mip_level).map_err(|_| crate::DeviceError::Lost)?;
    image.1.mipLevelCount = range
        .mip_level_count
        .map(u8::try_from)
        .transpose()
        .map_err(|_| crate::DeviceError::Lost)?
        .unwrap_or(0);
    Ok(image)
}

#[cfg(not(target_os = "horizon"))]
fn configure_raw_image_view(
    image: RawImage,
    _format: wgt::TextureFormat,
    _dimension: wgt::TextureViewDimension,
    _range: wgt::ImageSubresourceRange,
) -> DeviceResult<RawImage> {
    Ok(image)
}

fn supported_bind_group_layout_kind(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    if let [entry] = entries {
        let count = entry.count.map(core::num::NonZeroU32::get).unwrap_or(1);
        if count <= DEKO_STORAGE_TEXTURE_COUNT
            && !entry.visibility.is_empty()
            && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                .contains(entry.visibility)
        {
            if let wgt::BindingType::StorageTexture {
                access,
                format,
                view_dimension,
            } = entry.ty
            {
                let required = match access {
                    wgt::StorageTextureAccess::ReadOnly => {
                        crate::TextureFormatCapabilities::STORAGE_READ_ONLY
                    }
                    wgt::StorageTextureAccess::WriteOnly => {
                        crate::TextureFormatCapabilities::STORAGE_WRITE_ONLY
                    }
                    wgt::StorageTextureAccess::ReadWrite => {
                        crate::TextureFormatCapabilities::STORAGE_READ_WRITE
                    }
                    _ => return None,
                };
                if matches!(
                    access,
                    wgt::StorageTextureAccess::ReadOnly
                        | wgt::StorageTextureAccess::WriteOnly
                        | wgt::StorageTextureAccess::ReadWrite
                ) && deko3d_texture_format_capabilities(format).contains(required)
                    && matches!(
                        view_dimension,
                        wgt::TextureViewDimension::D2 | wgt::TextureViewDimension::D3
                    )
                {
                    return Some(BindGroupLayoutKind::StorageTexture {
                        binding: entry.binding,
                        visibility: entry.visibility,
                        access,
                        format,
                        view_dimension,
                        count,
                    });
                }
            }
        }
    }

    if !entries.is_empty() {
        let mut buffer_entries = entries
            .iter()
            .copied()
            .map(supported_buffer_binding_layout_kind)
            .collect::<Option<Vec<_>>>();
        if let Some(buffer_entries) = buffer_entries.as_mut() {
            buffer_entries.sort_unstable_by_key(|entry| entry.binding());
            if buffer_entries
                .windows(2)
                .any(|pair| pair[0].binding() == pair[1].binding())
            {
                return None;
            }
            if buffer_entries.len() == 1 {
                if let BufferBindGroupLayoutKind::Storage {
                    binding,
                    visibility,
                    read_only,
                    has_dynamic_offset,
                    count: 1,
                } = buffer_entries[0]
                {
                    return Some(BindGroupLayoutKind::StorageBuffer {
                        binding,
                        visibility,
                        read_only,
                        has_dynamic_offset,
                    });
                }
            }
            if buffer_entries
                .iter()
                .all(|entry| matches!(entry, BufferBindGroupLayoutKind::Uniform { count: 1, .. }))
            {
                let uniforms = buffer_entries
                    .iter()
                    .map(|entry| match *entry {
                        BufferBindGroupLayoutKind::Uniform {
                            binding,
                            visibility,
                            has_dynamic_offset,
                            count: 1,
                        } => (binding, visibility, has_dynamic_offset),
                        _ => unreachable!(),
                    })
                    .collect();
                return Some(BindGroupLayoutKind::UniformBuffers(uniforms));
            }
            return Some(BindGroupLayoutKind::BufferGroup(buffer_entries.clone()));
        }
    }

    let mut texture_bindings = Vec::new();
    let mut sampler_bindings = Vec::new();
    let mut uniforms = Vec::new();
    let mut occupied_bindings = Vec::new();
    let mut storage = None;
    for entry in entries {
        if occupied_bindings.contains(&entry.binding) {
            return None;
        }
        occupied_bindings.push(entry.binding);
        let count = entry.count.map(core::num::NonZeroU32::get).unwrap_or(1);
        match entry.ty {
            wgt::BindingType::Texture {
                sample_type:
                    wgt::TextureSampleType::Float { .. }
                    | wgt::TextureSampleType::Depth
                    | wgt::TextureSampleType::Sint
                    | wgt::TextureSampleType::Uint,
                view_dimension:
                    wgt::TextureViewDimension::D2
                    | wgt::TextureViewDimension::D3
                    | wgt::TextureViewDimension::D2Array
                    | wgt::TextureViewDimension::Cube
                    | wgt::TextureViewDimension::CubeArray,
                multisampled: false,
            } if !entry.visibility.is_empty()
                && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                    .contains(entry.visibility) =>
            {
                texture_bindings.push((entry.binding, count, entry.visibility));
            }
            wgt::BindingType::Sampler(
                wgt::SamplerBindingType::Filtering
                | wgt::SamplerBindingType::NonFiltering
                | wgt::SamplerBindingType::Comparison,
            ) if !entry.visibility.is_empty()
                && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                    .contains(entry.visibility) =>
            {
                sampler_bindings.push((entry.binding, count, entry.visibility));
            }
            wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Uniform,
                has_dynamic_offset,
                min_binding_size,
            } if !entry.visibility.is_empty()
                && min_binding_size.is_none_or(|size| size.get() <= DEKO_UNIFORM_BUF_MAX_SIZE) =>
            {
                if count != 1 {
                    return None;
                }
                uniforms.push((entry.binding, entry.visibility, has_dynamic_offset));
            }
            wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Storage { read_only },
                has_dynamic_offset,
                ..
            } if entries.len() == 1
                && count == 1
                && !entry.visibility.is_empty()
                && (wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE)
                    .contains(entry.visibility)
                && entry.binding < DEKO_STORAGE_BUFFER_COUNT =>
            {
                storage = Some(BindGroupLayoutKind::StorageBuffer {
                    binding: entry.binding,
                    visibility: entry.visibility,
                    read_only,
                    has_dynamic_offset,
                });
            }
            _ => return None,
        }
    }
    if let Some(storage) = storage {
        return Some(storage);
    }
    if texture_bindings.is_empty() {
        return (uniforms.len() == entries.len())
            .then_some(BindGroupLayoutKind::UniformBuffers(uniforms));
    }
    texture_bindings.sort_unstable_by_key(|(binding, _, _)| *binding);
    sampler_bindings.sort_unstable_by_key(|(binding, _, _)| *binding);
    uniforms.sort_unstable_by_key(|&(binding, _, _)| binding);
    if texture_bindings
        .iter()
        .map(|(_, count, _)| *count as usize)
        .sum::<usize>()
        > DEKO_TEXTURE_SAMPLER_COUNT
        || uniforms.len() > DEKO_UNIFORM_BUFFER_COUNT as usize
    {
        return None;
    }
    let bindings = texture_bindings
        .into_iter()
        .map(|(texture, count, visibility)| {
            let sampler = sampler_bindings
                .iter()
                .find(|&&(sampler, sampler_count, sampler_visibility)| {
                    sampler == texture + 1
                        && sampler_count == count
                        && sampler_visibility == visibility
                })
                .map(|&(sampler, _, _)| sampler);
            (texture, sampler, count, visibility)
        })
        .collect::<Vec<_>>();
    if sampler_bindings.iter().any(|(sampler, count, visibility)| {
        !bindings
            .iter()
            .any(|(_, paired, paired_count, paired_visibility)| {
                paired == &Some(*sampler)
                    && paired_count == count
                    && paired_visibility == visibility
            })
    }) {
        return None;
    }
    Some(BindGroupLayoutKind::TextureSamplers { bindings, uniforms })
}

fn supported_buffer_binding_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<BufferBindGroupLayoutKind> {
    if entry.visibility.is_empty() {
        return None;
    }
    let count = entry.count.map(core::num::NonZeroU32::get).unwrap_or(1);
    let supported_stages = wgt::ShaderStages::VERTEX_FRAGMENT | wgt::ShaderStages::COMPUTE;
    if !supported_stages.contains(entry.visibility) {
        return None;
    }
    match entry.ty {
        wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Uniform,
            has_dynamic_offset,
            min_binding_size,
        } if entry.binding.checked_add(count)? <= DEKO_UNIFORM_BUFFER_COUNT
            && !(has_dynamic_offset && count > 1)
            && min_binding_size.is_none_or(|size| size.get() <= DEKO_UNIFORM_BUF_MAX_SIZE) =>
        {
            Some(BufferBindGroupLayoutKind::Uniform {
                binding: entry.binding,
                visibility: entry.visibility,
                has_dynamic_offset,
                count,
            })
        }
        wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Storage { read_only },
            has_dynamic_offset,
            ..
        } if entry.binding.checked_add(count)? <= DEKO_STORAGE_BUFFER_COUNT
            && !(has_dynamic_offset && count > 1) =>
        {
            Some(BufferBindGroupLayoutKind::Storage {
                binding: entry.binding,
                visibility: entry.visibility,
                read_only,
                has_dynamic_offset,
                count,
            })
        }
        _ => None,
    }
}

#[cfg(any(target_os = "horizon", test))]
fn shader_error(message: &'static str) -> crate::ShaderError {
    crate::ShaderError::Compilation(String::from(message))
}

#[cfg(all(test, deko3d))]
mod tests {
    use super::*;

    const HEADER_SIZE: usize = size_of::<DkshHeader>();
    const PROGRAM_SIZE: usize = size_of::<DkshProgramHeader>();
    const FIXTURE: &[u8] = include_bytes!("test-data/deko-basic-color_fsh.dksh");
    const FIXTURE_SHA256: &str = "eea40963451820c1a6b7ea6834ac579989b37f5badab3740f38ba397d35df28d";

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + size_of::<u32>()].try_into().unwrap())
    }

    fn valid_dksh() -> Vec<u8> {
        let control_size = 256;
        let code_size = 256;
        let mut bytes = vec![0; control_size + code_size];
        put_u32(&mut bytes, 0, DKSH_MAGIC);
        put_u32(&mut bytes, 4, HEADER_SIZE as u32);
        put_u32(&mut bytes, 8, control_size as u32);
        put_u32(&mut bytes, 12, code_size as u32);
        put_u32(&mut bytes, 16, HEADER_SIZE as u32);
        put_u32(&mut bytes, 20, 1);
        put_u32(&mut bytes, HEADER_SIZE, 0);
        put_u32(&mut bytes, HEADER_SIZE + 4, 0);
        put_u32(&mut bytes, HEADER_SIZE + 8, 1);
        bytes
    }

    #[test]
    fn dksh_validation_accepts_a_structurally_valid_container() {
        assert!(validate_dksh(&valid_dksh()).is_ok());
    }

    #[test]
    fn dksh_validation_accepts_the_public_deko_basic_fragment_fixture() {
        assert_eq!(FIXTURE.len(), 512, "fixture sha256={FIXTURE_SHA256}");
        assert!(validate_dksh(FIXTURE).is_ok());
    }

    #[test]
    fn opaque_textured_formats_and_mesh_attributes_are_mapped() {
        assert_eq!(
            map_texture_image_format(wgt::TextureFormat::Rgba8Unorm),
            Some(dk::DkImageFormat::DkImageFormat_RGBA8_Unorm)
        );
        assert_eq!(
            map_texture_image_format(wgt::TextureFormat::Rgba8UnormSrgb),
            Some(dk::DkImageFormat::DkImageFormat_RGBA8_Unorm_sRGB)
        );
        assert_eq!(
            map_texture_image_format(wgt::TextureFormat::Depth32Float),
            Some(dk::DkImageFormat::DkImageFormat_ZF32)
        );
        assert_eq!(
            map_texture_image_format(wgt::TextureFormat::Bgra8Unorm),
            Some(dk::DkImageFormat::DkImageFormat_BGRA8_Unorm)
        );
        assert_eq!(
            map_texture_image_format(wgt::TextureFormat::Depth24PlusStencil8),
            Some(dk::DkImageFormat::DkImageFormat_Z24S8)
        );
        for format in [
            wgt::VertexFormat::Float32x2,
            wgt::VertexFormat::Float32x3,
            wgt::VertexFormat::Float32x4,
            wgt::VertexFormat::Float16x2,
            wgt::VertexFormat::Float16x4,
            wgt::VertexFormat::Unorm8x4,
            wgt::VertexFormat::Snorm8x4,
            wgt::VertexFormat::Uint16x4,
            wgt::VertexFormat::Sint16x4,
            wgt::VertexFormat::Unorm16x4,
            wgt::VertexFormat::Snorm16x4,
            wgt::VertexFormat::Uint32,
            wgt::VertexFormat::Uint32x2,
            wgt::VertexFormat::Uint32x3,
            wgt::VertexFormat::Uint32x4,
        ] {
            assert!(map_vertex_format(format).is_ok(), "{format:?}");
        }
        assert!(map_vertex_format(wgt::VertexFormat::Float64x4).is_err());
        assert!(map_depth_stencil_state(&wgt::DepthStencilState {
            format: wgt::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgt::CompareFunction::LessEqual),
            stencil: wgt::StencilState::default(),
            bias: wgt::DepthBiasState::default(),
        })
        .is_ok());
        assert!(map_depth_stencil_state(&wgt::DepthStencilState {
            format: wgt::TextureFormat::Depth24Plus,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgt::CompareFunction::Less),
            stencil: wgt::StencilState::default(),
            bias: wgt::DepthBiasState::default(),
        })
        .is_err());
        let disabled = depth_stencil_disabled_state();
        assert_eq!(disabled.depth_bits & 0b11, 0);
    }

    #[test]
    fn supported_color_targets_report_expected_capabilities() {
        for format in [
            wgt::TextureFormat::Rgba8Unorm,
            wgt::TextureFormat::Rgba8UnormSrgb,
            wgt::TextureFormat::Rgba16Float,
        ] {
            let capabilities = unsafe {
                <Adapter as crate::Adapter>::texture_format_capabilities(&Adapter, format)
            };
            assert!(
                capabilities.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT_BLEND),
                "{format:?} must support WebGPU color blending"
            );
        }
        let rg16 = unsafe {
            <Adapter as crate::Adapter>::texture_format_capabilities(
                &Adapter,
                wgt::TextureFormat::Rg16Float,
            )
        };
        assert!(rg16.contains(crate::TextureFormatCapabilities::SAMPLED));
        assert!(rg16.contains(crate::TextureFormatCapabilities::COLOR_ATTACHMENT));

        let rgba32 = unsafe {
            <Adapter as crate::Adapter>::texture_format_capabilities(
                &Adapter,
                wgt::TextureFormat::Rgba32Float,
            )
        };
        assert!(rgba32.contains(crate::TextureFormatCapabilities::SAMPLED));
        assert!(rgba32.contains(crate::TextureFormatCapabilities::COPY_DST));
    }

    #[test]
    fn ui_blend_and_color_write_states_are_conservative() {
        let (straight, _) = map_blend_state(Some(wgt::BlendState::ALPHA_BLENDING)).unwrap();
        let (premultiplied, _) =
            map_blend_state(Some(wgt::BlendState::PREMULTIPLIED_ALPHA_BLENDING)).unwrap();
        assert_ne!(straight.bits & 1, 0);
        assert_ne!(premultiplied.bits & 1, 0);
        assert!(map_blend_state(Some(wgt::BlendState {
            color: wgt::BlendComponent::REPLACE,
            alpha: wgt::BlendComponent::OVER,
        }))
        .is_err());
        let writes = map_color_write_state(wgt::ColorWrites::RED | wgt::ColorWrites::ALPHA);
        assert_eq!(writes.masks & 0xF, dk::DkColorMask_R | dk::DkColorMask_A);
    }

    #[test]
    fn static_bind_group_layouts_map_multiple_physical_slots() {
        let texture = |binding| wgt::BindGroupLayoutEntry {
            binding,
            visibility: wgt::ShaderStages::FRAGMENT,
            ty: wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { filterable: true },
                view_dimension: wgt::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler = |binding| wgt::BindGroupLayoutEntry {
            binding,
            visibility: wgt::ShaderStages::FRAGMENT,
            ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Filtering),
            count: None,
        };
        let uniform = |binding| wgt::BindGroupLayoutEntry {
            binding,
            visibility: wgt::ShaderStages::VERTEX_FRAGMENT,
            ty: wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = [
            texture(0),
            sampler(1),
            texture(2),
            sampler(3),
            uniform(8),
            uniform(9),
        ];
        let kind = supported_bind_group_layout_kind(&layout).unwrap();
        let BindGroupLayoutKind::TextureSamplers { bindings, uniforms } = kind else {
            panic!("expected texture/sampler layout");
        };
        assert_eq!(
            bindings,
            vec![
                (0, Some(1), 1, wgt::ShaderStages::FRAGMENT),
                (2, Some(3), 1, wgt::ShaderStages::FRAGMENT),
            ]
        );
        assert_eq!(uniforms.len(), 2);
        assert!(supported_bind_group_layout_kind(&[texture(0), sampler(3)]).is_none());
        assert!(supported_bind_group_layout_kind(&[texture(0), uniform(0), sampler(1)]).is_none());
        let too_many = (0..=DEKO_TEXTURE_SAMPLER_COUNT as u32)
            .flat_map(|index| [texture(index * 2), sampler(index * 2 + 1)])
            .collect::<Vec<_>>();
        assert!(supported_bind_group_layout_kind(&too_many).is_none());
        assert!(supported_bind_group_layout_kind(&[uniform(0), uniform(1)]).is_some());
        assert!(supported_bind_group_layout_kind(&[uniform(100)]).is_some());

        let texture_3d = wgt::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgt::ShaderStages::FRAGMENT,
            ty: wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { filterable: true },
                view_dimension: wgt::TextureViewDimension::D3,
                multisampled: false,
            },
            count: None,
        };
        let dynamic_uniform = wgt::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgt::ShaderStages::FRAGMENT,
            ty: wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            count: None,
        };
        assert!(supported_bind_group_layout_kind(&[
            texture(0),
            sampler(1),
            dynamic_uniform,
            texture_3d,
            sampler(4),
        ])
        .is_some());
    }

    #[test]
    fn buffer_binding_arrays_reserve_consecutive_slots() {
        let entry = wgt::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgt::ShaderStages::COMPUTE,
            ty: wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: core::num::NonZeroU32::new(4),
        };
        let Some(BindGroupLayoutKind::BufferGroup(entries)) =
            supported_bind_group_layout_kind(&[entry])
        else {
            panic!("expected a buffer array layout");
        };
        assert!(matches!(
            entries.as_slice(),
            [BufferBindGroupLayoutKind::Storage {
                binding: 3,
                count: 4,
                ..
            }]
        ));

        let mut dynamic = entry;
        dynamic.ty = wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: true,
            min_binding_size: None,
        };
        assert!(supported_bind_group_layout_kind(&[dynamic]).is_none());
    }

    #[test]
    fn sampled_texture_and_sampler_arrays_share_a_physical_range() {
        let count = core::num::NonZeroU32::new(3);
        let entries = [
            wgt::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgt::ShaderStages::FRAGMENT,
                ty: wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Float { filterable: true },
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: false,
                },
                count,
            },
            wgt::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgt::ShaderStages::FRAGMENT,
                ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Filtering),
                count,
            },
        ];
        let Some(BindGroupLayoutKind::TextureSamplers { bindings, .. }) =
            supported_bind_group_layout_kind(&entries)
        else {
            panic!("expected texture array layout");
        };
        assert_eq!(bindings, vec![(4, Some(5), 3, wgt::ShaderStages::FRAGMENT)]);
    }

    #[test]
    fn storage_texture_layout_accepts_r32_compute_images() {
        let layout = supported_bind_group_layout_kind(&[wgt::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgt::ShaderStages::COMPUTE,
            ty: wgt::BindingType::StorageTexture {
                access: wgt::StorageTextureAccess::ReadWrite,
                format: wgt::TextureFormat::R32Float,
                view_dimension: wgt::TextureViewDimension::D2,
            },
            count: None,
        }]);
        assert!(matches!(
            layout,
            Some(BindGroupLayoutKind::StorageTexture {
                binding: 3,
                access: wgt::StorageTextureAccess::ReadWrite,
                ..
            })
        ));
        let array = supported_bind_group_layout_kind(&[wgt::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgt::ShaderStages::COMPUTE,
            ty: wgt::BindingType::StorageTexture {
                access: wgt::StorageTextureAccess::WriteOnly,
                format: wgt::TextureFormat::Rgba8Unorm,
                view_dimension: wgt::TextureViewDimension::D2,
            },
            count: core::num::NonZeroU32::new(4),
        }]);
        assert!(matches!(
            array,
            Some(BindGroupLayoutKind::StorageTexture { count: 4, .. })
        ));
    }

    #[test]
    fn advertises_pipeline_cache_support() {
        assert!(supported_features().contains(wgt::Features::PIPELINE_CACHE));
    }

    #[test]
    fn timestamp_and_occlusion_query_ranges_are_checked() {
        for query_type in [wgt::QueryType::Timestamp, wgt::QueryType::Occlusion] {
            let query_set = QuerySetInner {
                query_type,
                count: 2,
            };
            assert!(query_set.validate_range(0..2).is_ok());
            assert!(query_set.validate_range(2..2).is_ok());
            assert!(query_set.validate_range(1..3).is_err());
        }
    }

    #[test]
    fn dksh_validation_rejects_truncated_and_corrupt_headers() {
        assert!(validate_dksh(&valid_dksh()[..HEADER_SIZE - 1]).is_err());

        let mut invalid_magic = valid_dksh();
        put_u32(&mut invalid_magic, 0, 0);
        assert!(validate_dksh(&invalid_magic).is_err());

        let mut invalid_lengths = valid_dksh();
        put_u32(&mut invalid_lengths, 8, 128);
        assert!(validate_dksh(&invalid_lengths).is_err());
    }

    #[test]
    fn dksh_validation_rejects_malformed_program_entries() {
        let mut table_out_of_bounds = valid_dksh();
        put_u32(&mut table_out_of_bounds, 20, 4);
        assert!(validate_dksh(&table_out_of_bounds).is_err());

        let mut invalid_stage = valid_dksh();
        put_u32(
            &mut invalid_stage,
            HEADER_SIZE,
            DKSH_PROGRAM_TYPE_COMPUTE + 1,
        );
        assert!(validate_dksh(&invalid_stage).is_err());

        let mut invalid_entrypoint = valid_dksh();
        put_u32(&mut invalid_entrypoint, HEADER_SIZE + 4, 256);
        assert!(validate_dksh(&invalid_entrypoint).is_err());

        let mut invalid_constant_buffer = valid_dksh();
        put_u32(&mut invalid_constant_buffer, HEADER_SIZE + 12, 255);
        put_u32(&mut invalid_constant_buffer, HEADER_SIZE + 16, 2);
        assert!(validate_dksh(&invalid_constant_buffer).is_err());

        assert_eq!(PROGRAM_SIZE, 64);
    }

    #[test]
    fn dksh_validation_rejects_every_structural_fixture_mutation() {
        let control_size = u32_at(FIXTURE, 8);
        let code_size = u32_at(FIXTURE, 12);
        let program_offset = u32_at(FIXTURE, 16) as usize;

        for (offset, value) in [
            (0, 0),
            (4, 0),
            (8, control_size - 1),
            (12, code_size - 1),
            (16, control_size - 4),
            (20, 2),
        ] {
            let mut mutated = FIXTURE.to_vec();
            put_u32(&mut mutated, offset, value);
            assert!(validate_dksh(&mutated).is_err(), "header offset {offset}");
        }

        for (offset, value) in [
            (program_offset, DKSH_PROGRAM_TYPE_COMPUTE + 1),
            (program_offset + 4, code_size),
        ] {
            let mut mutated = FIXTURE.to_vec();
            put_u32(&mut mutated, offset, value);
            assert!(validate_dksh(&mutated).is_err(), "program offset {offset}");
        }

        let mut invalid_constant_buffer = FIXTURE.to_vec();
        put_u32(&mut invalid_constant_buffer, program_offset + 12, code_size);
        put_u32(&mut invalid_constant_buffer, program_offset + 16, 1);
        assert!(validate_dksh(&invalid_constant_buffer).is_err());

        assert!(validate_dksh(&FIXTURE[..FIXTURE.len() - 1]).is_err());
    }

    #[cfg(not(target_os = "horizon"))]
    #[test]
    fn default_switch_surface_rejects_the_forced_host_path() {
        assert!(Instance.create_default_surface().is_err());
    }

    #[test]
    fn default_surface_lease_is_exclusive_and_released() {
        let lease = DefaultSurfaceLease::acquire().unwrap();
        assert!(DefaultSurfaceLease::acquire().is_err());
        drop(lease);
        assert!(DefaultSurfaceLease::acquire().is_ok());
    }

    #[test]
    fn surface_ids_are_monotonic_and_nonzero() {
        let first = NEXT_DEFAULT_SURFACE_ID.fetch_add(1, Ordering::Relaxed);
        let second = NEXT_DEFAULT_SURFACE_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(first, 0);
        assert!(second > first);
    }

    #[cfg(not(target_os = "horizon"))]
    #[test]
    fn surface_view_keeps_the_replaced_generation_alive() {
        let generation = Arc::new(SurfaceState {
            inner: Mutex::new(SurfaceStateInner),
            configuration: 1,
        });
        let weak_generation = Arc::downgrade(&generation);
        let view = Resource::surface_texture_view(
            RawImage,
            wgt::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            generation.clone(),
        );
        let current = Mutex::new(Some(generation.clone()));
        let replacement = Arc::new(SurfaceState {
            inner: Mutex::new(SurfaceStateInner),
            configuration: 2,
        });
        let retired = current.lock().unwrap().replace(replacement);
        drop(retired);
        drop(generation);

        assert!(weak_generation.upgrade().is_some());
        drop(view);
        assert!(weak_generation.upgrade().is_none());
    }
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

impl Adapter {
    #[cfg(target_os = "horizon")]
    unsafe fn open_horizon(&self) -> DeviceResult<crate::OpenDevice<Api>> {
        let device_maker = dk::DkDeviceMaker::defaults();
        let raw_device = unsafe { dk::dkDeviceCreate(&device_maker) };
        if raw_device.is_null() {
            eprintln!("[wgpu-deko3d] device_create=failed");
            return Err(crate::DeviceError::Lost);
        }
        eprintln!("[wgpu-deko3d] device_create=ok");

        let mut queue_maker = dk::DkQueueMaker::defaults(raw_device);
        queue_maker.flags = dk::DkQueueFlags_Graphics;
        let raw_queue = unsafe { dk::dkQueueCreate(&queue_maker) };
        if raw_queue.is_null() {
            eprintln!("[wgpu-deko3d] queue_create=failed");
            unsafe { dk::dkDeviceDestroy(raw_device) };
            return Err(crate::DeviceError::Lost);
        }
        eprintln!("[wgpu-deko3d] queue_create=ok");
        let command_recorder = match unsafe { CommandRecorder::new(raw_device) } {
            Ok(command_recorder) => command_recorder,
            Err(error) => {
                eprintln!("[wgpu-deko3d] command_recorder=failed error={error:?}");
                unsafe {
                    dk::dkQueueDestroy(raw_queue);
                    dk::dkDeviceDestroy(raw_device);
                }
                return Err(error);
            }
        };
        eprintln!("[wgpu-deko3d] command_recorder=ok");

        let buffer_pool = match GpuBufferPool::new(raw_device) {
            Ok(pool) => pool,
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
            buffer_pool,
        });

        Ok(crate::OpenDevice {
            device: Device {
                inner: inner.clone(),
            },
            queue: Queue {
                device: inner,
                state: Mutex::new(QueueState {
                    raw: RawQueue(raw_queue),
                    command_recorder,
                }),
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
        #[cfg(all(deko3d, not(target_os = "horizon")))]
        FORCED_HOST_INSTANCE_INIT_ATTEMPTS.fetch_add(1, Ordering::AcqRel);
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
        #[cfg(target_os = "horizon")]
        {
            self.create_default_surface()
        }
        #[cfg(not(target_os = "horizon"))]
        {
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
        transient_saves_memory: false,
    }
}

/// Minimal feature set backed by the current Deko3D proof paths.
///
/// `PASSTHROUGH_SHADERS` carries offline DKSH bytes through the public wgpu
/// unsafe shader path. `MAPPABLE_PRIMARY_BUFFERS` lets the first public triangle
/// proof initialize a vertex buffer directly, matching the direct-HAL smoke app.
/// `MULTI_DRAW_INDIRECT_COUNT` is currently CPU-side count-buffer emulation
/// over this backend's CPU-addressable buffers, not native Deko3D count-buffer
/// execution.
pub fn supported_features() -> wgt::Features {
    wgt::Features::PASSTHROUGH_SHADERS
        | wgt::Features::MAPPABLE_PRIMARY_BUFFERS
        | wgt::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
        | wgt::Features::PIPELINE_CACHE
        | wgt::Features::TIMESTAMP_QUERY
        | wgt::Features::MULTI_DRAW_INDIRECT_COUNT
}

/// Conservative capabilities for the first Deko3D adapter slice.
///
/// These are intentionally below the real hardware envelope until each resource
/// path is implemented and validated.
pub fn capabilities() -> crate::Capabilities {
    let mut limits = wgt::Limits::downlevel_defaults();
    limits.max_storage_buffers_per_shader_stage = 0;
    crate::Capabilities {
        limits,
        alignments: crate::Alignments {
            buffer_copy_offset: wgt::BufferSize::MIN,
            buffer_copy_pitch: wgt::BufferSize::new(256).unwrap(),
            uniform_bounds_check_alignment: wgt::BufferSize::new(256).unwrap(),
            raw_tlas_instance_size: 0,
            ray_tracing_scratch_buffer_alignment: 1,
        },
        downlevel: wgt::DownlevelCapabilities {
            flags: wgt::DownlevelFlags::SURFACE_VIEW_FORMATS
                | wgt::DownlevelFlags::CUBE_ARRAY_TEXTURES
                | wgt::DownlevelFlags::INDIRECT_EXECUTION,
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
            let mut state = self
                .state
                .lock()
                .map_err(|_| crate::SurfaceError::Other("deko3d surface mutex is poisoned"))?;
            if let Some(existing) = state.as_ref() {
                let existing = existing.inner.lock().map_err(|_| {
                    crate::SurfaceError::Other("deko3d surface generation mutex is poisoned")
                })?;
                if !Arc::ptr_eq(&existing.device, &device.inner) {
                    return Err(crate::SurfaceError::Other(
                        "deko3d surface is already owned by a different device",
                    ));
                }
                if existing.acquired_slot.is_some() {
                    return Err(crate::SurfaceError::Other(
                        "deko3d surface cannot be reconfigured with an acquired texture",
                    ));
                }
            }
            *state = None;
            let configuration = self.next_configuration.fetch_add(1, Ordering::Relaxed);
            *state = Some(Arc::new(unsafe {
                SurfaceState::new(
                    device.inner.clone(),
                    self.native_window,
                    configuration,
                    config,
                )
            }?));
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
        #[cfg(target_os = "horizon")]
        if let Ok(mut state) = self.state.lock() {
            let can_unconfigure = state.as_ref().is_some_and(|existing| {
                existing.inner.lock().is_ok_and(|existing| {
                    Arc::ptr_eq(&existing.device, &device.inner) && existing.acquired_slot.is_none()
                })
            });
            if can_unconfigure {
                *state = None;
            }
        }
    }

    unsafe fn acquire_texture(
        &self,
        _timeout: Option<Duration>,
        _fence: &Fence,
    ) -> Result<crate::AcquiredSurfaceTexture<Api>, crate::SurfaceError> {
        #[cfg(target_os = "horizon")]
        {
            let surface_state = self
                .state
                .lock()
                .map_err(|_| crate::SurfaceError::Other("deko3d surface mutex is poisoned"))?;
            let generation = surface_state
                .as_ref()
                .cloned()
                .ok_or(crate::SurfaceError::Other(
                    "deko3d surface is not configured",
                ))?;
            drop(surface_state);
            let mut generation_state = generation.inner.lock().map_err(|_| {
                crate::SurfaceError::Other("deko3d surface generation mutex is poisoned")
            })?;
            if generation_state.acquired_slot.is_some() {
                return Err(crate::SurfaceError::Timeout);
            }

            let slot = unsafe {
                dk::dkQueueAcquireImage(generation_state.render_queue, generation_state.swapchain)
            };
            if slot < 0 || slot as usize >= FRAMEBUFFER_COUNT {
                return Err(crate::SurfaceError::Lost);
            }
            generation_state.acquired_slot = Some(slot);
            let image: *const dk::DkImage = &generation_state.framebuffers[slot as usize];
            let image = RawImage(image, dk::DkImageView::defaults(image));
            let queue = RawQueueHandle(generation_state.render_queue);
            if !ACQUIRE_TRACED.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "[wgpu-deko3d] surface_trace acquire slot={slot} image={:p} queue={:p}",
                    image.0, queue.0
                );
            }
            let extent = generation_state.extent;
            let device_id = Arc::as_ptr(&generation_state.device) as usize;
            drop(generation_state);

            Ok(crate::AcquiredSurfaceTexture {
                texture: Resource::SurfaceTexture {
                    slot,
                    image,
                    queue,
                    extent,
                    device_id,
                    configuration: generation.configuration,
                    surface_id: self.id,
                    generation,
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
            if let Ok(state) = self.state.lock() {
                let Some(generation) = state.as_ref() else {
                    return;
                };
                let Ok(mut state) = generation.inner.lock() else {
                    return;
                };
                let Resource::SurfaceTexture {
                    slot,
                    queue,
                    device_id,
                    configuration,
                    surface_id,
                    ..
                } = texture
                else {
                    return;
                };
                if surface_id == self.id
                    && configuration == generation.configuration
                    && device_id == Arc::as_ptr(&state.device) as usize
                    && queue.0 == state.render_queue
                    && state.acquired_slot == Some(slot)
                {
                    state.acquired_slot = None;
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
            formats: vec![wgt::TextureFormat::Rgba8Unorm],
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
        (fence, fence_value): (&mut Fence, crate::FenceValue),
    ) -> DeviceResult<()> {
        #[cfg(target_os = "horizon")]
        let surface_queue = {
            let mut surface_queue: Option<RawQueueHandle> = None;
            for texture in surface_textures {
                let Resource::SurfaceTexture {
                    queue,
                    device_id,
                    surface_id,
                    ..
                } = texture
                else {
                    continue;
                };
                if *surface_id == 0 || *device_id != Arc::as_ptr(&self.device) as usize {
                    return Err(crate::DeviceError::Lost);
                }
                if let Some(existing) = surface_queue {
                    if existing.0 != queue.0 {
                        return Err(crate::DeviceError::Lost);
                    }
                } else {
                    surface_queue = Some(*queue);
                }
            }
            surface_queue
        };
        #[cfg(target_os = "horizon")]
        if !SUBMIT_TRACED.swap(true, Ordering::Relaxed) {
            eprintln!(
                "[wgpu-deko3d] surface_trace submit surface_textures={} queue={:p}",
                surface_textures.len(),
                surface_queue.map_or(ptr::null_mut(), |queue| queue.0)
            );
        }
        #[cfg(not(target_os = "horizon"))]
        let surface_queue = None;
        // All commands are executed synchronously.
        for cb in command_buffers {
            // SAFETY: Caller is responsible for ensuring synchronization between commands and
            // other mutations.
            unsafe {
                cb.execute(self, surface_queue)?;
            }
        }
        #[cfg(target_os = "horizon")]
        {
            let raw_queue = surface_queue.unwrap_or(self.raw_queue()?).0;
            let mut raw = fence.raw.lock().map_err(|_| crate::DeviceError::Lost)?;
            unsafe {
                dk::dkQueueSignalFence(raw_queue, &mut *raw, true);
                dk::dkQueueWaitFence(raw_queue, &mut *raw);
            }
            *fence.queue.lock().map_err(|_| crate::DeviceError::Lost)? =
                Some(RawQueueHandle(raw_queue));
        }
        fence.value.store(fence_value, Ordering::Release);
        Ok(())
    }
    unsafe fn present(
        &self,
        surface: &Surface,
        texture: Resource,
    ) -> Result<(), crate::SurfaceError> {
        #[cfg(target_os = "horizon")]
        {
            let state = surface
                .state
                .lock()
                .map_err(|_| crate::SurfaceError::Other("deko3d surface mutex is poisoned"))?;
            let generation = state.as_ref().ok_or(crate::SurfaceError::Other(
                "deko3d surface is not configured",
            ))?;
            let mut state = generation.inner.lock().map_err(|_| {
                crate::SurfaceError::Other("deko3d surface generation mutex is poisoned")
            })?;
            let Resource::SurfaceTexture {
                slot,
                queue,
                device_id,
                configuration,
                surface_id,
                ..
            } = texture
            else {
                return Err(crate::SurfaceError::Other(
                    "deko3d present requires a surface texture",
                ));
            };
            if surface_id != surface.id
                || device_id != Arc::as_ptr(&self.device) as usize
                || device_id != Arc::as_ptr(&state.device) as usize
                || queue.0 != state.render_queue
                || configuration != generation.configuration
                || state.acquired_slot != Some(slot)
            {
                return Err(crate::SurfaceError::Other(
                    "deko3d present received a texture from a different surface, device, or configuration",
                ));
            }
            unsafe { dk::dkQueuePresentImage(state.render_queue, state.swapchain, slot) };
            if !PRESENT_TRACED.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "[wgpu-deko3d] surface_trace present slot={slot} image={:p} queue={:p}",
                    &state.framebuffers[slot as usize], state.render_queue
                );
            }
            state.acquired_slot = None;
            Ok(())
        }
        #[cfg(not(target_os = "horizon"))]
        {
            Err(crate::SurfaceError::Other(
                "deko3d present requires the Horizon/Switch target",
            ))
        }
    }

    unsafe fn get_timestamp_period(&self) -> f32 {
        #[cfg(target_os = "horizon")]
        {
            625.0 / 384.0
        }
        #[cfg(not(target_os = "horizon"))]
        {
            1.0
        }
    }
}

impl crate::Device for Device {
    type A = Api;

    unsafe fn create_buffer(&self, desc: &crate::BufferDescriptor) -> DeviceResult<Buffer> {
        #[cfg(target_os = "horizon")]
        {
            Buffer::new(desc, self.inner.buffer_pool.clone())
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
    unsafe fn flush_mapped_ranges<I>(&self, buffer: &Buffer, ranges: I) {
        let _ = ranges;
        #[cfg(not(target_os = "horizon"))]
        let _ = buffer;
        #[cfg(target_os = "horizon")]
        {
            if let Err(error) = unsafe { buffer.upload_to_gpu() } {
                eprintln!("[wgpu-deko3d] flush_mapped_ranges failed: {error:?}");
                trace::record(format_args!(
                    "failure kind=flush_mapped_ranges buffer_id={} error={error:?}",
                    buffer.id()
                ));
                trace::dump("flush_mapped_ranges_failed");
            }
        }
    }
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
        #[cfg(target_os = "horizon")]
        let view_id = trace::resource_id();
        #[cfg(not(target_os = "horizon"))]
        let view_id = 0;
        match texture {
            Resource::SurfaceTexture {
                image,
                extent,
                generation,
                ..
            } => Ok(Resource::TextureView {
                id: view_id,
                label: desc.label.map(String::from),
                image: configure_raw_image_view(
                    *image,
                    desc.format,
                    wgt::TextureViewDimension::D2,
                    desc.range,
                )?,
                extent: *extent,
                sample_count: 1,
                format: desc.format,
                aspect: desc.range.aspect,
                view_dimension: wgt::TextureViewDimension::D2,
                owner: None,
                surface_generation: Some(generation.clone()),
            }),
            Resource::Texture(texture) => {
                if desc.format != texture.format()
                    || !matches!(
                        (texture.format(), desc.range.aspect),
                        (wgt::TextureFormat::Depth32Float, wgt::TextureAspect::All)
                            | (wgt::TextureFormat::Depth16Unorm, wgt::TextureAspect::All)
                            | (
                                wgt::TextureFormat::Depth32Float,
                                wgt::TextureAspect::DepthOnly
                            )
                            | (
                                wgt::TextureFormat::Depth16Unorm,
                                wgt::TextureAspect::DepthOnly
                            )
                            | (_, wgt::TextureAspect::All)
                    )
                {
                    return Err(crate::DeviceError::Lost);
                }
                #[cfg(target_os = "horizon")]
                trace::record_resource(format_args!(
                    "resource kind=texture_view id={} label={:?} texture_id={} texture_label={:?} format={:?} dimension={:?} base_mip={} mip_count={:?} base_layer={} layer_count={:?}",
                    view_id,
                    desc.label,
                    texture.id(),
                    texture.label(),
                    desc.format,
                    desc.dimension,
                    desc.range.base_mip_level,
                    desc.range.mip_level_count,
                    desc.range.base_array_layer,
                    desc.range.array_layer_count
                ));
                Ok(Resource::TextureView {
                    id: view_id,
                    label: desc.label.map(String::from),
                    image: configure_raw_image_view(
                        texture.raw_image(),
                        desc.format,
                        desc.dimension,
                        desc.range,
                    )?,
                    extent: texture.extent(),
                    sample_count: texture.sample_count(),
                    format: texture.format(),
                    aspect: desc.range.aspect,
                    view_dimension: desc.dimension,
                    owner: Some(texture.clone()),
                    surface_generation: None,
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
        let kind = supported_bind_group_layout_kind(desc.entries).or_else(|| {
            desc.entries
                .iter()
                .all(|entry| entry.visibility == wgt::ShaderStages::COMPUTE)
                .then_some(BindGroupLayoutKind::Inactive)
        });
        let Some(kind) = kind else {
            return Err(crate::DeviceError::Lost);
        };
        Ok(Resource::BindGroupLayout(kind))
    }
    unsafe fn destroy_bind_group_layout(&self, bg_layout: Resource) {}
    unsafe fn create_pipeline_layout(
        &self,
        desc: &crate::PipelineLayoutDescriptor<Resource>,
    ) -> DeviceResult<Resource> {
        if desc.bind_group_layouts.len() > 4 {
            return Err(crate::DeviceError::Lost);
        }
        for layout in desc.bind_group_layouts.iter().flatten() {
            if !matches!(layout, Resource::BindGroupLayout(_)) {
                return Err(crate::DeviceError::Lost);
            }
        }
        #[cfg(target_os = "horizon")]
        let layout = PipelineLayoutInner::new(desc, self.inner.buffer_pool.clone())?;
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
            let group = Arc::new(unsafe { BindGroupInner::new(self.inner.raw_device(), desc)? });
            trace::record_resource(format_args!(
                "resource kind=bind_group id={} label={:?} entries={} buffers={} textures={} samplers={}",
                group.id(),
                group.label,
                desc.entries.len(),
                desc.buffers.len(),
                desc.textures.len(),
                desc.samplers.len()
            ));
            Ok(Resource::BindGroup(group))
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
                crate::ShaderInput::Naga(_) => Ok(Resource::Placeholder),
                _ => Err(crate::ShaderError::Compilation(String::from(
                    "deko3d accepts WGSL only through a trusted stage-specific DKSH artifact provider",
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
            match RenderPipelineInner::new(desc) {
                Ok(pipeline) => Ok(Resource::RenderPipeline(Arc::new(pipeline))),
                Err(error) => {
                    trace::record(format_args!(
                        "failure kind=render_pipeline_create label={:?} topology={:?} color_targets={:?} depth_stencil={:?} error={error:?}",
                        desc.label,
                        desc.primitive.topology,
                        desc.color_targets,
                        desc.depth_stencil.as_ref().map(|state| state.format),
                    ));
                    trace::dump("render_pipeline_create");
                    Err(error)
                }
            }
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
    unsafe fn create_pipeline_cache(
        &self,
        _desc: &crate::PipelineCacheDescriptor<'_>,
    ) -> Result<Resource, crate::PipelineCacheError> {
        Ok(Resource::PipelineCache)
    }
    unsafe fn destroy_pipeline_cache(&self, cache: Resource) {}

    unsafe fn create_query_set(
        &self,
        desc: &wgt::QuerySetDescriptor<crate::Label>,
    ) -> DeviceResult<Resource> {
        #[cfg(target_os = "horizon")]
        {
            Ok(Resource::QuerySet(Arc::new(QuerySetInner::new(
                desc,
                self.inner.buffer_pool.clone(),
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
        Ok(Fence {
            value: AtomicU64::new(0),
            queue: Mutex::new(None),
            #[cfg(target_os = "horizon")]
            raw: Mutex::new(dk::DkFence::new()),
        })
    }
    unsafe fn destroy_fence(&self, fence: Fence) {}
    unsafe fn get_fence_value(&self, fence: &Fence) -> DeviceResult<crate::FenceValue> {
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
            Ok(fence.value.load(Ordering::Acquire) >= value)
        }
        #[cfg(not(target_os = "horizon"))]
        {
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
