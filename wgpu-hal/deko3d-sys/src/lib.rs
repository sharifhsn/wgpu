#![no_std]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_long, c_void};

pub type DkGpuAddr = u64;
pub type DkCmdList = usize;
pub type DkResHandle = u32;

#[repr(C, align(8))]
pub struct DkFence {
    _storage: [u8; 64],
}

impl DkFence {
    pub const fn new() -> Self {
        Self { _storage: [0; 64] }
    }
}

#[repr(C)]
pub struct tag_DkDevice {
    _private: [u8; 0],
}

#[repr(C)]
pub struct tag_DkMemBlock {
    _private: [u8; 0],
}

#[repr(C)]
pub struct tag_DkCmdBuf {
    _private: [u8; 0],
}

#[repr(C)]
pub struct tag_DkQueue {
    _private: [u8; 0],
}

#[repr(C)]
pub struct tag_DkSwapchain {
    _private: [u8; 0],
}

#[repr(C)]
pub struct NWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

pub type DkDevice = *mut tag_DkDevice;
pub type DkMemBlock = *mut tag_DkMemBlock;
pub type DkCmdBuf = *mut tag_DkCmdBuf;
pub type DkQueue = *mut tag_DkQueue;
pub type DkSwapchain = *mut tag_DkSwapchain;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkResult {
    DkResult_Success,
    DkResult_Fail,
    DkResult_Timeout,
    DkResult_OutOfMemory,
    DkResult_NotImplemented,
    DkResult_MisalignedSize,
    DkResult_MisalignedData,
    DkResult_BadInput,
    DkResult_BadFlags,
    DkResult_BadState,
}

pub type DkDebugFunc = Option<
    unsafe extern "C" fn(
        userData: *mut c_void,
        context: *const c_char,
        result: DkResult,
        message: *const c_char,
    ),
>;
pub type DkAllocFunc = Option<
    unsafe extern "C" fn(
        userData: *mut c_void,
        alignment: usize,
        size: usize,
        out: *mut *mut c_void,
    ) -> DkResult,
>;
pub type DkFreeFunc = Option<unsafe extern "C" fn(userData: *mut c_void, mem: *mut c_void)>;
pub type DkCmdBufAddMemFunc =
    Option<unsafe extern "C" fn(userData: *mut c_void, cmdbuf: DkCmdBuf, minReqSize: usize)>;

pub const DkDeviceFlags_DepthZeroToOne: u32 = 0 << 8;
pub const DkDeviceFlags_DepthMinusOneToOne: u32 = 1 << 8;
pub const DkDeviceFlags_OriginUpperLeft: u32 = 0 << 9;
pub const DkDeviceFlags_OriginLowerLeft: u32 = 1 << 9;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkDeviceMaker {
    pub userData: *mut c_void,
    pub cbDebug: DkDebugFunc,
    pub cbAlloc: DkAllocFunc,
    pub cbFree: DkFreeFunc,
    pub flags: u32,
}

impl DkDeviceMaker {
    pub const fn defaults() -> Self {
        Self {
            userData: core::ptr::null_mut(),
            cbDebug: None,
            cbAlloc: None,
            cbFree: None,
            flags: DkDeviceFlags_DepthZeroToOne | DkDeviceFlags_OriginUpperLeft,
        }
    }
}

pub const DK_MEMBLOCK_ALIGNMENT: u32 = 0x1000;
pub const DK_CMDMEM_ALIGNMENT: u32 = 4;
pub const DK_SHADER_CODE_ALIGNMENT: u32 = 0x100;
pub const DK_SHADER_CODE_UNUSABLE_SIZE: u32 = 0x400;
pub const DK_IMAGE_DESCRIPTOR_ALIGNMENT: u32 = 0x20;
pub const DK_SAMPLER_DESCRIPTOR_ALIGNMENT: u32 = 0x20;
pub const DK_IMAGE_LINEAR_STRIDE_ALIGNMENT: u32 = 0x20;
pub const DK_UNIFORM_BUF_ALIGNMENT: u32 = 0x100;
pub const DK_UNIFORM_BUF_MAX_SIZE: u32 = 0x10000;
pub const DK_NUM_STORAGE_BUFS: u32 = 16;
pub const DK_NUM_IMAGE_BINDINGS: u32 = 8;
pub const DK_QUEUE_MIN_CMDMEM_SIZE: u32 = 0x10000;
pub const DK_PER_WARP_SCRATCH_MEM_ALIGNMENT: u32 = 0x200;
pub const DK_DEFAULT_MAX_COMPUTE_CONCURRENT_JOBS: u32 = 128;

pub const DkMemAccess_None: u32 = 0;
pub const DkMemAccess_Uncached: u32 = 1;
pub const DkMemAccess_Cached: u32 = 2;
pub const DkMemAccess_Mask: u32 = 3;

pub const DkMemBlockFlags_CpuAccessShift: u32 = 0;
pub const DkMemBlockFlags_GpuAccessShift: u32 = 2;
pub const DkMemBlockFlags_CpuUncached: u32 = DkMemAccess_Uncached << DkMemBlockFlags_CpuAccessShift;
pub const DkMemBlockFlags_CpuCached: u32 = DkMemAccess_Cached << DkMemBlockFlags_CpuAccessShift;
pub const DkMemBlockFlags_CpuAccessMask: u32 = DkMemAccess_Mask << DkMemBlockFlags_CpuAccessShift;
pub const DkMemBlockFlags_GpuUncached: u32 = DkMemAccess_Uncached << DkMemBlockFlags_GpuAccessShift;
pub const DkMemBlockFlags_GpuCached: u32 = DkMemAccess_Cached << DkMemBlockFlags_GpuAccessShift;
pub const DkMemBlockFlags_GpuAccessMask: u32 = DkMemAccess_Mask << DkMemBlockFlags_GpuAccessShift;
pub const DkMemBlockFlags_Code: u32 = 1 << 4;
pub const DkMemBlockFlags_Image: u32 = 1 << 5;
pub const DkMemBlockFlags_ZeroFillInit: u32 = 1 << 8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkMemBlockMaker {
    pub device: DkDevice,
    pub size: u32,
    pub flags: u32,
    pub storage: *mut c_void,
}

impl DkMemBlockMaker {
    pub const fn defaults(device: DkDevice, size: u32) -> Self {
        Self {
            device,
            size,
            flags: DkMemBlockFlags_CpuUncached | DkMemBlockFlags_GpuCached,
            storage: core::ptr::null_mut(),
        }
    }
}

pub const DkQueueFlags_Graphics: u32 = 1 << 0;
pub const DkQueueFlags_Compute: u32 = 1 << 1;
pub const DkQueueFlags_MediumPrio: u32 = 0 << 2;
pub const DkQueueFlags_HighPrio: u32 = 1 << 2;
pub const DkQueueFlags_LowPrio: u32 = 2 << 2;
pub const DkQueueFlags_PrioMask: u32 = 3 << 2;
pub const DkQueueFlags_EnableZcull: u32 = 0 << 4;
pub const DkQueueFlags_DisableZcull: u32 = 1 << 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkCounter {
    DkCounter_TimestampPipelineTop = 0,
    DkCounter_Timestamp = 1,
    DkCounter_SamplesPassed = 2,
    DkCounter_VertexShaderInvocations = 6,
    DkCounter_FragmentShaderInvocations = 10,
    DkCounter_ClipperInputPrimitives = 13,
    DkCounter_ClipperOutputPrimitives = 14,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkCmdBufMaker {
    pub device: DkDevice,
    pub userData: *mut c_void,
    pub cbAddMem: DkCmdBufAddMemFunc,
}

impl DkCmdBufMaker {
    pub const fn defaults(device: DkDevice) -> Self {
        Self {
            device,
            userData: core::ptr::null_mut(),
            cbAddMem: None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkQueueMaker {
    pub device: DkDevice,
    pub flags: u32,
    pub commandMemorySize: u32,
    pub flushThreshold: u32,
    pub perWarpScratchMemorySize: u32,
    pub maxConcurrentComputeJobs: u32,
}

impl DkQueueMaker {
    pub const fn defaults(device: DkDevice) -> Self {
        Self {
            device,
            flags: DkQueueFlags_Graphics
                | DkQueueFlags_Compute
                | DkQueueFlags_MediumPrio
                | DkQueueFlags_EnableZcull,
            commandMemorySize: DK_QUEUE_MIN_CMDMEM_SIZE,
            flushThreshold: DK_QUEUE_MIN_CMDMEM_SIZE / 8,
            perWarpScratchMemorySize: 4 * DK_PER_WARP_SCRATCH_MEM_ALIGNMENT,
            maxConcurrentComputeJobs: DK_DEFAULT_MAX_COMPUTE_CONCURRENT_JOBS,
        }
    }
}

#[repr(C, align(8))]
pub struct DkShader {
    _storage: [u8; 128],
}

impl DkShader {
    pub const fn zeroed() -> Self {
        Self { _storage: [0; 128] }
    }
}

#[repr(C, align(8))]
pub struct DkImageLayout {
    _storage: [u8; 128],
}

impl DkImageLayout {
    pub const fn zeroed() -> Self {
        Self { _storage: [0; 128] }
    }
}

#[repr(C, align(8))]
pub struct DkImage {
    _storage: [u8; 128],
}

impl DkImage {
    pub const fn zeroed() -> Self {
        Self { _storage: [0; 128] }
    }
}

#[repr(C, align(4))]
pub struct DkImageDescriptor {
    _storage: [u8; 32],
}

impl DkImageDescriptor {
    pub const fn zeroed() -> Self {
        Self { _storage: [0; 32] }
    }
}

#[repr(C, align(4))]
pub struct DkSamplerDescriptor {
    _storage: [u8; 32],
}

impl DkSamplerDescriptor {
    pub const fn zeroed() -> Self {
        Self { _storage: [0; 32] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkImageType {
    DkImageType_None = 0,
    DkImageType_1D = 1,
    DkImageType_2D = 2,
    DkImageType_3D = 3,
    DkImageType_1DArray = 4,
    DkImageType_2DArray = 5,
    DkImageType_2DMS = 6,
    DkImageType_2DMSArray = 7,
    DkImageType_Rectangle = 8,
    DkImageType_Cubemap = 9,
    DkImageType_CubemapArray = 10,
    DkImageType_Buffer = 11,
}

pub const DkImageFlags_BlockLinear: u32 = 0 << 0;
pub const DkImageFlags_PitchLinear: u32 = 1 << 0;
pub const DkImageFlags_CustomTileSize: u32 = 1 << 1;
pub const DkImageFlags_HwCompression: u32 = 1 << 2;
pub const DkImageFlags_Z16EnableZbc: u32 = 1 << 3;
pub const DkImageFlags_UsageRender: u32 = 1 << 8;
pub const DkImageFlags_UsageLoadStore: u32 = 1 << 9;
pub const DkImageFlags_UsagePresent: u32 = 1 << 10;
pub const DkImageFlags_Usage2DEngine: u32 = 1 << 11;
pub const DkImageFlags_UsageVideo: u32 = 1 << 12;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkImageFormat {
    DkImageFormat_None,
    DkImageFormat_R8_Unorm,
    DkImageFormat_R8_Snorm,
    DkImageFormat_R8_Uint,
    DkImageFormat_R8_Sint,
    DkImageFormat_R16_Float,
    DkImageFormat_R16_Unorm,
    DkImageFormat_R16_Snorm,
    DkImageFormat_R16_Uint,
    DkImageFormat_R16_Sint,
    DkImageFormat_R32_Float,
    DkImageFormat_R32_Uint,
    DkImageFormat_R32_Sint,
    DkImageFormat_RG8_Unorm,
    DkImageFormat_RG8_Snorm,
    DkImageFormat_RG8_Uint,
    DkImageFormat_RG8_Sint,
    DkImageFormat_RG16_Float,
    DkImageFormat_RG16_Unorm,
    DkImageFormat_RG16_Snorm,
    DkImageFormat_RG16_Uint,
    DkImageFormat_RG16_Sint,
    DkImageFormat_RG32_Float,
    DkImageFormat_RG32_Uint,
    DkImageFormat_RG32_Sint,
    DkImageFormat_RGB32_Float,
    DkImageFormat_RGB32_Uint,
    DkImageFormat_RGB32_Sint,
    DkImageFormat_RGBA8_Unorm,
    DkImageFormat_RGBA8_Snorm,
    DkImageFormat_RGBA8_Uint,
    DkImageFormat_RGBA8_Sint,
    DkImageFormat_RGBA16_Float,
    DkImageFormat_RGBA16_Unorm,
    DkImageFormat_RGBA16_Snorm,
    DkImageFormat_RGBA16_Uint,
    DkImageFormat_RGBA16_Sint,
    DkImageFormat_RGBA32_Float,
    DkImageFormat_RGBA32_Uint,
    DkImageFormat_RGBA32_Sint,
    DkImageFormat_S8,
    DkImageFormat_Z16,
    DkImageFormat_Z24X8,
    DkImageFormat_ZF32,
    DkImageFormat_Z24S8,
    DkImageFormat_ZF32_X24S8,
    DkImageFormat_RGBX8_Unorm_sRGB,
    DkImageFormat_RGBA8_Unorm_sRGB,
    DkImageFormat_E5BGR9_Float = 55,
    DkImageFormat_BGRA8_Unorm = 117,
    DkImageFormat_BGRA8_Unorm_sRGB = 119,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkMsMode {
    DkMsMode_1x = 0,
    DkMsMode_2x = 1,
    DkMsMode_4x = 2,
    DkMsMode_8x = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkCoverageModulation {
    DkCoverageModulation_None = 0,
    DkCoverageModulation_Rgb = 1,
    DkCoverageModulation_Alpha = 2,
    DkCoverageModulation_Rgba = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkMultisampleState {
    pub bits: u32,
    pub sampleLocations: [u32; 4],
}

impl DkMultisampleState {
    pub const fn defaults() -> Self {
        Self {
            bits: 1 << 7,
            sampleLocations: [0x8888_8888; 4],
        }
    }

    pub fn set_mode(&mut self, mode: DkMsMode) {
        self.bits = (self.bits & !0b111) | ((mode as u32) & 0b111);
    }

    pub fn set_rasterizer_mode(&mut self, mode: DkMsMode) {
        self.bits = (self.bits & !(0b111 << 3)) | (((mode as u32) & 0b111) << 3);
    }

    pub fn set_alpha_to_coverage_enable(&mut self, enable: bool) {
        self.bits = (self.bits & !(1 << 6)) | (u32::from(enable) << 6);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkTileSize {
    DkTileSize_OneGob = 0,
    DkTileSize_TwoGobs = 1,
    DkTileSize_FourGobs = 2,
    DkTileSize_EightGobs = 3,
    DkTileSize_SixteenGobs = 4,
    DkTileSize_ThirtyTwoGobs = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkImageLayoutMaker {
    pub device: DkDevice,
    pub type_: DkImageType,
    pub flags: u32,
    pub format: DkImageFormat,
    pub msMode: DkMsMode,
    pub dimensions: [u32; 3],
    pub mipLevels: u32,
    pub pitchStride: u32,
}

impl DkImageLayoutMaker {
    pub const fn defaults(device: DkDevice) -> Self {
        Self {
            device,
            type_: DkImageType::DkImageType_2D,
            flags: 0,
            format: DkImageFormat::DkImageFormat_None,
            msMode: DkMsMode::DkMsMode_1x,
            dimensions: [0; 3],
            mipLevels: 1,
            pitchStride: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkImageSwizzle {
    DkImageSwizzle_Zero = 0,
    DkImageSwizzle_One = 1,
    DkImageSwizzle_Red = 2,
    DkImageSwizzle_Green = 3,
    DkImageSwizzle_Blue = 4,
    DkImageSwizzle_Alpha = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkDsSource {
    DkDsSource_Depth = 0,
    DkDsSource_Stencil = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkImageView {
    pub pImage: *const DkImage,
    pub type_: DkImageType,
    pub format: DkImageFormat,
    pub swizzle: [DkImageSwizzle; 4],
    pub dsSource: DkDsSource,
    pub layerOffset: u16,
    pub layerCount: u16,
    pub mipLevelOffset: u8,
    pub mipLevelCount: u8,
}

impl DkImageView {
    pub const fn defaults(pImage: *const DkImage) -> Self {
        Self {
            pImage,
            type_: DkImageType::DkImageType_None,
            format: DkImageFormat::DkImageFormat_None,
            swizzle: [
                DkImageSwizzle::DkImageSwizzle_Red,
                DkImageSwizzle::DkImageSwizzle_Green,
                DkImageSwizzle::DkImageSwizzle_Blue,
                DkImageSwizzle::DkImageSwizzle_Alpha,
            ],
            dsSource: DkDsSource::DkDsSource_Depth,
            layerOffset: 0,
            layerCount: 0,
            mipLevelOffset: 0,
            mipLevelCount: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkViewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub near: f32,
    pub far: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkScissor {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkRasterizerState {
    pub bits: u32,
}

pub const DkFace_None: u32 = 0;
pub const DkFace_Front: u32 = 1;
pub const DkFace_Back: u32 = 2;
pub const DkFace_FrontAndBack: u32 = 3;

pub const DkFrontFace_CW: u32 = 0;
pub const DkFrontFace_CCW: u32 = 1;

impl DkRasterizerState {
    pub const fn defaults() -> Self {
        Self {
            bits: 1 | (2 << 3) | (2 << 5) | (2 << 7) | (1 << 9) | (1 << 10),
        }
    }

    pub fn set_cull_mode(&mut self, cull_mode: u32) {
        self.bits = (self.bits & !(0b11 << 7)) | ((cull_mode & 0b11) << 7);
    }

    pub fn set_front_face(&mut self, front_face: u32) {
        self.bits = (self.bits & !(0b1 << 9)) | ((front_face & 0b1) << 9);
    }

    pub fn set_depth_bias_enable(&mut self, enable: bool) {
        self.bits = (self.bits & !(0b111 << 14)) | (u32::from(enable) * 0b100 << 14);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkColorState {
    pub bits: u32,
}

impl DkColorState {
    pub const fn defaults() -> Self {
        Self {
            bits: (3 << 8) | (8 << 16),
        }
    }

    pub fn set_blend_enable(&mut self, id: u32, enable: bool) {
        let bit = 1u32 << id;
        if enable {
            self.bits |= bit;
        } else {
            self.bits &= !bit;
        }
    }
}

pub const DkColorMask_R: u32 = 1 << 0;
pub const DkColorMask_G: u32 = 1 << 1;
pub const DkColorMask_B: u32 = 1 << 2;
pub const DkColorMask_A: u32 = 1 << 3;
pub const DkColorMask_RGB: u32 = DkColorMask_R | DkColorMask_G | DkColorMask_B;
pub const DkColorMask_RGBA: u32 = DkColorMask_RGB | DkColorMask_A;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkColorWriteState {
    pub masks: u32,
}

impl DkColorWriteState {
    pub const fn defaults() -> Self {
        Self { masks: 0xFFFF_FFFF }
    }

    pub fn set_mask(&mut self, id: u32, color_write_mask: u32) {
        self.masks = (self.masks & !(0xF << (id * 4))) | ((color_write_mask & 0xF) << (id * 4));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkBlendOp {
    DkBlendOp_Add = 1,
    DkBlendOp_Sub = 2,
    DkBlendOp_RevSub = 3,
    DkBlendOp_Min = 4,
    DkBlendOp_Max = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkBlendFactor {
    DkBlendFactor_Zero = 1,
    DkBlendFactor_One = 2,
    DkBlendFactor_SrcColor = 3,
    DkBlendFactor_InvSrcColor = 4,
    DkBlendFactor_SrcAlpha = 5,
    DkBlendFactor_InvSrcAlpha = 6,
    DkBlendFactor_DstAlpha = 7,
    DkBlendFactor_InvDstAlpha = 8,
    DkBlendFactor_DstColor = 9,
    DkBlendFactor_InvDstColor = 10,
    DkBlendFactor_SrcAlphaSaturate = 11,
    DkBlendFactor_Src1Color = 16,
    DkBlendFactor_InvSrc1Color = 17,
    DkBlendFactor_Src1Alpha = 18,
    DkBlendFactor_InvSrc1Alpha = 19,
    DkBlendFactor_ConstColor = 1 | 0x20,
    DkBlendFactor_InvConstColor = 2 | 0x20,
    DkBlendFactor_ConstAlpha = 3 | 0x20,
    DkBlendFactor_InvConstAlpha = 4 | 0x20,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkBlendState {
    pub bits: u32,
}

impl DkBlendState {
    pub const fn defaults() -> Self {
        Self {
            bits: (DkBlendOp::DkBlendOp_Add as u32)
                | ((DkBlendFactor::DkBlendFactor_SrcAlpha as u32) << 3)
                | ((DkBlendFactor::DkBlendFactor_InvSrcAlpha as u32) << 9)
                | ((DkBlendOp::DkBlendOp_Add as u32) << 15)
                | ((DkBlendFactor::DkBlendFactor_One as u32) << 18)
                | ((DkBlendFactor::DkBlendFactor_Zero as u32) << 24),
        }
    }

    pub fn set_ops(&mut self, color: DkBlendOp, alpha: DkBlendOp) {
        self.bits = (self.bits & !0b111) | ((color as u32) & 0b111);
        self.bits = (self.bits & !(0b111 << 15)) | (((alpha as u32) & 0b111) << 15);
    }

    pub fn set_factors(
        &mut self,
        src_color: DkBlendFactor,
        dst_color: DkBlendFactor,
        src_alpha: DkBlendFactor,
        dst_alpha: DkBlendFactor,
    ) {
        self.bits = (self.bits & !(0b11_1111 << 3)) | (((src_color as u32) & 0b11_1111) << 3);
        self.bits = (self.bits & !(0b11_1111 << 9)) | (((dst_color as u32) & 0b11_1111) << 9);
        self.bits = (self.bits & !(0b11_1111 << 18)) | (((src_alpha as u32) & 0b11_1111) << 18);
        self.bits = (self.bits & !(0b11_1111 << 24)) | (((dst_alpha as u32) & 0b11_1111) << 24);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkFilter {
    DkFilter_Nearest = 1,
    DkFilter_Linear = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkMipFilter {
    DkMipFilter_None = 1,
    DkMipFilter_Nearest = 2,
    DkMipFilter_Linear = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkWrapMode {
    DkWrapMode_Repeat = 0,
    DkWrapMode_MirroredRepeat = 1,
    DkWrapMode_ClampToEdge = 2,
    DkWrapMode_ClampToBorder = 3,
    DkWrapMode_Clamp = 4,
    DkWrapMode_MirrorClampToEdge = 5,
    DkWrapMode_MirrorClampToBorder = 6,
    DkWrapMode_MirrorClamp = 7,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkCompareOp {
    DkCompareOp_Never = 1,
    DkCompareOp_Less = 2,
    DkCompareOp_Equal = 3,
    DkCompareOp_Lequal = 4,
    DkCompareOp_Greater = 5,
    DkCompareOp_NotEqual = 6,
    DkCompareOp_Gequal = 7,
    DkCompareOp_Always = 8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkStencilOp {
    DkStencilOp_Keep = 1,
    DkStencilOp_Zero = 2,
    DkStencilOp_Replace = 3,
    DkStencilOp_Incr = 4,
    DkStencilOp_Decr = 5,
    DkStencilOp_Invert = 6,
    DkStencilOp_IncrWrap = 7,
    DkStencilOp_DecrWrap = 8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkDepthStencilState {
    pub depth_bits: u32,
    pub stencil_bits: u32,
}

impl DkDepthStencilState {
    pub const fn defaults() -> Self {
        Self {
            depth_bits: 1 | (1 << 1) | ((DkCompareOp::DkCompareOp_Less as u32) << 4),
            stencil_bits: (DkStencilOp::DkStencilOp_Keep as u32)
                | ((DkStencilOp::DkStencilOp_Replace as u32) << 4)
                | ((DkStencilOp::DkStencilOp_Keep as u32) << 8)
                | ((DkCompareOp::DkCompareOp_Always as u32) << 12)
                | ((DkStencilOp::DkStencilOp_Keep as u32) << 16)
                | ((DkStencilOp::DkStencilOp_Replace as u32) << 20)
                | ((DkStencilOp::DkStencilOp_Keep as u32) << 24)
                | ((DkCompareOp::DkCompareOp_Always as u32) << 28),
        }
    }

    pub fn set_depth_test_enable(&mut self, enable: bool) {
        self.set_depth_bit(0, enable);
    }

    pub fn set_depth_write_enable(&mut self, enable: bool) {
        self.set_depth_bit(1, enable);
    }

    pub fn set_stencil_test_enable(&mut self, enable: bool) {
        self.set_depth_bit(2, enable);
    }

    pub fn set_depth_compare_op(&mut self, op: DkCompareOp) {
        self.depth_bits = (self.depth_bits & !(0b1111 << 4)) | (((op as u32) & 0b1111) << 4);
    }

    pub fn set_stencil_front(
        &mut self,
        fail: DkStencilOp,
        pass: DkStencilOp,
        depth_fail: DkStencilOp,
        compare: DkCompareOp,
    ) {
        self.stencil_bits = (self.stencil_bits & !0x0000_FFFF)
            | ((fail as u32) & 0b1111)
            | (((pass as u32) & 0b1111) << 4)
            | (((depth_fail as u32) & 0b1111) << 8)
            | (((compare as u32) & 0b1111) << 12);
    }

    pub fn set_stencil_back(
        &mut self,
        fail: DkStencilOp,
        pass: DkStencilOp,
        depth_fail: DkStencilOp,
        compare: DkCompareOp,
    ) {
        self.stencil_bits = (self.stencil_bits & !0xFFFF_0000)
            | (((fail as u32) & 0b1111) << 16)
            | (((pass as u32) & 0b1111) << 20)
            | (((depth_fail as u32) & 0b1111) << 24)
            | (((compare as u32) & 0b1111) << 28);
    }

    fn set_depth_bit(&mut self, bit: u32, enable: bool) {
        let mask = 1u32 << bit;
        if enable {
            self.depth_bits |= mask;
        } else {
            self.depth_bits &= !mask;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkSamplerReduction {
    DkSamplerReduction_WeightedAverage = 0,
    DkSamplerReduction_Min = 1,
    DkSamplerReduction_Max = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union DkSamplerBorderColor {
    pub value_f: f32,
    pub value_ui: u32,
    pub value_i: i32,
}

impl DkSamplerBorderColor {
    pub const fn zeroed() -> Self {
        Self { value_ui: 0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkSampler {
    pub minFilter: DkFilter,
    pub magFilter: DkFilter,
    pub mipFilter: DkMipFilter,
    pub wrapMode: [DkWrapMode; 3],
    pub lodClampMin: f32,
    pub lodClampMax: f32,
    pub lodBias: f32,
    pub lodSnap: f32,
    pub compareEnable: bool,
    pub compareOp: DkCompareOp,
    pub borderColor: [DkSamplerBorderColor; 4],
    pub maxAnisotropy: f32,
    pub reductionMode: DkSamplerReduction,
}

impl DkSampler {
    pub const fn defaults() -> Self {
        Self {
            minFilter: DkFilter::DkFilter_Nearest,
            magFilter: DkFilter::DkFilter_Nearest,
            mipFilter: DkMipFilter::DkMipFilter_None,
            wrapMode: [
                DkWrapMode::DkWrapMode_Repeat,
                DkWrapMode::DkWrapMode_Repeat,
                DkWrapMode::DkWrapMode_Repeat,
            ],
            lodClampMin: 0.0,
            lodClampMax: 1000.0,
            lodBias: 0.0,
            lodSnap: 0.0,
            compareEnable: false,
            compareOp: DkCompareOp::DkCompareOp_Less,
            borderColor: [DkSamplerBorderColor::zeroed(); 4],
            maxAnisotropy: 1.0,
            reductionMode: DkSamplerReduction::DkSamplerReduction_WeightedAverage,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkBufExtents {
    pub addr: DkGpuAddr,
    pub size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkVtxAttribSize {
    DkVtxAttribSize_1x32 = 0x12,
    DkVtxAttribSize_2x32 = 0x04,
    DkVtxAttribSize_3x32 = 0x02,
    DkVtxAttribSize_4x32 = 0x01,
    DkVtxAttribSize_1x16 = 0x1b,
    DkVtxAttribSize_2x16 = 0x0f,
    DkVtxAttribSize_3x16 = 0x05,
    DkVtxAttribSize_4x16 = 0x03,
    DkVtxAttribSize_1x8 = 0x1d,
    DkVtxAttribSize_2x8 = 0x18,
    DkVtxAttribSize_3x8 = 0x13,
    DkVtxAttribSize_4x8 = 0x0a,
    DkVtxAttribSize_10_10_10_2 = 0x30,
    DkVtxAttribSize_11_11_10 = 0x31,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkVtxAttribType {
    DkVtxAttribType_None = 0,
    DkVtxAttribType_Snorm = 1,
    DkVtxAttribType_Unorm = 2,
    DkVtxAttribType_Sint = 3,
    DkVtxAttribType_Uint = 4,
    DkVtxAttribType_Uscaled = 5,
    DkVtxAttribType_Sscaled = 6,
    DkVtxAttribType_Float = 7,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkVtxAttribState {
    pub bits: u32,
}

impl DkVtxAttribState {
    pub const fn new(
        buffer_id: u32,
        is_fixed: bool,
        offset: u32,
        size: DkVtxAttribSize,
        type_: DkVtxAttribType,
        is_bgra: bool,
    ) -> Self {
        Self {
            bits: (buffer_id & 0x1f)
                | ((is_fixed as u32) << 6)
                | ((offset & 0x3fff) << 7)
                | (((size as u32) & 0x3f) << 21)
                | (((type_ as u32) & 0x7) << 27)
                | ((is_bgra as u32) << 31),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkVtxBufferState {
    pub stride: u32,
    pub divisor: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkStage {
    DkStage_Vertex = 0,
    DkStage_TessCtrl = 1,
    DkStage_TessEval = 2,
    DkStage_Geometry = 3,
    DkStage_Fragment = 4,
    DkStage_Compute = 5,
}

pub const DkStageFlag_Vertex: u32 = 1 << DkStage::DkStage_Vertex as u32;
pub const DkStageFlag_TessCtrl: u32 = 1 << DkStage::DkStage_TessCtrl as u32;
pub const DkStageFlag_TessEval: u32 = 1 << DkStage::DkStage_TessEval as u32;
pub const DkStageFlag_Geometry: u32 = 1 << DkStage::DkStage_Geometry as u32;
pub const DkStageFlag_Fragment: u32 = 1 << DkStage::DkStage_Fragment as u32;
pub const DkStageFlag_Compute: u32 = 1 << DkStage::DkStage_Compute as u32;
pub const DkStageFlag_GraphicsMask: u32 = (1 << DkStage::DkStage_Compute as u32) - 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkBarrier {
    DkBarrier_None = 0,
    DkBarrier_Tiles = 1,
    DkBarrier_Fragments = 2,
    DkBarrier_Primitives = 3,
    DkBarrier_Full = 4,
}

pub const DkInvalidateFlags_Image: u32 = 1 << 0;
pub const DkInvalidateFlags_Shader: u32 = 1 << 1;
pub const DkInvalidateFlags_Descriptors: u32 = 1 << 2;
pub const DkInvalidateFlags_Zcull: u32 = 1 << 3;
pub const DkInvalidateFlags_L2Cache: u32 = 1 << 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkPrimitive {
    DkPrimitive_Points = 0,
    DkPrimitive_Lines = 1,
    DkPrimitive_LineLoop = 2,
    DkPrimitive_LineStrip = 3,
    DkPrimitive_Triangles = 4,
    DkPrimitive_TriangleStrip = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DkIdxFormat {
    DkIdxFormat_Uint8 = 0,
    DkIdxFormat_Uint16 = 1,
    DkIdxFormat_Uint32 = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DkDrawIndirectData {
    pub vertexCount: u32,
    pub instanceCount: u32,
    pub firstVertex: u32,
    pub firstInstance: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DkDrawIndexedIndirectData {
    pub indexCount: u32,
    pub instanceCount: u32,
    pub firstIndex: u32,
    pub vertexOffset: i32,
    pub firstInstance: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DkDispatchIndirectData {
    pub numGroupsX: u32,
    pub numGroupsY: u32,
    pub numGroupsZ: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkShaderMaker {
    pub codeMem: DkMemBlock,
    pub control: *const c_void,
    pub codeOffset: u32,
    pub programId: u32,
}

impl DkShaderMaker {
    pub const fn defaults(codeMem: DkMemBlock, codeOffset: u32) -> Self {
        Self {
            codeMem,
            control: core::ptr::null(),
            codeOffset,
            programId: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkImageRect {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkCopyBuf {
    pub addr: DkGpuAddr,
    pub rowLength: u32,
    pub imageHeight: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DkSwapchainMaker {
    pub device: DkDevice,
    pub nativeWindow: *mut c_void,
    pub pImages: *const *const DkImage,
    pub numImages: u32,
}

impl DkSwapchainMaker {
    pub const fn defaults(
        device: DkDevice,
        nativeWindow: *mut c_void,
        pImages: *const *const DkImage,
        numImages: u32,
    ) -> Self {
        Self {
            device,
            nativeWindow,
            pImages,
            numImages,
        }
    }
}

unsafe extern "C" {
    pub fn dkDeviceCreate(maker: *const DkDeviceMaker) -> DkDevice;
    pub fn dkDeviceDestroy(obj: DkDevice);

    pub fn dkMemBlockCreate(maker: *const DkMemBlockMaker) -> DkMemBlock;
    pub fn dkMemBlockDestroy(obj: DkMemBlock);
    pub fn dkMemBlockGetCpuAddr(obj: DkMemBlock) -> *mut c_void;
    pub fn dkMemBlockGetGpuAddr(obj: DkMemBlock) -> DkGpuAddr;
    pub fn dkMemBlockGetSize(obj: DkMemBlock) -> u32;
    pub fn dkMemBlockFlushCpuCache(obj: DkMemBlock, offset: u32, size: u32) -> DkResult;
    pub fn dkFenceWait(obj: *mut DkFence, timeout_ns: i64) -> DkResult;

    pub fn dkCmdBufCreate(maker: *const DkCmdBufMaker) -> DkCmdBuf;
    pub fn dkCmdBufDestroy(obj: DkCmdBuf);
    pub fn dkCmdBufAddMemory(obj: DkCmdBuf, mem: DkMemBlock, offset: u32, size: u32);
    pub fn dkCmdBufFinishList(obj: DkCmdBuf) -> DkCmdList;
    pub fn dkCmdBufClear(obj: DkCmdBuf);
    pub fn dkCmdBufBarrier(obj: DkCmdBuf, mode: DkBarrier, invalidateFlags: u32);
    pub fn dkCmdBufBindRenderTargets(
        obj: DkCmdBuf,
        colorTargets: *const *const DkImageView,
        numColorTargets: u32,
        depthTarget: *const DkImageView,
    );
    pub fn dkCmdBufSetViewports(
        obj: DkCmdBuf,
        firstId: u32,
        viewports: *const DkViewport,
        numViewports: u32,
    );
    pub fn dkCmdBufSetScissors(
        obj: DkCmdBuf,
        firstId: u32,
        scissors: *const DkScissor,
        numScissors: u32,
    );
    pub fn dkCmdBufClearColor(
        obj: DkCmdBuf,
        targetId: u32,
        clearMask: u32,
        clearData: *const c_void,
    );
    pub fn dkCmdBufBindShaders(
        obj: DkCmdBuf,
        stageMask: u32,
        shaders: *const *const DkShader,
        numShaders: u32,
    );
    pub fn dkCmdBufBindRasterizerState(obj: DkCmdBuf, state: *const DkRasterizerState);
    pub fn dkCmdBufBindColorState(obj: DkCmdBuf, state: *const DkColorState);
    pub fn dkCmdBufBindColorWriteState(obj: DkCmdBuf, state: *const DkColorWriteState);
    pub fn dkCmdBufBindBlendStates(
        obj: DkCmdBuf,
        firstId: u32,
        states: *const DkBlendState,
        numStates: u32,
    );
    pub fn dkCmdBufBindDepthStencilState(obj: DkCmdBuf, state: *const DkDepthStencilState);
    pub fn dkCmdBufBindMultisampleState(obj: DkCmdBuf, state: *const DkMultisampleState);
    pub fn dkCmdBufSetBlendConst(obj: DkCmdBuf, red: f32, green: f32, blue: f32, alpha: f32);
    pub fn dkCmdBufSetSampleMask(obj: DkCmdBuf, mask: u32);
    pub fn dkCmdBufSetDepthBias(obj: DkCmdBuf, constantFactor: f32, clamp: f32, slopeFactor: f32);
    pub fn dkCmdBufSetPrimitiveRestart(obj: DkCmdBuf, enable: bool, index: u32);
    pub fn dkCmdBufSetStencil(obj: DkCmdBuf, face: u32, mask: u8, funcRef: u8, funcMask: u8);
    pub fn dkCmdBufClearDepthStencil(
        obj: DkCmdBuf,
        clearDepth: bool,
        depthValue: f32,
        stencilMask: u8,
        stencilValue: u8,
    );
    pub fn dkCmdBufDiscardColor(obj: DkCmdBuf, targetId: u32);
    pub fn dkCmdBufDiscardDepthStencil(obj: DkCmdBuf);
    pub fn dkCmdBufResolveImage(
        obj: DkCmdBuf,
        srcView: *const DkImageView,
        dstView: *const DkImageView,
    );
    pub fn dkCmdBufPushConstants(
        obj: DkCmdBuf,
        uboAddr: DkGpuAddr,
        uboSize: u32,
        offset: u32,
        size: u32,
        data: *const c_void,
    );
    pub fn dkCmdBufPushData(obj: DkCmdBuf, addr: DkGpuAddr, data: *const c_void, size: u32);
    pub fn dkCmdBufBindTextures(
        obj: DkCmdBuf,
        stage: DkStage,
        firstId: u32,
        handles: *const DkResHandle,
        numHandles: u32,
    );
    pub fn dkCmdBufBindImages(
        obj: DkCmdBuf,
        stage: DkStage,
        firstId: u32,
        handles: *const DkResHandle,
        numHandles: u32,
    );
    pub fn dkCmdBufBindImageDescriptorSet(obj: DkCmdBuf, setAddr: DkGpuAddr, numDescriptors: u32);
    pub fn dkCmdBufBindSamplerDescriptorSet(obj: DkCmdBuf, setAddr: DkGpuAddr, numDescriptors: u32);
    pub fn dkCmdBufBindUniformBuffers(
        obj: DkCmdBuf,
        stage: DkStage,
        firstId: u32,
        buffers: *const DkBufExtents,
        numBuffers: u32,
    );
    pub fn dkCmdBufBindStorageBuffers(
        obj: DkCmdBuf,
        stage: DkStage,
        firstId: u32,
        buffers: *const DkBufExtents,
        numBuffers: u32,
    );
    pub fn dkCmdBufBindVtxAttribState(
        obj: DkCmdBuf,
        attribs: *const DkVtxAttribState,
        numAttribs: u32,
    );
    pub fn dkCmdBufBindVtxBufferState(
        obj: DkCmdBuf,
        buffers: *const DkVtxBufferState,
        numBuffers: u32,
    );
    pub fn dkCmdBufBindVtxBuffers(
        obj: DkCmdBuf,
        firstId: u32,
        buffers: *const DkBufExtents,
        numBuffers: u32,
    );
    pub fn dkCmdBufBindIdxBuffer(obj: DkCmdBuf, format: DkIdxFormat, address: DkGpuAddr);
    pub fn dkCmdBufDraw(
        obj: DkCmdBuf,
        prim: DkPrimitive,
        vertexCount: u32,
        instanceCount: u32,
        firstVertex: u32,
        firstInstance: u32,
    );
    pub fn dkCmdBufDrawIndirect(obj: DkCmdBuf, prim: DkPrimitive, indirect: DkGpuAddr);
    pub fn dkCmdBufDrawIndexed(
        obj: DkCmdBuf,
        prim: DkPrimitive,
        indexCount: u32,
        instanceCount: u32,
        firstIndex: u32,
        vertexOffset: i32,
        firstInstance: u32,
    );
    pub fn dkCmdBufDrawIndexedIndirect(obj: DkCmdBuf, prim: DkPrimitive, indirect: DkGpuAddr);
    pub fn dkCmdBufDispatchCompute(
        obj: DkCmdBuf,
        numGroupsX: u32,
        numGroupsY: u32,
        numGroupsZ: u32,
    );
    pub fn dkCmdBufDispatchComputeIndirect(obj: DkCmdBuf, indirect: DkGpuAddr);
    pub fn dkCmdBufCopyImage(
        obj: DkCmdBuf,
        srcView: *const DkImageView,
        srcRect: *const DkImageRect,
        dstView: *const DkImageView,
        dstRect: *const DkImageRect,
        flags: u32,
    );
    pub fn dkCmdBufCopyBufferToImage(
        obj: DkCmdBuf,
        src: *const DkCopyBuf,
        dstView: *const DkImageView,
        dstRect: *const DkImageRect,
        flags: u32,
    );
    pub fn dkCmdBufCopyImageToBuffer(
        obj: DkCmdBuf,
        srcView: *const DkImageView,
        srcRect: *const DkImageRect,
        dst: *const DkCopyBuf,
        flags: u32,
    );
    pub fn dkCmdBufCopyBuffer(obj: DkCmdBuf, srcAddr: DkGpuAddr, dstAddr: DkGpuAddr, size: u32);
    pub fn dkCmdBufReportCounter(obj: DkCmdBuf, type_: DkCounter, addr: DkGpuAddr);
    pub fn dkCmdBufReportValue(obj: DkCmdBuf, value: u32, addr: DkGpuAddr);
    pub fn dkCmdBufResetCounter(obj: DkCmdBuf, type_: DkCounter);

    pub fn dkQueueCreate(maker: *const DkQueueMaker) -> DkQueue;
    pub fn dkQueueDestroy(obj: DkQueue);
    pub fn dkQueueWaitFence(obj: DkQueue, fence: *mut DkFence);
    pub fn dkQueueSignalFence(obj: DkQueue, fence: *mut DkFence, flush: bool);
    pub fn dkQueueWaitIdle(obj: DkQueue);
    pub fn dkQueueAcquireImage(obj: DkQueue, swapchain: DkSwapchain) -> c_int;
    pub fn dkQueueSubmitCommands(obj: DkQueue, cmds: DkCmdList);
    pub fn dkQueueFlush(obj: DkQueue);
    pub fn dkQueuePresentImage(obj: DkQueue, swapchain: DkSwapchain, imageSlot: c_int);

    pub fn dkShaderInitialize(obj: *mut DkShader, maker: *const DkShaderMaker);
    pub fn dkShaderIsValid(obj: *const DkShader) -> bool;

    pub fn dkImageLayoutInitialize(obj: *mut DkImageLayout, maker: *const DkImageLayoutMaker);
    pub fn dkImageLayoutGetSize(obj: *const DkImageLayout) -> u64;
    pub fn dkImageLayoutGetAlignment(obj: *const DkImageLayout) -> u32;

    pub fn dkImageInitialize(
        obj: *mut DkImage,
        layout: *const DkImageLayout,
        memBlock: DkMemBlock,
        offset: u32,
    );
    pub fn dkImageDescriptorInitialize(
        obj: *mut DkImageDescriptor,
        view: *const DkImageView,
        usesLoadOrStore: bool,
        decayMS: bool,
    );
    pub fn dkSamplerDescriptorInitialize(obj: *mut DkSamplerDescriptor, sampler: *const DkSampler);

    pub fn dkSwapchainCreate(maker: *const DkSwapchainMaker) -> DkSwapchain;
    pub fn dkSwapchainDestroy(obj: DkSwapchain);

    pub fn nwindowGetDefault() -> *mut NWindow;

    pub fn romfsMountSelf(name: *const c_char) -> u32;
    pub fn romfsUnmount(name: *const c_char) -> u32;
    pub fn appletMainLoop() -> bool;

    pub fn padConfigureInput(max_players: u32, style_set: u32);
    pub fn padInitializeWithMask(pad: *mut PadState, mask: u64);
    pub fn padUpdate(pad: *mut PadState);

    pub fn diagAbortWithResult(res: u32) -> !;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fseek(stream: *mut FILE, offset: c_long, whence: c_int) -> c_int;
    pub fn ftell(stream: *mut FILE) -> c_long;
    pub fn rewind(stream: *mut FILE);
    pub fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut FILE) -> usize;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HidAnalogStickState {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PadState {
    pub id_mask: u8,
    pub active_id_mask: u8,
    pub read_handheld: bool,
    pub active_handheld: bool,
    pub style_set: u32,
    pub attributes: u32,
    pub buttons_cur: u64,
    pub buttons_old: u64,
    pub sticks: [HidAnalogStickState; 2],
    pub gc_triggers: [u32; 2],
}

impl PadState {
    pub const fn zeroed() -> Self {
        Self {
            id_mask: 0,
            active_id_mask: 0,
            read_handheld: false,
            active_handheld: false,
            style_set: 0,
            attributes: 0,
            buttons_cur: 0,
            buttons_old: 0,
            sticks: [HidAnalogStickState { x: 0, y: 0 }; 2],
            gc_triggers: [0; 2],
        }
    }

    pub const fn buttons_down(&self) -> u64 {
        !self.buttons_old & self.buttons_cur
    }
}

pub const PAD_ANY_ID_MASK: u64 = 0x1000100FF;
pub const HidNpadIdType_No1: u32 = 0;
pub const HidNpadIdType_Handheld: u32 = 0x20;
pub const HidNpadStyleTag_NpadFullKey: u32 = 1 << 0;
pub const HidNpadStyleTag_NpadHandheld: u32 = 1 << 1;
pub const HidNpadStyleTag_NpadJoyDual: u32 = 1 << 2;
pub const HidNpadStyleTag_NpadJoyLeft: u32 = 1 << 3;
pub const HidNpadStyleTag_NpadJoyRight: u32 = 1 << 4;
pub const HidNpadStyleSet_NpadFullCtrl: u32 =
    HidNpadStyleTag_NpadFullKey | HidNpadStyleTag_NpadHandheld | HidNpadStyleTag_NpadJoyDual;
pub const HidNpadStyleSet_NpadStandard: u32 =
    HidNpadStyleSet_NpadFullCtrl | HidNpadStyleTag_NpadJoyLeft | HidNpadStyleTag_NpadJoyRight;
pub const HidNpadButton_Plus: u64 = 1 << 10;

pub const SEEK_END: c_int = 2;
pub const SEEK_SET: c_int = 0;

pub unsafe fn romfsInit() -> u32 {
    unsafe { romfsMountSelf(c"romfs".as_ptr()) }
}

pub unsafe fn romfsExit() -> u32 {
    unsafe { romfsUnmount(c"romfs".as_ptr()) }
}

pub unsafe fn padInitializeDefault(pad: *mut PadState) {
    const MASK: u64 = (1 << HidNpadIdType_No1) | (1 << HidNpadIdType_Handheld);
    unsafe { padInitializeWithMask(pad, MASK) };
}

pub unsafe fn dkCmdBufBindRenderTarget(
    obj: DkCmdBuf,
    colorTarget: *const DkImageView,
    depthTarget: *const DkImageView,
) {
    unsafe { dkCmdBufBindRenderTargets(obj, &colorTarget, 1, depthTarget) };
}

pub unsafe fn dkCmdBufClearColorFloat(
    obj: DkCmdBuf,
    targetId: u32,
    clearMask: u32,
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
) {
    let data = [red, green, blue, alpha];
    unsafe { dkCmdBufClearColor(obj, targetId, clearMask, data.as_ptr().cast()) };
}

pub unsafe fn dkCmdBufBindVtxBuffer(obj: DkCmdBuf, id: u32, bufAddr: DkGpuAddr, bufSize: u32) {
    let ext = DkBufExtents {
        addr: bufAddr,
        size: bufSize,
    };
    unsafe { dkCmdBufBindVtxBuffers(obj, id, &ext, 1) };
}

pub unsafe fn dkCmdBufBindUniformBuffer(
    obj: DkCmdBuf,
    stage: DkStage,
    id: u32,
    bufAddr: DkGpuAddr,
    bufSize: u32,
) {
    let ext = DkBufExtents {
        addr: bufAddr,
        size: bufSize,
    };
    unsafe { dkCmdBufBindUniformBuffers(obj, stage, id, &ext, 1) };
}

pub unsafe fn dkCmdBufBindStorageBuffer(
    obj: DkCmdBuf,
    stage: DkStage,
    id: u32,
    bufAddr: DkGpuAddr,
    bufSize: u32,
) {
    let ext = DkBufExtents {
        addr: bufAddr,
        size: bufSize,
    };
    unsafe { dkCmdBufBindStorageBuffers(obj, stage, id, &ext, 1) };
}

pub const fn dkMakeImageHandle(id: u32) -> DkResHandle {
    id & ((1 << 20) - 1)
}

pub const fn dkMakeSamplerHandle(id: u32) -> DkResHandle {
    id << 20
}

pub const fn dkMakeTextureHandle(imageId: u32, samplerId: u32) -> DkResHandle {
    dkMakeImageHandle(imageId) | dkMakeSamplerHandle(samplerId)
}

pub unsafe fn dkCmdBufBindTexture(obj: DkCmdBuf, stage: DkStage, id: u32, handle: DkResHandle) {
    unsafe { dkCmdBufBindTextures(obj, stage, id, &handle, 1) };
}

pub unsafe fn dkCmdBufBindImage(obj: DkCmdBuf, stage: DkStage, id: u32, handle: DkResHandle) {
    unsafe { dkCmdBufBindImages(obj, stage, id, &handle, 1) };
}

pub fn milestone_1_compile_smoke() {
    let _ = core::mem::size_of::<DkDeviceMaker>();
    let _ = core::mem::size_of::<DkQueue>();
    let _ = core::mem::size_of::<DkCmdBuf>();
    let _ = core::mem::size_of::<DkSwapchainMaker>();
    let _ = dkDeviceCreate as unsafe extern "C" fn(*const DkDeviceMaker) -> DkDevice;
    let _ = dkCmdBufDispatchCompute as unsafe extern "C" fn(DkCmdBuf, u32, u32, u32);
    let _ = dkCmdBufDispatchComputeIndirect as unsafe extern "C" fn(DkCmdBuf, DkGpuAddr);
    let _ = dkCmdBufSetDepthBias as unsafe extern "C" fn(DkCmdBuf, f32, f32, f32);
    let _ = dkCmdBufSetPrimitiveRestart as unsafe extern "C" fn(DkCmdBuf, bool, u32);
    let _ = nwindowGetDefault as unsafe extern "C" fn() -> *mut NWindow;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_1_surface_is_nameable() {
        let _device_maker = DkDeviceMaker::defaults();
        let _cmd_maker = DkCmdBufMaker::defaults(core::ptr::null_mut());
        let _queue_maker = DkQueueMaker::defaults(core::ptr::null_mut());
        let _swapchain_maker = DkSwapchainMaker::defaults(
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null(),
            0,
        );
        milestone_1_compile_smoke();
    }

    #[test]
    fn bgra8_image_format_values_match_deko3d() {
        assert_eq!(DkImageFormat::DkImageFormat_BGRA8_Unorm as u32, 117);
        assert_eq!(DkImageFormat::DkImageFormat_BGRA8_Unorm_sRGB as u32, 119);
    }

    #[test]
    fn depth_bias_enable_uses_the_fill_polygon_bit() {
        let mut state = DkRasterizerState::defaults();
        state.set_depth_bias_enable(true);
        assert_eq!((state.bits >> 14) & 0b111, 0b100);
        state.set_depth_bias_enable(false);
        assert_eq!((state.bits >> 14) & 0b111, 0);
    }

    #[test]
    fn alpha_to_coverage_uses_the_native_multisample_bit() {
        let mut state = DkMultisampleState::defaults();
        state.set_alpha_to_coverage_enable(true);
        assert_eq!((state.bits >> 6) & 1, 1);
        state.set_alpha_to_coverage_enable(false);
        assert_eq!((state.bits >> 6) & 1, 0);
    }
}
