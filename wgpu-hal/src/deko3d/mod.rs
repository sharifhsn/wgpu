#![allow(unused_variables)]

use alloc::{string::String, vec, vec::Vec};
use core::{cell::UnsafeCell, ptr, sync::atomic::Ordering, time::Duration};

use core::fmt;
#[cfg(target_os = "horizon")]
use core::mem::size_of;

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
    command_recorder: UnsafeCell<CommandRecorder>,
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
    },
    TextureView {
        image: RawImage,
        extent: wgt::Extent3d,
        owner: Option<Arc<TextureInner>>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum BindGroupLayoutKind {
    TextureSampler,
    UniformBuffer {
        binding: u32,
        visibility: wgt::ShaderStages,
        has_dynamic_offset: bool,
    },
}

#[derive(Debug)]
pub struct Fence {
    value: AtomicU64,
}

type DeviceResult<T> = Result<T, crate::DeviceError>;

#[derive(Debug)]
struct DeviceInner {
    #[allow(dead_code)]
    raw: RawDevice,
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
    blend_state: dk::DkBlendState,
}

#[cfg(target_os = "horizon")]
struct TextureInnerRaw {
    mem_block: dk::DkMemBlock,
    image: dk::DkImage,
    extent: wgt::Extent3d,
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
    },
    UniformBuffer(UniformBufferBinding),
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

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn extent(&self) -> wgt::Extent3d {
        wgt::Extent3d {
            width: 0,
            height: 0,
            depth_or_array_layers: 1,
        }
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
            BindGroupInnerRaw::TextureSampler {
                image_descriptor_gpu_addr,
                sampler_descriptor_gpu_addr,
                image_descriptor,
                sampler_descriptor,
                ..
            } => unsafe {
                if !dynamic_offsets.is_empty() {
                    return Err(crate::DeviceError::Lost);
                }
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
            },
            BindGroupInnerRaw::UniformBuffer(binding) => {
                let dynamic_offset = match (binding.has_dynamic_offset, dynamic_offsets) {
                    (true, [offset]) => u64::from(*offset),
                    (false, []) => 0,
                    _ => return Err(crate::DeviceError::Lost),
                };
                if dynamic_offset % u64::from(dk::DK_UNIFORM_BUF_ALIGNMENT) != 0 {
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
            }
        }
        Ok(())
    }
}

impl Queue {
    #[cfg(target_os = "horizon")]
    pub(super) fn raw_queue(&self) -> RawQueueHandle {
        RawQueueHandle(self.raw.0)
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn record_and_submit(
        &self,
        raw_queue: dk::DkQueue,
        record: impl FnOnce(dk::DkCmdBuf) -> DeviceResult<()>,
    ) -> DeviceResult<()> {
        let command_recorder = unsafe { &mut *self.command_recorder.get() };
        unsafe { command_recorder.record_and_submit(raw_queue, record) }
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
        if desc.depth_stencil.is_some()
            || desc.multiview_mask.is_some()
            || desc.multisample.count != 1
            || desc.multisample.alpha_to_coverage_enabled
        {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let primitive = map_primitive_topology(&desc.primitive)?;
        if desc.primitive.strip_index_format.is_some()
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
        if color_target.format != wgt::TextureFormat::Rgba8Unorm {
            return Err(crate::PipelineError::Device(crate::DeviceError::Lost));
        }
        let (color_state, blend_state) = map_blend_state(color_target.blend)?;
        let color_write_state = map_color_write_state(color_target.write_mask);
        let rasterizer_state = map_rasterizer_state(&desc.primitive);

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
                blend_state,
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
fn map_color_write_state(write_mask: wgt::ColorWrites) -> dk::DkColorWriteState {
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

    dk::DkColorWriteState { masks: mask }
}

#[cfg(target_os = "horizon")]
fn map_blend_state(
    blend: Option<wgt::BlendState>,
) -> Result<(dk::DkColorState, dk::DkBlendState), crate::PipelineError> {
    let mut color_state = dk::DkColorState::defaults();
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
        color_state.set_blend_enable(0, true);
    }

    Ok((color_state, blend_state))
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
        if desc.dimension != wgt::TextureDimension::D2
            || desc.format != wgt::TextureFormat::Rgba8Unorm
            || desc.mip_level_count != 1
            || desc.sample_count != 1
            || desc.size.width == 0
            || desc.size.height == 0
            || desc.size.depth_or_array_layers != 1
        {
            return Err(crate::DeviceError::Lost);
        }
        if !desc.usage.contains(wgt::TextureUses::RESOURCE)
            || !desc.usage.contains(wgt::TextureUses::COPY_DST)
        {
            return Err(crate::DeviceError::Lost);
        }

        let mut image_layout_maker = dk::DkImageLayoutMaker::defaults(raw_device);
        image_layout_maker.format = dk::DkImageFormat::DkImageFormat_RGBA8_Unorm;
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
            BindGroupLayoutKind::TextureSampler => unsafe {
                Self::new_texture_sampler(raw_device, desc)
            },
            BindGroupLayoutKind::UniformBuffer {
                binding,
                visibility,
                has_dynamic_offset,
            } => Self::new_uniform_buffer(desc, binding, visibility, has_dynamic_offset),
        }
    }

    unsafe fn new_texture_sampler(
        raw_device: dk::DkDevice,
        desc: &crate::BindGroupDescriptor<Resource, Buffer, Resource, Resource, Resource>,
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

        let Resource::Sampler(sampler) = desc.samplers[0] else {
            return Err(crate::DeviceError::Lost);
        };
        let texture_binding = &desc.textures[0];
        if !texture_binding.usage.contains(wgt::TextureUses::RESOURCE) {
            return Err(crate::DeviceError::Lost);
        }
        let Resource::TextureView {
            image,
            owner: Some(texture),
            ..
        } = texture_binding.view
        else {
            return Err(crate::DeviceError::Lost);
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
        let image_view = dk::DkImageView::defaults(image.0);
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
            },
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

        Ok(Self {
            inner: BindGroupInnerRaw::UniformBuffer(UniformBufferBinding {
                binding,
                visibility,
                has_dynamic_offset,
                buffer: buffer.buffer.clone(),
                offset: buffer.offset,
                size: buffer.size,
            }),
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
    if entries.len() == 2 {
        return supported_texture_bind_group_layout_kind(entries);
    }

    if entries.len() == 1 {
        return supported_uniform_bind_group_layout_kind(entries[0]);
    }

    None
}

fn supported_pipeline_layout(bind_group_layouts: &[Option<&Resource>]) -> bool {
    if bind_group_layouts.len() > crate::MAX_BIND_GROUPS {
        return false;
    }

    let mut has_texture_sampler = false;
    let mut vertex_uniform_bindings = 0u32;
    let mut fragment_uniform_bindings = 0u32;
    for layout in bind_group_layouts.iter().flatten() {
        match layout {
            Resource::BindGroupLayout(BindGroupLayoutKind::TextureSampler) => {
                if has_texture_sampler {
                    return false;
                }
                has_texture_sampler = true;
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
            }
            _ => return false,
        }
    }

    true
}

fn supported_texture_bind_group_layout_kind(
    entries: &[wgt::BindGroupLayoutEntry],
) -> Option<BindGroupLayoutKind> {
    let mut has_texture = false;
    let mut has_sampler = false;
    for entry in entries {
        if entry.count.is_some() || !entry.visibility.contains(wgt::ShaderStages::FRAGMENT) {
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
            ) => has_texture = true,
            (
                1,
                wgt::BindingType::Sampler(
                    wgt::SamplerBindingType::Filtering | wgt::SamplerBindingType::NonFiltering,
                ),
            ) => has_sampler = true,
            _ => return None,
        }
    }
    (has_texture && has_sampler).then_some(BindGroupLayoutKind::TextureSampler)
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
            has_dynamic_offset,
            min_binding_size,
        } => {
            if min_binding_size.is_some_and(|size| size.get() > DEKO_UNIFORM_BUF_MAX_SIZE) {
                return None;
            }
            Some(BindGroupLayoutKind::UniformBuffer {
                binding: entry.binding,
                visibility: entry.visibility,
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
                raw: RawQueue(raw_queue),
                command_recorder: UnsafeCell::new(command_recorder),
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
        if format == wgt::TextureFormat::Rgba8Unorm {
            crate::TextureFormatCapabilities::SAMPLED
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT
                | crate::TextureFormatCapabilities::COLOR_ATTACHMENT_BLEND
                | crate::TextureFormatCapabilities::COPY_SRC
                | crate::TextureFormatCapabilities::COPY_DST
        } else {
            crate::TextureFormatCapabilities::empty()
        }
    }

    unsafe fn surface_capabilities(&self, surface: &Surface) -> Option<crate::SurfaceCapabilities> {
        Some(crate::SurfaceCapabilities {
            formats: vec![wgt::SurfaceFormatCapabilities {
                format: wgt::TextureFormat::Rgba8Unorm,
                color_spaces: wgt::SurfaceColorSpaces::SRGB,
            }],
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
        let surface_queue = surface_textures.iter().find_map(|texture| match texture {
            Resource::SurfaceTexture { queue, .. } => Some(*queue),
            _ => None,
        });
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
                owner: None,
            }),
            Resource::Texture(texture) => Ok(Resource::TextureView {
                image: texture.raw_image(),
                extent: texture.extent(),
                owner: Some(texture.clone()),
            }),
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
        Err(crate::PipelineError::Device(crate::DeviceError::Lost))
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
        // This method is inherited from the temporary skeleton. Real Deko3D
        // fence semantics must replace it before any adapter is exposed.
        assert!(
            fence.value.load(Ordering::Acquire) >= value,
            "submission must have already been done"
        );
        Ok(true)
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
