#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <type_traits>

#include <deko3d.h>
#include <switch.h>

#define CHECK(name, expr) static_assert((expr), name)

constexpr uint32_t rasterizer_bits(DkRasterizerState state) {
    return (uint32_t(state.rasterizerEnable) << 0) |
           (uint32_t(state.depthClampEnable) << 1) |
           (uint32_t(state.fillRectangleEnable) << 2) |
           (uint32_t(state.polygonModeFront) << 3) |
           (uint32_t(state.polygonModeBack) << 5) |
           (uint32_t(state.cullMode) << 7) |
           (uint32_t(state.frontFace) << 9) |
           (uint32_t(state.provokingVertex) << 10) |
           (uint32_t(state.polygonSmoothEnableMask) << 11) |
           (uint32_t(state.depthBiasEnableMask) << 14);
}

constexpr DkRasterizerState default_rasterizer() {
    DkRasterizerState state = {};
    dkRasterizerStateDefaults(&state);
    return state;
}

constexpr uint32_t color_bits(DkColorState state) {
    return (uint32_t(state.blendEnableMask) << 0) |
           (uint32_t(state.logicOp) << 8) |
           (uint32_t(state.alphaCompareOp) << 16);
}

constexpr DkColorState default_color() {
    DkColorState state = {};
    dkColorStateDefaults(&state);
    return state;
}

constexpr DkColorWriteState default_color_write() {
    DkColorWriteState state = {};
    dkColorWriteStateDefaults(&state);
    return state;
}

constexpr uint32_t blend_bits(DkBlendState state) {
    return (uint32_t(state.colorBlendOp) << 0) |
           (uint32_t(state.srcColorBlendFactor) << 3) |
           (uint32_t(state.dstColorBlendFactor) << 9) |
           (uint32_t(state.alphaBlendOp) << 15) |
           (uint32_t(state.srcAlphaBlendFactor) << 18) |
           (uint32_t(state.dstAlphaBlendFactor) << 24);
}

constexpr DkBlendState default_blend() {
    DkBlendState state = {};
    dkBlendStateDefaults(&state);
    return state;
}

constexpr uint32_t depth_stencil_depth_bits(DkDepthStencilState state) {
    return (uint32_t(state.depthTestEnable) << 0) |
           (uint32_t(state.depthWriteEnable) << 1) |
           (uint32_t(state.stencilTestEnable) << 2) |
           (uint32_t(state.depthCompareOp) << 4);
}

constexpr uint32_t depth_stencil_stencil_bits(DkDepthStencilState state) {
    return (uint32_t(state.stencilFrontFailOp) << 0) |
           (uint32_t(state.stencilFrontPassOp) << 4) |
           (uint32_t(state.stencilFrontDepthFailOp) << 8) |
           (uint32_t(state.stencilFrontCompareOp) << 12) |
           (uint32_t(state.stencilBackFailOp) << 16) |
           (uint32_t(state.stencilBackPassOp) << 20) |
           (uint32_t(state.stencilBackDepthFailOp) << 24) |
           (uint32_t(state.stencilBackCompareOp) << 28);
}

constexpr DkDepthStencilState default_depth_stencil() {
    DkDepthStencilState state = {};
    dkDepthStencilStateDefaults(&state);
    return state;
}

constexpr uint32_t vtx_attrib_bits(DkVtxAttribState state) {
    return (uint32_t(state.bufferId) << 0) |
           (uint32_t(state.isFixed) << 6) |
           (uint32_t(state.offset) << 7) |
           (uint32_t(state.size) << 21) |
           (uint32_t(state.type) << 27) |
           (uint32_t(state.isBgra) << 31);
}

constexpr DkVtxAttribState vtx_attrib(
    uint32_t bufferId,
    uint32_t isFixed,
    uint32_t offset,
    DkVtxAttribSize size,
    DkVtxAttribType type,
    uint32_t isBgra) {
    return DkVtxAttribState{bufferId, isFixed, offset, size, type, isBgra};
}

CHECK("DkGpuAddr size", sizeof(DkGpuAddr) == 8);
CHECK("DkCmdList size", sizeof(DkCmdList) == sizeof(uintptr_t));
CHECK("DkResHandle size", sizeof(DkResHandle) == 4);
CHECK("DkFence align", alignof(DkFence) == 8);
CHECK("DkFence size", sizeof(DkFence) == 64);
CHECK("DkCounter size", sizeof(DkCounter) == 4);
CHECK("DkCounter_Timestamp value", DkCounter_Timestamp == 1);
CHECK("DK_NUM_STORAGE_BUFS", DK_NUM_STORAGE_BUFS == 16);
CHECK("DK_SHADER_CODE_UNUSABLE_SIZE", DK_SHADER_CODE_UNUSABLE_SIZE == 0x400);
CHECK("DK_IMAGE_DESCRIPTOR_ALIGNMENT", DK_IMAGE_DESCRIPTOR_ALIGNMENT == 0x20);
CHECK("DK_SAMPLER_DESCRIPTOR_ALIGNMENT", DK_SAMPLER_DESCRIPTOR_ALIGNMENT == 0x20);
CHECK("DK_IMAGE_LINEAR_STRIDE_ALIGNMENT", DK_IMAGE_LINEAR_STRIDE_ALIGNMENT == 0x20);
CHECK("DK_UNIFORM_BUF_ALIGNMENT", DK_UNIFORM_BUF_ALIGNMENT == 0x100);
CHECK("DK_UNIFORM_BUF_MAX_SIZE", DK_UNIFORM_BUF_MAX_SIZE == 0x10000);
CHECK("DkImageFormat_BGRA8_Unorm value", DkImageFormat_BGRA8_Unorm == 117);
CHECK("DkImageFormat_BGRA8_Unorm_sRGB value", DkImageFormat_BGRA8_Unorm_sRGB == 119);

CHECK("DkDeviceMaker size", sizeof(DkDeviceMaker) == 40);
CHECK("DkDeviceMaker flags offset", offsetof(DkDeviceMaker, flags) == 32);

CHECK("DkMemBlockMaker size", sizeof(DkMemBlockMaker) == 24);
CHECK("DkMemBlockMaker storage offset", offsetof(DkMemBlockMaker, storage) == 16);

CHECK("DkCmdBufMaker size", sizeof(DkCmdBufMaker) == 24);
CHECK("DkCmdBufMaker cbAddMem offset", offsetof(DkCmdBufMaker, cbAddMem) == 16);

CHECK("DkQueueMaker size", sizeof(DkQueueMaker) == 32);
CHECK("DkQueueMaker flags offset", offsetof(DkQueueMaker, flags) == 8);
CHECK("DkQueueMaker max jobs offset", offsetof(DkQueueMaker, maxConcurrentComputeJobs) == 24);

CHECK("DkShaderMaker size", sizeof(DkShaderMaker) == 24);
CHECK("DkShaderMaker programId offset", offsetof(DkShaderMaker, programId) == 20);

CHECK("DkShader align", alignof(DkShader) == 8);
CHECK("DkShader size", sizeof(DkShader) == 128);
CHECK("DkImageLayout align", alignof(DkImageLayout) == 8);
CHECK("DkImageLayout size", sizeof(DkImageLayout) == 128);
CHECK("DkImage align", alignof(DkImage) == 8);
CHECK("DkImage size", sizeof(DkImage) == 128);
CHECK("DkImageDescriptor align", alignof(DkImageDescriptor) == 4);
CHECK("DkImageDescriptor size", sizeof(DkImageDescriptor) == 32);
CHECK("DkSamplerDescriptor align", alignof(DkSamplerDescriptor) == 4);
CHECK("DkSamplerDescriptor size", sizeof(DkSamplerDescriptor) == 32);

CHECK("DkImageFormat_RGBA8_Unorm", DkImageFormat_RGBA8_Unorm == 28);
CHECK("DkImageFormat_S8", DkImageFormat_S8 == 40);
CHECK("DkImageFormat_ZF32", DkImageFormat_ZF32 == 43);
CHECK("DkImageFormat_RGBA8_Unorm_sRGB", DkImageFormat_RGBA8_Unorm_sRGB == 47);
CHECK("DkImageLayoutMaker size", sizeof(DkImageLayoutMaker) == 48);
CHECK("DkImageLayoutMaker dimensions offset", offsetof(DkImageLayoutMaker, dimensions) == 24);
CHECK("DkImageLayoutMaker pitchStride offset", offsetof(DkImageLayoutMaker, pitchStride) == 40);
CHECK("DkMultisampleState align", alignof(DkMultisampleState) == 4);
CHECK("DkMultisampleState size", sizeof(DkMultisampleState) == 20);

CHECK("DkImageView size", sizeof(DkImageView) == 48);
CHECK("DkImageView swizzle offset", offsetof(DkImageView, swizzle) == 16);
CHECK("DkImageView layerOffset offset", offsetof(DkImageView, layerOffset) == 36);

CHECK("DkViewport size", sizeof(DkViewport) == 24);
CHECK("DkScissor size", sizeof(DkScissor) == 16);

CHECK("DkFace_None", DkFace_None == 0);
CHECK("DkFace_Front", DkFace_Front == 1);
CHECK("DkFace_Back", DkFace_Back == 2);
CHECK("DkFace_FrontAndBack", DkFace_FrontAndBack == 3);
CHECK("DkFrontFace_CW", DkFrontFace_CW == 0);
CHECK("DkFrontFace_CCW", DkFrontFace_CCW == 1);
CHECK("DkRasterizerState size", sizeof(DkRasterizerState) == 4);
CHECK("DkRasterizerState defaults", rasterizer_bits(default_rasterizer()) == 1873);

CHECK("DkColorState size", sizeof(DkColorState) == 4);
CHECK("DkColorState defaults", color_bits(default_color()) == 525056);

CHECK("DkColorWriteState size", sizeof(DkColorWriteState) == 4);
CHECK("DkColorWriteState defaults", default_color_write().masks == 0xFFFFFFFF);

CHECK("DkBlendOp_Add", DkBlendOp_Add == 1);
CHECK("DkBlendOp_Max", DkBlendOp_Max == 5);
CHECK("DkBlendFactor_Zero", DkBlendFactor_Zero == 1);
CHECK("DkBlendFactor_SrcAlpha", DkBlendFactor_SrcAlpha == 5);
CHECK("DkBlendFactor_InvSrcAlpha", DkBlendFactor_InvSrcAlpha == 6);
CHECK("DkBlendFactor_InvSrc1Alpha", DkBlendFactor_InvSrc1Alpha == 19);
CHECK("DkBlendFactor_ConstColor", DkBlendFactor_ConstColor == 33);
CHECK("DkBlendState size", sizeof(DkBlendState) == 4);
CHECK("DkBlendState defaults", blend_bits(default_blend()) == 17337385);

CHECK("DkStencilOp_Keep", DkStencilOp_Keep == 1);
CHECK("DkStencilOp_DecrWrap", DkStencilOp_DecrWrap == 8);
CHECK("DkDepthStencilState size", sizeof(DkDepthStencilState) == 8);
CHECK("DkDepthStencilState depth defaults", depth_stencil_depth_bits(default_depth_stencil()) == 35);
CHECK(
    "DkDepthStencilState stencil defaults",
    depth_stencil_stencil_bits(default_depth_stencil()) == 2167505201U);

CHECK("DkFilter_Nearest", DkFilter_Nearest == 1);
CHECK("DkFilter_Linear", DkFilter_Linear == 2);
CHECK("DkMipFilter_None", DkMipFilter_None == 1);
CHECK("DkWrapMode_ClampToEdge", DkWrapMode_ClampToEdge == 2);
CHECK("DkCompareOp_Less", DkCompareOp_Less == 2);
CHECK("DkSamplerReduction_WeightedAverage", DkSamplerReduction_WeightedAverage == 0);
CHECK("DkSampler size", sizeof(DkSampler) == 72);
CHECK("DkSampler compareEnable offset", offsetof(DkSampler, compareEnable) == 40);
CHECK("DkSampler compareOp offset", offsetof(DkSampler, compareOp) == 44);
CHECK("DkSampler borderColor offset", offsetof(DkSampler, borderColor) == 48);
CHECK("DkSampler reductionMode offset", offsetof(DkSampler, reductionMode) == 68);

CHECK("DkBufExtents size", sizeof(DkBufExtents) == 16);
CHECK("DkBufExtents addr offset", offsetof(DkBufExtents, addr) == 0);
CHECK("DkBufExtents size offset", offsetof(DkBufExtents, size) == 8);
CHECK("dkMakeImageHandle", dkMakeImageHandle(3) == 3);
CHECK("dkMakeSamplerHandle", dkMakeSamplerHandle(2) == (2U << 20));
CHECK("dkMakeTextureHandle", dkMakeTextureHandle(3, 2) == ((2U << 20) | 3U));

CHECK("DkVtxAttribSize_3x32", DkVtxAttribSize_3x32 == 0x02);
CHECK("DkVtxAttribType_Float", DkVtxAttribType_Float == 7);
CHECK("DkVtxAttribState size", sizeof(DkVtxAttribState) == 4);
CHECK(
    "DkVtxAttribState position bits",
    vtx_attrib_bits(vtx_attrib(0, 0, 0, DkVtxAttribSize_3x32, DkVtxAttribType_Float, 0)) ==
        ((uint32_t(DkVtxAttribSize_3x32) << 21) |
         (uint32_t(DkVtxAttribType_Float) << 27)));
CHECK(
    "DkVtxAttribState color bits",
    vtx_attrib_bits(vtx_attrib(0, 0, 12, DkVtxAttribSize_3x32, DkVtxAttribType_Float, 0)) ==
        ((12U << 7) |
         (uint32_t(DkVtxAttribSize_3x32) << 21) |
         (uint32_t(DkVtxAttribType_Float) << 27)));

CHECK("DkVtxBufferState size", sizeof(DkVtxBufferState) == 8);
CHECK("DkVtxBufferState stride offset", offsetof(DkVtxBufferState, stride) == 0);
CHECK("DkVtxBufferState divisor offset", offsetof(DkVtxBufferState, divisor) == 4);

CHECK("DkStageFlag_GraphicsMask", DkStageFlag_GraphicsMask == 31);
CHECK("DkStageFlag_Compute", DkStageFlag_Compute == 32);
CHECK("DkBarrier_None", DkBarrier_None == 0);
CHECK("DkBarrier_Primitives", DkBarrier_Primitives == 3);
CHECK("DkBarrier_Full", DkBarrier_Full == 4);
CHECK("DkInvalidateFlags_Image", DkInvalidateFlags_Image == (1U << 0));
CHECK("DkInvalidateFlags_Shader", DkInvalidateFlags_Shader == (1U << 1));
CHECK("DkInvalidateFlags_Descriptors", DkInvalidateFlags_Descriptors == (1U << 2));
CHECK("DkInvalidateFlags_Zcull", DkInvalidateFlags_Zcull == (1U << 3));
CHECK("DkInvalidateFlags_L2Cache", DkInvalidateFlags_L2Cache == (1U << 4));
CHECK("DkPrimitive_Triangles", DkPrimitive_Triangles == 4);
CHECK("DkPrimitive_TriangleStrip", DkPrimitive_TriangleStrip == 5);
CHECK("DkIdxFormat_Uint16", DkIdxFormat_Uint16 == 1);
CHECK("DkIdxFormat_Uint32", DkIdxFormat_Uint32 == 2);
CHECK("DkDispatchIndirectData size", sizeof(DkDispatchIndirectData) == 12);
CHECK("DkDispatchIndirectData numGroupsX offset", offsetof(DkDispatchIndirectData, numGroupsX) == 0);
CHECK("DkDispatchIndirectData numGroupsY offset", offsetof(DkDispatchIndirectData, numGroupsY) == 4);
CHECK("DkDispatchIndirectData numGroupsZ offset", offsetof(DkDispatchIndirectData, numGroupsZ) == 8);
CHECK(
    "dkCmdBufDispatchCompute signature",
    (std::is_same<decltype(&dkCmdBufDispatchCompute), void (*)(DkCmdBuf, uint32_t, uint32_t, uint32_t)>::value));
CHECK(
    "dkCmdBufDispatchComputeIndirect signature",
    (std::is_same<decltype(&dkCmdBufDispatchComputeIndirect), void (*)(DkCmdBuf, DkGpuAddr)>::value));

CHECK("DkImageRect size", sizeof(DkImageRect) == 24);
CHECK("DkImageRect width offset", offsetof(DkImageRect, width) == 12);
CHECK("DkCopyBuf size", sizeof(DkCopyBuf) == 16);
CHECK("DkCopyBuf addr offset", offsetof(DkCopyBuf, addr) == 0);
CHECK("DkCopyBuf rowLength offset", offsetof(DkCopyBuf, rowLength) == 8);
CHECK("DkCopyBuf imageHeight offset", offsetof(DkCopyBuf, imageHeight) == 12);

CHECK("DkSwapchainMaker size", sizeof(DkSwapchainMaker) == 32);
CHECK("DkSwapchainMaker pImages offset", offsetof(DkSwapchainMaker, pImages) == 16);

CHECK("HidAnalogStickState size", sizeof(HidAnalogStickState) == 8);
CHECK("PadState size", sizeof(PadState) == 56);
CHECK("PadState buttons_cur offset", offsetof(PadState, buttons_cur) == 16);
CHECK("HidNpadButton_Plus", HidNpadButton_Plus == (1UL << 10));
CHECK("PAD_ANY_ID_MASK", PAD_ANY_ID_MASK == 0x1000100FFUL);
CHECK("HidNpadIdType_Handheld", HidNpadIdType_Handheld == 0x20);

using DkDeviceCreateSig = DkDevice (*)(DkDeviceMaker const*);
using DkCmdBufClearSig = void (*)(DkCmdBuf);
using DkCmdBufBindRenderTargetsSig =
    void (*)(DkCmdBuf, DkImageView const* const*, uint32_t, DkImageView const*);
using DkCmdBufPushDataSig = void (*)(DkCmdBuf, DkGpuAddr, void const*, uint32_t);
using DkCmdBufBarrierSig = void (*)(DkCmdBuf, DkBarrier, uint32_t);
using DkCmdBufBindTexturesSig =
    void (*)(DkCmdBuf, DkStage, uint32_t, DkResHandle const*, uint32_t);
using DkCmdBufBindImagesSig =
    void (*)(DkCmdBuf, DkStage, uint32_t, DkResHandle const*, uint32_t);
using DkCmdBufBindImageDescriptorSetSig = void (*)(DkCmdBuf, DkGpuAddr, uint32_t);
using DkCmdBufBindSamplerDescriptorSetSig = void (*)(DkCmdBuf, DkGpuAddr, uint32_t);
using DkCmdBufBindUniformBuffersSig =
    void (*)(DkCmdBuf, DkStage, uint32_t, DkBufExtents const*, uint32_t);
using DkCmdBufBindStorageBuffersSig =
    void (*)(DkCmdBuf, DkStage, uint32_t, DkBufExtents const*, uint32_t);
using DkCmdBufBindBlendStatesSig = void (*)(DkCmdBuf, uint32_t, DkBlendState const*, uint32_t);
using DkCmdBufSetBlendConstSig = void (*)(DkCmdBuf, float, float, float, float);
using DkCmdBufBindVtxAttribStateSig =
    void (*)(DkCmdBuf, DkVtxAttribState const*, uint32_t);
using DkCmdBufBindVtxBufferStateSig =
    void (*)(DkCmdBuf, DkVtxBufferState const*, uint32_t);
using DkCmdBufBindVtxBuffersSig =
    void (*)(DkCmdBuf, uint32_t, DkBufExtents const*, uint32_t);
using DkCmdBufBindIdxBufferSig = void (*)(DkCmdBuf, DkIdxFormat, DkGpuAddr);
using DkCmdBufDrawIndexedSig =
    void (*)(DkCmdBuf, DkPrimitive, uint32_t, uint32_t, uint32_t, int32_t, uint32_t);
using DkCmdBufCopyImageSig =
    void (*)(DkCmdBuf, DkImageView const*, DkImageRect const*, DkImageView const*, DkImageRect const*, uint32_t);
using DkCmdBufCopyBufferToImageSig =
    void (*)(DkCmdBuf, DkCopyBuf const*, DkImageView const*, DkImageRect const*, uint32_t);
using DkCmdBufCopyImageToBufferSig =
    void (*)(DkCmdBuf, DkImageView const*, DkImageRect const*, DkCopyBuf const*, uint32_t);
using DkCmdBufCopyBufferSig = void (*)(DkCmdBuf, DkGpuAddr, DkGpuAddr, uint32_t);
using DkCmdBufReportCounterSig = void (*)(DkCmdBuf, DkCounter, DkGpuAddr);
using DkImageDescriptorInitializeSig =
    void (*)(DkImageDescriptor*, DkImageView const*, bool, bool);
using DkSamplerDescriptorInitializeSig = void (*)(DkSamplerDescriptor*, DkSampler const*);
using DkCmdBufAddMemFuncSig = void (*)(void*, DkCmdBuf, size_t);
using DkQueueAcquireImageSig = int (*)(DkQueue, DkSwapchain);
using DkFenceWaitSig = DkResult (*)(DkFence*, int64_t);
using DkQueueWaitFenceSig = void (*)(DkQueue, DkFence*);
using DkQueueSignalFenceSig = void (*)(DkQueue, DkFence*, bool);
using DkQueueFlushSig = void (*)(DkQueue);
using NWindowGetDefaultSig = NWindow* (*)();
using RomfsMountSelfSig = Result (*)(const char*);
using PadConfigureInputSig = void (*)(uint32_t, uint32_t);
using DkDeviceCreateMatches = std::is_same<decltype(&dkDeviceCreate), DkDeviceCreateSig>;
using DkCmdBufClearMatches = std::is_same<decltype(&dkCmdBufClear), DkCmdBufClearSig>;
using DkCmdBufBindRenderTargetsMatches =
    std::is_same<decltype(&dkCmdBufBindRenderTargets), DkCmdBufBindRenderTargetsSig>;
using DkCmdBufPushDataMatches =
    std::is_same<decltype(&dkCmdBufPushData), DkCmdBufPushDataSig>;
using DkCmdBufBarrierMatches = std::is_same<decltype(&dkCmdBufBarrier), DkCmdBufBarrierSig>;
using DkCmdBufBindTexturesMatches =
    std::is_same<decltype(&dkCmdBufBindTextures), DkCmdBufBindTexturesSig>;
using DkCmdBufBindImagesMatches =
    std::is_same<decltype(&dkCmdBufBindImages), DkCmdBufBindImagesSig>;
using DkCmdBufBindImageDescriptorSetMatches =
    std::is_same<decltype(&dkCmdBufBindImageDescriptorSet), DkCmdBufBindImageDescriptorSetSig>;
using DkCmdBufBindSamplerDescriptorSetMatches =
    std::is_same<decltype(&dkCmdBufBindSamplerDescriptorSet), DkCmdBufBindSamplerDescriptorSetSig>;
using DkCmdBufBindUniformBuffersMatches =
    std::is_same<decltype(&dkCmdBufBindUniformBuffers), DkCmdBufBindUniformBuffersSig>;
using DkCmdBufBindStorageBuffersMatches =
    std::is_same<decltype(&dkCmdBufBindStorageBuffers), DkCmdBufBindStorageBuffersSig>;
using DkCmdBufBindBlendStatesMatches =
    std::is_same<decltype(&dkCmdBufBindBlendStates), DkCmdBufBindBlendStatesSig>;
using DkCmdBufSetBlendConstMatches =
    std::is_same<decltype(&dkCmdBufSetBlendConst), DkCmdBufSetBlendConstSig>;
using DkCmdBufBindVtxAttribStateMatches =
    std::is_same<decltype(&dkCmdBufBindVtxAttribState), DkCmdBufBindVtxAttribStateSig>;
using DkCmdBufBindVtxBufferStateMatches =
    std::is_same<decltype(&dkCmdBufBindVtxBufferState), DkCmdBufBindVtxBufferStateSig>;
using DkCmdBufBindVtxBuffersMatches =
    std::is_same<decltype(&dkCmdBufBindVtxBuffers), DkCmdBufBindVtxBuffersSig>;
using DkCmdBufBindIdxBufferMatches =
    std::is_same<decltype(&dkCmdBufBindIdxBuffer), DkCmdBufBindIdxBufferSig>;
using DkCmdBufDrawIndexedMatches =
    std::is_same<decltype(&dkCmdBufDrawIndexed), DkCmdBufDrawIndexedSig>;
using DkCmdBufCopyImageMatches =
    std::is_same<decltype(&dkCmdBufCopyImage), DkCmdBufCopyImageSig>;
using DkCmdBufCopyBufferToImageMatches =
    std::is_same<decltype(&dkCmdBufCopyBufferToImage), DkCmdBufCopyBufferToImageSig>;
using DkCmdBufCopyImageToBufferMatches =
    std::is_same<decltype(&dkCmdBufCopyImageToBuffer), DkCmdBufCopyImageToBufferSig>;
using DkCmdBufCopyBufferMatches =
    std::is_same<decltype(&dkCmdBufCopyBuffer), DkCmdBufCopyBufferSig>;
using DkCmdBufReportCounterMatches =
    std::is_same<decltype(&dkCmdBufReportCounter), DkCmdBufReportCounterSig>;
using DkImageDescriptorInitializeMatches =
    std::is_same<decltype(&dkImageDescriptorInitialize), DkImageDescriptorInitializeSig>;
using DkSamplerDescriptorInitializeMatches =
    std::is_same<decltype(&dkSamplerDescriptorInitialize), DkSamplerDescriptorInitializeSig>;
using DkCmdBufAddMemFuncMatches =
    std::is_same<decltype(DkCmdBufMaker{}.cbAddMem), DkCmdBufAddMemFuncSig>;
using DkQueueAcquireImageMatches =
    std::is_same<decltype(&dkQueueAcquireImage), DkQueueAcquireImageSig>;
using DkFenceWaitMatches = std::is_same<decltype(&dkFenceWait), DkFenceWaitSig>;
using DkQueueWaitFenceMatches =
    std::is_same<decltype(&dkQueueWaitFence), DkQueueWaitFenceSig>;
using DkQueueSignalFenceMatches =
    std::is_same<decltype(&dkQueueSignalFence), DkQueueSignalFenceSig>;
using DkQueueFlushMatches = std::is_same<decltype(&dkQueueFlush), DkQueueFlushSig>;
using NWindowGetDefaultMatches = std::is_same<decltype(&nwindowGetDefault), NWindowGetDefaultSig>;
using RomfsMountSelfMatches = std::is_same<decltype(&romfsMountSelf), RomfsMountSelfSig>;
using PadConfigureInputMatches = std::is_same<decltype(&padConfigureInput), PadConfigureInputSig>;

CHECK("dkDeviceCreate signature", DkDeviceCreateMatches::value);
CHECK("dkCmdBufClear signature", DkCmdBufClearMatches::value);
CHECK("dkCmdBufBindRenderTargets signature", DkCmdBufBindRenderTargetsMatches::value);
CHECK("dkCmdBufPushData signature", DkCmdBufPushDataMatches::value);
CHECK("dkCmdBufBarrier signature", DkCmdBufBarrierMatches::value);
CHECK("dkCmdBufBindTextures signature", DkCmdBufBindTexturesMatches::value);
CHECK("dkCmdBufBindImages signature", DkCmdBufBindImagesMatches::value);
CHECK("dkCmdBufBindImageDescriptorSet signature", DkCmdBufBindImageDescriptorSetMatches::value);
CHECK("dkCmdBufBindSamplerDescriptorSet signature", DkCmdBufBindSamplerDescriptorSetMatches::value);
CHECK("dkCmdBufBindUniformBuffers signature", DkCmdBufBindUniformBuffersMatches::value);
CHECK("dkCmdBufBindStorageBuffers signature", DkCmdBufBindStorageBuffersMatches::value);
CHECK("dkCmdBufBindBlendStates signature", DkCmdBufBindBlendStatesMatches::value);
CHECK("dkCmdBufSetBlendConst signature", DkCmdBufSetBlendConstMatches::value);
CHECK("dkCmdBufBindVtxAttribState signature", DkCmdBufBindVtxAttribStateMatches::value);
CHECK("dkCmdBufBindVtxBufferState signature", DkCmdBufBindVtxBufferStateMatches::value);
CHECK("dkCmdBufBindVtxBuffers signature", DkCmdBufBindVtxBuffersMatches::value);
CHECK("dkCmdBufBindIdxBuffer signature", DkCmdBufBindIdxBufferMatches::value);
CHECK("dkCmdBufDrawIndexed signature", DkCmdBufDrawIndexedMatches::value);
CHECK("dkCmdBufCopyImage signature", DkCmdBufCopyImageMatches::value);
CHECK("dkCmdBufCopyBufferToImage signature", DkCmdBufCopyBufferToImageMatches::value);
CHECK("dkCmdBufCopyImageToBuffer signature", DkCmdBufCopyImageToBufferMatches::value);
CHECK("dkCmdBufCopyBuffer signature", DkCmdBufCopyBufferMatches::value);
CHECK("dkCmdBufReportCounter signature", DkCmdBufReportCounterMatches::value);
CHECK("dkImageDescriptorInitialize signature", DkImageDescriptorInitializeMatches::value);
CHECK("dkSamplerDescriptorInitialize signature", DkSamplerDescriptorInitializeMatches::value);
CHECK("DkCmdBufMaker cbAddMem signature", DkCmdBufAddMemFuncMatches::value);
CHECK("dkQueueAcquireImage signature", DkQueueAcquireImageMatches::value);
CHECK("dkFenceWait signature", DkFenceWaitMatches::value);
CHECK("dkQueueWaitFence signature", DkQueueWaitFenceMatches::value);
CHECK("dkQueueSignalFence signature", DkQueueSignalFenceMatches::value);
CHECK("dkQueueFlush signature", DkQueueFlushMatches::value);
CHECK("nwindowGetDefault signature", NWindowGetDefaultMatches::value);
CHECK("romfsMountSelf signature", RomfsMountSelfMatches::value);
CHECK("padConfigureInput signature", PadConfigureInputMatches::value);
CHECK("malloc call return", (std::is_same<decltype(malloc(size_t{1})), void*>::value));
CHECK("free call return", (std::is_same<decltype(free(nullptr)), void>::value));

CHECK("dkDeviceDestroy signature", (std::is_same<decltype(&dkDeviceDestroy), void (*)(DkDevice)>::value));
CHECK("dkMemBlockCreate signature", (std::is_same<decltype(&dkMemBlockCreate), DkMemBlock (*)(DkMemBlockMaker const*)>::value));
CHECK("dkMemBlockDestroy signature", (std::is_same<decltype(&dkMemBlockDestroy), void (*)(DkMemBlock)>::value));
CHECK("dkMemBlockGetCpuAddr signature", (std::is_same<decltype(&dkMemBlockGetCpuAddr), void* (*)(DkMemBlock)>::value));
CHECK("dkMemBlockGetGpuAddr signature", (std::is_same<decltype(&dkMemBlockGetGpuAddr), DkGpuAddr (*)(DkMemBlock)>::value));
CHECK("dkCmdBufCreate signature", (std::is_same<decltype(&dkCmdBufCreate), DkCmdBuf (*)(DkCmdBufMaker const*)>::value));
CHECK("dkCmdBufDestroy signature", (std::is_same<decltype(&dkCmdBufDestroy), void (*)(DkCmdBuf)>::value));
CHECK("dkCmdBufAddMemory signature", (std::is_same<decltype(&dkCmdBufAddMemory), void (*)(DkCmdBuf, DkMemBlock, uint32_t, uint32_t)>::value));
CHECK("dkCmdBufFinishList signature", (std::is_same<decltype(&dkCmdBufFinishList), DkCmdList (*)(DkCmdBuf)>::value));
CHECK("dkCmdBufSetViewports signature", (std::is_same<decltype(&dkCmdBufSetViewports), void (*)(DkCmdBuf, uint32_t, DkViewport const*, uint32_t)>::value));
CHECK("dkCmdBufSetScissors signature", (std::is_same<decltype(&dkCmdBufSetScissors), void (*)(DkCmdBuf, uint32_t, DkScissor const*, uint32_t)>::value));
CHECK("dkCmdBufClearColor signature", (std::is_same<decltype(&dkCmdBufClearColor), void (*)(DkCmdBuf, uint32_t, uint32_t, void const*)>::value));
CHECK("dkCmdBufBindShaders signature", (std::is_same<decltype(&dkCmdBufBindShaders), void (*)(DkCmdBuf, uint32_t, DkShader const* const*, uint32_t)>::value));
CHECK("dkCmdBufBindRasterizerState signature", (std::is_same<decltype(&dkCmdBufBindRasterizerState), void (*)(DkCmdBuf, DkRasterizerState const*)>::value));
CHECK("dkCmdBufBindColorState signature", (std::is_same<decltype(&dkCmdBufBindColorState), void (*)(DkCmdBuf, DkColorState const*)>::value));
CHECK("dkCmdBufBindColorWriteState signature", (std::is_same<decltype(&dkCmdBufBindColorWriteState), void (*)(DkCmdBuf, DkColorWriteState const*)>::value));
CHECK("dkCmdBufDraw signature", (std::is_same<decltype(&dkCmdBufDraw), void (*)(DkCmdBuf, DkPrimitive, uint32_t, uint32_t, uint32_t, uint32_t)>::value));
CHECK("dkQueueCreate signature", (std::is_same<decltype(&dkQueueCreate), DkQueue (*)(DkQueueMaker const*)>::value));
CHECK("dkQueueDestroy signature", (std::is_same<decltype(&dkQueueDestroy), void (*)(DkQueue)>::value));
CHECK("dkQueueWaitIdle signature", (std::is_same<decltype(&dkQueueWaitIdle), void (*)(DkQueue)>::value));
CHECK("dkQueueSubmitCommands signature", (std::is_same<decltype(&dkQueueSubmitCommands), void (*)(DkQueue, DkCmdList)>::value));
CHECK("dkQueuePresentImage signature", (std::is_same<decltype(&dkQueuePresentImage), void (*)(DkQueue, DkSwapchain, int)>::value));
CHECK("dkShaderInitialize signature", (std::is_same<decltype(&dkShaderInitialize), void (*)(DkShader*, DkShaderMaker const*)>::value));
CHECK("dkShaderIsValid signature", (std::is_same<decltype(&dkShaderIsValid), bool (*)(DkShader const*)>::value));
CHECK("dkImageLayoutInitialize signature", (std::is_same<decltype(&dkImageLayoutInitialize), void (*)(DkImageLayout*, DkImageLayoutMaker const*)>::value));
CHECK("dkImageLayoutGetSize signature", (std::is_same<decltype(&dkImageLayoutGetSize), uint64_t (*)(DkImageLayout const*)>::value));
CHECK("dkImageLayoutGetAlignment signature", (std::is_same<decltype(&dkImageLayoutGetAlignment), uint32_t (*)(DkImageLayout const*)>::value));
CHECK("dkImageInitialize signature", (std::is_same<decltype(&dkImageInitialize), void (*)(DkImage*, DkImageLayout const*, DkMemBlock, uint32_t)>::value));
CHECK("dkSwapchainCreate signature", (std::is_same<decltype(&dkSwapchainCreate), DkSwapchain (*)(DkSwapchainMaker const*)>::value));
CHECK("dkSwapchainDestroy signature", (std::is_same<decltype(&dkSwapchainDestroy), void (*)(DkSwapchain)>::value));

int main() { return 0; }
