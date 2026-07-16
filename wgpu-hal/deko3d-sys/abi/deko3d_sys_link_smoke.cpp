#include <cstdint>

#include <deko3d.h>
#include <switch.h>

int main() {
    volatile std::uintptr_t symbols[] = {
        reinterpret_cast<std::uintptr_t>(&dkDeviceCreate),
        reinterpret_cast<std::uintptr_t>(&dkDeviceDestroy),
        reinterpret_cast<std::uintptr_t>(&dkMemBlockCreate),
        reinterpret_cast<std::uintptr_t>(&dkMemBlockDestroy),
        reinterpret_cast<std::uintptr_t>(&dkMemBlockGetSize),
        reinterpret_cast<std::uintptr_t>(&dkMemBlockFlushCpuCache),
        reinterpret_cast<std::uintptr_t>(&dkFenceWait),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufCreate),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDestroy),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufFinishList),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufBindBlendStates),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufClearDepthStencil),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufBindDepthStencilState),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufBindMultisampleState),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufSetBlendConst),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufSetSampleMask),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufSetDepthBias),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufSetPrimitiveRestart),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufSetStencil),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDiscardColor),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDiscardDepthStencil),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufResolveImage),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufPushConstants),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufBindStorageBuffers),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDrawIndirect),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDrawIndexedIndirect),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDispatchCompute),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDispatchComputeIndirect),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufReportCounter),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufReportValue),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufResetCounter),
        reinterpret_cast<std::uintptr_t>(&dkQueueCreate),
        reinterpret_cast<std::uintptr_t>(&dkQueueDestroy),
        reinterpret_cast<std::uintptr_t>(&dkQueueSubmitCommands),
        reinterpret_cast<std::uintptr_t>(&dkQueuePresentImage),
        reinterpret_cast<std::uintptr_t>(&dkShaderInitialize),
        reinterpret_cast<std::uintptr_t>(&dkImageInitialize),
        reinterpret_cast<std::uintptr_t>(&dkSwapchainCreate),
        reinterpret_cast<std::uintptr_t>(&nwindowGetDefault),
    };
    return symbols[0] == 0;
}
