#![allow(unused_variables)]

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

#[cfg(any(target_os = "horizon", test))]
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
    PipelineLayout,
    ShaderModule(Arc<ShaderModuleInner>),
    RenderPipeline(Arc<RenderPipelineInner>),
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
        image: RawImage,
        extent: wgt::Extent3d,
        format: wgt::TextureFormat,
        aspect: wgt::TextureAspect,
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
            image,
            extent,
            format: wgt::TextureFormat::Rgba8Unorm,
            aspect: wgt::TextureAspect::All,
            owner: None,
            surface_generation: Some(generation),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BindGroupLayoutKind {
    TextureSampler {
        uniform: Option<(u32, wgt::ShaderStages)>,
    },
    UniformBuffer {
        binding: u32,
        visibility: wgt::ShaderStages,
    },
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
    mem_block: dk::DkMemBlock,
    cmdbuf: dk::DkCmdBuf,
}

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
    acquired_slot: Option<i32>,
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
    depth_stencil_state: dk::DkDepthStencilState,
    uses_depth_stencil: bool,
}

#[cfg(target_os = "horizon")]
struct TextureInnerRaw {
    mem_block: dk::DkMemBlock,
    image: dk::DkImage,
    extent: wgt::Extent3d,
    format: wgt::TextureFormat,
}

#[cfg(target_os = "horizon")]
struct SamplerInnerRaw {
    sampler: dk::DkSampler,
}

#[cfg(target_os = "horizon")]
pub(super) enum BindGroupInnerRaw {
    TextureSampler {
        #[allow(dead_code)]
        texture: Arc<TextureInner>,
        #[allow(dead_code)]
        sampler: Arc<SamplerInner>,
        descriptor_mem_block: dk::DkMemBlock,
        image_descriptor_gpu_addr: dk::DkGpuAddr,
        sampler_descriptor_gpu_addr: dk::DkGpuAddr,
        image_descriptor: dk::DkImageDescriptor,
        sampler_descriptor: dk::DkSamplerDescriptor,
        uniform: Option<UniformBufferBinding>,
    },
    UniformBuffer(UniformBufferBinding),
}

#[cfg(target_os = "horizon")]
#[derive(Clone, Debug)]
pub(super) struct UniformBufferBinding {
    binding: u32,
    visibility: wgt::ShaderStages,
    buffer: Buffer,
    offset: wgt::BufferAddress,
    size: Option<wgt::BufferSize>,
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
const DEKO_UNIFORM_BUF_MAX_SIZE: u64 = 0x10000;

static DEFAULT_SURFACE_LEASED: AtomicBool = AtomicBool::new(false);
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
    pub(super) fn extent(&self) -> wgt::Extent3d {
        self.inner.extent
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn format(&self) -> wgt::TextureFormat {
        self.inner.format
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
}

impl BindGroupInner {
    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn bind_descriptor_sets(&self, cmdbuf: dk::DkCmdBuf) -> DeviceResult<()> {
        match &self.inner {
            BindGroupInnerRaw::TextureSampler {
                image_descriptor_gpu_addr,
                sampler_descriptor_gpu_addr,
                image_descriptor,
                sampler_descriptor,
                uniform,
                ..
            } => unsafe {
                dk::dkCmdBufPushData(
                    cmdbuf,
                    *image_descriptor_gpu_addr,
                    ptr::addr_of!(*image_descriptor).cast(),
                    size_of::<dk::DkImageDescriptor>() as u32,
                );
                dk::dkCmdBufPushData(
                    cmdbuf,
                    *sampler_descriptor_gpu_addr,
                    ptr::addr_of!(*sampler_descriptor).cast(),
                    size_of::<dk::DkSamplerDescriptor>() as u32,
                );
                dk::dkCmdBufBindImageDescriptorSet(cmdbuf, *image_descriptor_gpu_addr, 1);
                dk::dkCmdBufBindSamplerDescriptorSet(cmdbuf, *sampler_descriptor_gpu_addr, 1);
                dk::dkCmdBufBindTexture(
                    cmdbuf,
                    dk::DkStage::DkStage_Fragment,
                    0,
                    dk::dkMakeTextureHandle(0, 0),
                );
                if let Some(uniform) = uniform {
                    bind_uniform_buffer(cmdbuf, uniform)?;
                }
            },
            BindGroupInnerRaw::UniformBuffer(binding) => {
                unsafe { bind_uniform_buffer(cmdbuf, binding)? };
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "horizon")]
unsafe fn bind_uniform_buffer(
    cmdbuf: dk::DkCmdBuf,
    binding: &UniformBufferBinding,
) -> DeviceResult<()> {
    let (gpu_addr, gpu_size) = binding.buffer.gpu_binding(binding.offset, binding.size)?;
    if gpu_addr % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0
        || gpu_size > dk::DK_UNIFORM_BUF_MAX_SIZE
    {
        return Err(crate::DeviceError::Lost);
    }
    unsafe {
        if binding.visibility.contains(wgt::ShaderStages::VERTEX) {
            dk::dkCmdBufBindUniformBuffer(
                cmdbuf,
                dk::DkStage::DkStage_Vertex,
                binding.binding,
                gpu_addr,
                gpu_size,
            );
        }
        if binding.visibility.contains(wgt::ShaderStages::FRAGMENT) {
            dk::dkCmdBufBindUniformBuffer(
                cmdbuf,
                dk::DkStage::DkStage_Fragment,
                binding.binding,
                gpu_addr,
                gpu_size,
            );
        }
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
        record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
    ) -> DeviceResult<()> {
        let mut state = self.state.lock().map_err(|_| crate::DeviceError::Lost)?;
        unsafe { state.command_recorder.record_and_submit(raw_queue, record) }
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
        record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
    ) -> DeviceResult<()> {
        unsafe { dk::dkCmdBufClear(self.cmdbuf) };
        let result = record(self.cmdbuf);
        if result.is_ok() {
            unsafe {
                let cmds = dk::dkCmdBufFinishList(self.cmdbuf);
                dk::dkQueueSubmitCommands(raw_queue, cmds);
                dk::dkQueueWaitIdle(raw_queue);
            }
        }
        unsafe { dk::dkCmdBufClear(self.cmdbuf) };
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
        if let Self::TextureSampler {
            descriptor_mem_block,
            ..
        } = self
        {
            if !descriptor_mem_block.is_null() {
                unsafe { dk::dkMemBlockDestroy(*descriptor_mem_block) };
            }
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
            || desc.multisample.count != 1
            || desc.multisample.alpha_to_coverage_enabled
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        if desc.primitive.topology != wgt::PrimitiveTopology::TriangleList
            || desc.primitive.strip_index_format.is_some()
            || desc.primitive.unclipped_depth
            || desc.primitive.polygon_mode != wgt::PolygonMode::Fill
            || desc.primitive.conservative
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        if desc.color_targets.len() != 1 {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let Some(color_target) = &desc.color_targets[0] else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        if color_target.format != wgt::TextureFormat::Rgba8Unorm
            || color_target.blend.is_some()
            || color_target.write_mask != wgt::ColorWrites::ALL
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }

        let crate::VertexProcessor::Standard {
            vertex_buffers,
            vertex_stage,
        } = &desc.vertex_processor
        else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        if !vertex_stage.entry_point.is_empty() && vertex_stage.entry_point != "main" {
            return Err(crate::PipelineError::EntryPoint(naga::ShaderStage::Vertex));
        }
        let Some(fragment_stage) = &desc.fragment_stage else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        if !fragment_stage.entry_point.is_empty() && fragment_stage.entry_point != "main" {
            return Err(crate::PipelineError::EntryPoint(
                naga::ShaderStage::Fragment,
            ));
        }

        let Resource::ShaderModule(vertex_shader) = vertex_stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };
        let Resource::ShaderModule(fragment_shader) = fragment_stage.module else {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        };

        let mut vertex_buffer_states = Vec::new();
        let mut vertex_attributes = Vec::new();
        for (buffer_id, layout) in vertex_buffers.iter().enumerate() {
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

        let mut rasterizer_state = dk::DkRasterizerState::defaults();
        rasterizer_state.set_cull_mode(match desc.primitive.cull_mode {
            None => dk::DkFace_None,
            Some(wgt::Face::Front) => dk::DkFace_Front,
            Some(wgt::Face::Back) => dk::DkFace_Back,
        });
        rasterizer_state.set_front_face(match desc.primitive.front_face {
            wgt::FrontFace::Ccw => dk::DkFrontFace_CCW,
            wgt::FrontFace::Cw => dk::DkFrontFace_CW,
        });
        let uses_depth_stencil = desc.depth_stencil.is_some();
        let depth_stencil_state = desc
            .depth_stencil
            .as_ref()
            .map(map_depth_stencil_state)
            .transpose()?
            .unwrap_or_else(depth_stencil_disabled_state);

        Ok(Self {
            inner: RenderPipelineInnerRaw {
                vertex_shader: vertex_shader.clone(),
                fragment_shader: fragment_shader.clone(),
                primitive: dk::DkPrimitive::DkPrimitive_Triangles,
                vertex_buffers: vertex_buffer_states,
                vertex_attributes,
                rasterizer_state,
                color_state: dk::DkColorState::defaults(),
                color_write_state: dk::DkColorWriteState::defaults(),
                depth_stencil_state,
                uses_depth_stencil,
            },
        })
    }
}

#[cfg(target_os = "horizon")]
impl TextureInner {
    unsafe fn new(raw_device: dk::DkDevice, desc: &crate::TextureDescriptor) -> DeviceResult<Self> {
        if desc.dimension != wgt::TextureDimension::D2
            || !matches!(
                desc.format,
                wgt::TextureFormat::Rgba8Unorm
                    | wgt::TextureFormat::Rgba8UnormSrgb
                    | wgt::TextureFormat::Depth32Float
            )
            || desc.mip_level_count != 1
            || desc.sample_count != 1
            || desc.size.width == 0
            || desc.size.height == 0
            || desc.size.depth_or_array_layers != 1
        {
            return Err(crate::DeviceError::Lost);
        }
        let is_depth = desc.format == wgt::TextureFormat::Depth32Float;
        if (is_depth && !desc.usage.contains(wgt::TextureUses::DEPTH_STENCIL_WRITE))
            || (!is_depth
                && (!desc.usage.contains(wgt::TextureUses::RESOURCE)
                    || !desc.usage.contains(wgt::TextureUses::COPY_DST)))
        {
            return Err(crate::DeviceError::Lost);
        }

        let mut image_layout_maker = dk::DkImageLayoutMaker::defaults(raw_device);
        image_layout_maker.format =
            map_texture_image_format(desc.format).ok_or(crate::DeviceError::Lost)?;
        if is_depth {
            image_layout_maker.flags = dk::DkImageFlags_UsageRender;
        }
        image_layout_maker.dimensions[0] = desc.size.width;
        image_layout_maker.dimensions[1] = desc.size.height;
        image_layout_maker.mipLevels = 1;

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
                extent: desc.size,
                format: desc.format,
            },
        })
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
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
    ) -> DeviceResult<Self> {
        let Resource::BindGroupLayout(kind) = desc.layout else {
            return Err(crate::DeviceError::Lost);
        };
        match *kind {
            BindGroupLayoutKind::TextureSampler { uniform } => unsafe {
                Self::new_texture_sampler(raw_device, desc, uniform)
            },
            BindGroupLayoutKind::UniformBuffer {
                binding,
                visibility,
            } => Self::new_uniform_buffer(desc, binding, visibility),
        }
    }

    unsafe fn new_texture_sampler(
        raw_device: dk::DkDevice,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        uniform_layout: Option<(u32, wgt::ShaderStages)>,
    ) -> DeviceResult<Self> {
        if desc.buffers.len() != usize::from(uniform_layout.is_some())
            || desc.samplers.len() != 1
            || desc.textures.len() != 1
            || !desc.acceleration_structures.is_empty()
            || !desc.external_textures.is_empty()
            || desc.entries.len() != 2 + usize::from(uniform_layout.is_some())
        {
            return Err(crate::DeviceError::Lost);
        }

        let Resource::Sampler(sampler) = desc.samplers[0] else {
            return Err(crate::DeviceError::Lost);
        };
        let texture_binding = &desc.textures[0];
        if !texture_binding.usage.contains(wgt::TextureUses::RESOURCE) {
            return Err(crate::DeviceError::Lost);
        }
        let Resource::TextureView {
            image,
            format,
            aspect,
            owner: Some(texture),
            ..
        } = texture_binding.view
        else {
            return Err(crate::DeviceError::Lost);
        };
        if *aspect != wgt::TextureAspect::All {
            return Err(crate::DeviceError::Lost);
        }
        let uniform = match uniform_layout {
            None => None,
            Some((binding, visibility)) => {
                Some(Self::uniform_buffer_binding(desc, binding, visibility)?)
            }
        };

        let descriptor_size = align_up(
            size_of::<dk::DkImageDescriptor>() as u32,
            dk::DK_IMAGE_DESCRIPTOR_ALIGNMENT,
        ) + align_up(
            size_of::<dk::DkSamplerDescriptor>() as u32,
            dk::DK_SAMPLER_DESCRIPTOR_ALIGNMENT,
        );
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

        let mut image_descriptor = dk::DkImageDescriptor::zeroed();
        let mut image_view = dk::DkImageView::defaults(image.0);
        image_view.format = map_texture_image_format(*format).ok_or(crate::DeviceError::Lost)?;
        let mut sampler_descriptor = dk::DkSamplerDescriptor::zeroed();
        unsafe {
            dk::dkImageDescriptorInitialize(&mut image_descriptor, &image_view, false, false);
            dk::dkSamplerDescriptorInitialize(&mut sampler_descriptor, &sampler.inner.sampler);
        }

        let sampler_offset = align_up(
            size_of::<dk::DkImageDescriptor>() as u32,
            dk::DK_SAMPLER_DESCRIPTOR_ALIGNMENT,
        ) as u64;

        Ok(Self {
            inner: BindGroupInnerRaw::TextureSampler {
                texture: texture.clone(),
                sampler: sampler.clone(),
                descriptor_mem_block,
                image_descriptor_gpu_addr: descriptor_gpu_addr,
                sampler_descriptor_gpu_addr: descriptor_gpu_addr + sampler_offset,
                image_descriptor,
                sampler_descriptor,
                uniform,
            },
        })
    }

    fn new_uniform_buffer(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
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
        Ok(Self {
            inner: BindGroupInnerRaw::UniformBuffer(Self::uniform_buffer_binding(
                desc, binding, visibility,
            )?),
        })
    }

    fn uniform_buffer_binding(
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
        binding: u32,
        visibility: wgt::ShaderStages,
    ) -> DeviceResult<UniformBufferBinding> {
        let Some(entry) = desc
            .entries
            .iter()
            .find(|entry| entry.binding == binding && entry.resource_index == 0)
        else {
            return Err(crate::DeviceError::Lost);
        };
        if entry.count != 1 {
            return Err(crate::DeviceError::Lost);
        }

        let buffer = &desc.buffers[0];
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
            buffer: buffer.buffer.clone(),
            offset: buffer.offset,
            size: buffer.size,
        })
    }
}

#[cfg(any(target_os = "horizon", test))]
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
        _ => Err(crate::PipelineError::Device(crate::DeviceError::Lost)),
    }
}

#[cfg(any(target_os = "horizon", test))]
fn map_texture_image_format(format: wgt::TextureFormat) -> Option<dk::DkImageFormat> {
    match format {
        wgt::TextureFormat::Rgba8Unorm => Some(dk::DkImageFormat::DkImageFormat_RGBA8_Unorm),
        wgt::TextureFormat::Rgba8UnormSrgb => {
            Some(dk::DkImageFormat::DkImageFormat_RGBA8_Unorm_sRGB)
        }
        wgt::TextureFormat::Depth32Float => Some(dk::DkImageFormat::DkImageFormat_ZF32),
        _ => None,
    }
}

#[cfg(any(target_os = "horizon", test))]
fn map_depth_stencil_state(
    state: &wgt::DepthStencilState,
) -> Result<dk::DkDepthStencilState, crate::PipelineError> {
    if state.format != wgt::TextureFormat::Depth32Float
        || state.stencil != wgt::StencilState::default()
        || state.bias != wgt::DepthBiasState::default()
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
        wgt::AddressMode::ClampToBorder => Err(crate::DeviceError::Lost),
    }
}

fn supported_bind_group_layout_kind(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    if entries.len() == 2 || entries.len() == 3 {
        return supported_texture_bind_group_layout_kind(entries);
    }

    if entries.len() == 1 {
        return supported_uniform_bind_group_layout_kind(entries[0]);
    }

    None
}

fn supported_texture_bind_group_layout_kind(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut has_texture = false;
    let mut has_sampler = false;
    let mut uniform = None;
    for entry in entries {
        if entry.count.is_some() {
            return None;
        }
        match (entry.binding, entry.ty) {
            (
                0,
                wgt::BindingType::Texture {
                    sample_type: wgt::TextureSampleType::Float { .. },
                    view_dimension: wgt::TextureViewDimension::D2,
                    multisampled: false,
                },
            ) if entry.visibility.contains(wgt::ShaderStages::FRAGMENT) => has_texture = true,
            (
                1,
                wgt::BindingType::Sampler(
                    wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
                ),
            ) if entry.visibility.contains(wgt::ShaderStages::FRAGMENT) => has_sampler = true,
            (
                2,
                wgt::BindingType::Buffer {
                    ty: wgt::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size,
                },
            ) if entry.visibility.contains(wgt::ShaderStages::VERTEX)
                && min_binding_size.is_none_or(|size| size.get() <= DEKO_UNIFORM_BUF_MAX_SIZE) =>
            {
                uniform = Some((entry.binding, entry.visibility));
            }
            _ => return None,
        }
    }
    (has_texture && has_sampler).then_some(BindGroupLayoutKind::TextureSampler { uniform })
}

fn supported_uniform_bind_group_layout_kind(
    entry: wgt::BindGroupLayoutEntry,
) -> Option<BindGroupLayoutKind> {
    if entry.count.is_some()
        || entry.visibility.is_empty()
        || !wgt::ShaderStages::VERTEX_FRAGMENT.contains(entry.visibility)
        || entry.binding >= DEKO_UNIFORM_BUFFER_COUNT
    {
        return None;
    }

    match entry.ty {
        wgt::BindingType::Buffer {
            ty: wgt::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size,
        } => {
            if min_binding_size.is_some_and(|size| size.get() > DEKO_UNIFORM_BUF_MAX_SIZE) {
                return None;
            }
            Some(BindGroupLayoutKind::UniformBuffer {
                binding: entry.binding,
                visibility: entry.visibility,
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
        assert!(map_texture_image_format(wgt::TextureFormat::Bgra8Unorm).is_none());
        for format in [
            wgt::VertexFormat::Float32x2,
            wgt::VertexFormat::Float32x3,
            wgt::VertexFormat::Float32x4,
            wgt::VertexFormat::Uint32,
            wgt::VertexFormat::Uint32x2,
            wgt::VertexFormat::Uint32x3,
            wgt::VertexFormat::Uint32x4,
        ] {
            assert!(map_vertex_format(format).is_ok(), "{format:?}");
        }
        assert!(map_vertex_format(wgt::VertexFormat::Sint16x4).is_err());
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
            return Err(crate::DeviceError::Lost);
        }

        let mut queue_maker = dk::DkQueueMaker::defaults(raw_device);
        queue_maker.flags = dk::DkQueueFlags_Graphics;
        let raw_queue = unsafe { dk::dkQueueCreate(&queue_maker) };
        if raw_queue.is_null() {
            unsafe { dk::dkDeviceDestroy(raw_device) };
            return Err(crate::DeviceError::Lost);
        }
        let command_recorder = match unsafe { CommandRecorder::new(raw_device) } {
            Ok(command_recorder) => command_recorder,
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
        Err(crate::InstanceError::new(String::from(
            "deko3d surface creation is unavailable until the explicit B1c Switch surface policy",
        )))
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
pub fn supported_features() -> wgt::Features {
    wgt::Features::PASSTHROUGH_SHADERS | wgt::Features::MAPPABLE_PRIMARY_BUFFERS
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
        },
        downlevel: wgt::DownlevelCapabilities {
            flags: wgt::DownlevelFlags::empty(),
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
            let image = RawImage(image);
            let queue = RawQueueHandle(generation_state.render_queue);
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
        if format == wgt::TextureFormat::Rgba8Unorm {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        } else if format == wgt::TextureFormat::Rgba8UnormSrgb {
            crate::TextureFormatCapabilities::SAMPLED | crate::TextureFormatCapabilities::COPY_DST
        } else if format == wgt::TextureFormat::Depth32Float {
            crate::TextureFormatCapabilities::DEPTH_STENCIL_ATTACHMENT
        } else {
            crate::TextureFormatCapabilities::empty()
        }
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
            Resource::SurfaceTexture {
                image,
                extent,
                generation,
                ..
            } => Ok(Resource::surface_texture_view(
                *image,
                *extent,
                generation.clone(),
            )),
            Resource::Texture(texture) => {
                if desc.format != texture.format()
                    || desc.dimension != wgt::TextureViewDimension::D2
                    || desc.range.base_mip_level != 0
                    || !matches!(desc.range.mip_level_count, None | Some(1))
                    || desc.range.base_array_layer != 0
                    || !matches!(desc.range.array_layer_count, None | Some(1))
                    || !matches!(
                        (texture.format(), desc.range.aspect),
                        (wgt::TextureFormat::Depth32Float, wgt::TextureAspect::All)
                            | (
                                wgt::TextureFormat::Depth32Float,
                                wgt::TextureAspect::DepthOnly
                            )
                            | (_, wgt::TextureAspect::All)
                    )
                {
                    return Err(crate::DeviceError::Lost);
                }
                Ok(Resource::TextureView {
                    image: texture.raw_image(),
                    extent: texture.extent(),
                    format: texture.format(),
                    aspect: desc.range.aspect,
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
        if desc.immediate_size != 0 || desc.bind_group_layouts.len() > 2 {
            return Err(crate::DeviceError::Lost);
        }
        for layout in desc.bind_group_layouts.iter().flatten() {
            if !matches!(layout, Resource::BindGroupLayout(_)) {
                return Err(crate::DeviceError::Lost);
            }
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
                BindGroupInner::new(self.inner.raw_device(), desc)?
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
                    "deko3d only accepts explicit offline DKSH shader input",
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
        Err(crate::PipelineError::Device(crate::DeviceError::Lost))
    }
    unsafe fn destroy_compute_pipeline(&self, pipeline: Resource) {}
    unsafe fn create_pipeline_cache(
        &self,
        desc: &crate::PipelineCacheDescriptor<'_>,
    ) -> Result<Resource, crate::PipelineCacheError> {
        Err(crate::PipelineCacheError::Device(crate::DeviceError::Lost))
    }
    unsafe fn destroy_pipeline_cache(&self, cache: Resource) {}

    unsafe fn create_query_set(
        &self,
        desc: &wgt::QuerySetDescriptor<crate::Label>,
    ) -> DeviceResult<Resource> {
        Err(crate::DeviceError::Lost)
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
            if fence.value.load(Ordering::Acquire) < value {
                return Ok(false);
            }
            let queue = (*fence.queue.lock().map_err(|_| crate::DeviceError::Lost)?)
                .ok_or(crate::DeviceError::Lost)?;
            let mut raw = fence.raw.lock().map_err(|_| crate::DeviceError::Lost)?;
            unsafe { dk::dkQueueWaitFence(queue.0, &mut *raw) };
            Ok(true)
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
