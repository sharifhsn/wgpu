#include <stdint.h>
#include <type_traits>

#include <deko3d.h>
#include <switch.h>

#define CHECK_FN(name, type) static_assert(std::is_same<decltype(&name), type>::value, #name)

CHECK_FN(dkDeviceCreate, DkDevice (*)(DkDeviceMaker const*));
CHECK_FN(dkDeviceDestroy, void (*)(DkDevice));
CHECK_FN(dkMemBlockCreate, DkMemBlock (*)(DkMemBlockMaker const*));
CHECK_FN(dkMemBlockDestroy, void (*)(DkMemBlock));
CHECK_FN(dkMemBlockGetCpuAddr, void* (*)(DkMemBlock));
CHECK_FN(dkMemBlockGetGpuAddr, DkGpuAddr (*)(DkMemBlock));
CHECK_FN(dkCmdBufCreate, DkCmdBuf (*)(DkCmdBufMaker const*));
CHECK_FN(dkCmdBufDestroy, void (*)(DkCmdBuf));
CHECK_FN(dkCmdBufAddMemory, void (*)(DkCmdBuf, DkMemBlock, uint32_t, uint32_t));
CHECK_FN(dkCmdBufFinishList, DkCmdList (*)(DkCmdBuf));
CHECK_FN(dkCmdBufClear, void (*)(DkCmdBuf));
CHECK_FN(dkCmdBufBindRenderTargets, void (*)(DkCmdBuf, DkImageView const* const*, uint32_t, DkImageView const*));
CHECK_FN(dkCmdBufSetViewports, void (*)(DkCmdBuf, uint32_t, DkViewport const*, uint32_t));
CHECK_FN(dkCmdBufSetScissors, void (*)(DkCmdBuf, uint32_t, DkScissor const*, uint32_t));
CHECK_FN(dkCmdBufClearColor, void (*)(DkCmdBuf, uint32_t, uint32_t, void const*));
CHECK_FN(dkCmdBufBindShaders, void (*)(DkCmdBuf, uint32_t, DkShader const* const*, uint32_t));
CHECK_FN(dkCmdBufBindRasterizerState, void (*)(DkCmdBuf, DkRasterizerState const*));
CHECK_FN(dkCmdBufBindColorState, void (*)(DkCmdBuf, DkColorState const*));
CHECK_FN(dkCmdBufBindColorWriteState, void (*)(DkCmdBuf, DkColorWriteState const*));
CHECK_FN(dkCmdBufPushData, void (*)(DkCmdBuf, DkGpuAddr, void const*, uint32_t));
CHECK_FN(dkCmdBufBindTextures, void (*)(DkCmdBuf, DkStage, uint32_t, DkResHandle const*, uint32_t));
CHECK_FN(dkCmdBufBindImages, void (*)(DkCmdBuf, DkStage, uint32_t, DkResHandle const*, uint32_t));
CHECK_FN(dkCmdBufBindImageDescriptorSet, void (*)(DkCmdBuf, DkGpuAddr, uint32_t));
CHECK_FN(dkCmdBufBindSamplerDescriptorSet, void (*)(DkCmdBuf, DkGpuAddr, uint32_t));
CHECK_FN(dkCmdBufBindUniformBuffers, void (*)(DkCmdBuf, DkStage, uint32_t, DkBufExtents const*, uint32_t));
CHECK_FN(dkCmdBufBindVtxAttribState, void (*)(DkCmdBuf, DkVtxAttribState const*, uint32_t));
CHECK_FN(dkCmdBufBindVtxBufferState, void (*)(DkCmdBuf, DkVtxBufferState const*, uint32_t));
CHECK_FN(dkCmdBufBindVtxBuffers, void (*)(DkCmdBuf, uint32_t, DkBufExtents const*, uint32_t));
CHECK_FN(dkCmdBufBindIdxBuffer, void (*)(DkCmdBuf, DkIdxFormat, DkGpuAddr));
CHECK_FN(dkCmdBufDraw, void (*)(DkCmdBuf, DkPrimitive, uint32_t, uint32_t, uint32_t, uint32_t));
CHECK_FN(dkCmdBufDrawIndexed, void (*)(DkCmdBuf, DkPrimitive, uint32_t, uint32_t, uint32_t, int32_t, uint32_t));
CHECK_FN(dkCmdBufCopyBufferToImage, void (*)(DkCmdBuf, DkCopyBuf const*, DkImageView const*, DkImageRect const*, uint32_t));
CHECK_FN(dkQueueCreate, DkQueue (*)(DkQueueMaker const*));
CHECK_FN(dkQueueDestroy, void (*)(DkQueue));
CHECK_FN(dkQueueWaitIdle, void (*)(DkQueue));
CHECK_FN(dkQueueAcquireImage, int (*)(DkQueue, DkSwapchain));
CHECK_FN(dkQueueSubmitCommands, void (*)(DkQueue, DkCmdList));
CHECK_FN(dkQueuePresentImage, void (*)(DkQueue, DkSwapchain, int));
CHECK_FN(dkShaderInitialize, void (*)(DkShader*, DkShaderMaker const*));
CHECK_FN(dkShaderIsValid, bool (*)(DkShader const*));
CHECK_FN(dkImageLayoutInitialize, void (*)(DkImageLayout*, DkImageLayoutMaker const*));
CHECK_FN(dkImageLayoutGetSize, uint64_t (*)(DkImageLayout const*));
CHECK_FN(dkImageLayoutGetAlignment, uint32_t (*)(DkImageLayout const*));
CHECK_FN(dkImageInitialize, void (*)(DkImage*, DkImageLayout const*, DkMemBlock, uint32_t));
CHECK_FN(dkImageDescriptorInitialize, void (*)(DkImageDescriptor*, DkImageView const*, bool, bool));
CHECK_FN(dkSamplerDescriptorInitialize, void (*)(DkSamplerDescriptor*, DkSampler const*));
CHECK_FN(dkSwapchainCreate, DkSwapchain (*)(DkSwapchainMaker const*));
CHECK_FN(dkSwapchainDestroy, void (*)(DkSwapchain));
CHECK_FN(nwindowGetDefault, NWindow* (*)());

int main() { return 0; }
