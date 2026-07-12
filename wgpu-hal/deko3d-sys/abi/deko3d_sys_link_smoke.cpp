#include <cstdint>

#include <deko3d.h>
#include <switch.h>

int main() {
    volatile std::uintptr_t symbols[] = {
        reinterpret_cast<std::uintptr_t>(&dkDeviceCreate),
        reinterpret_cast<std::uintptr_t>(&dkDeviceDestroy),
        reinterpret_cast<std::uintptr_t>(&dkMemBlockCreate),
        reinterpret_cast<std::uintptr_t>(&dkMemBlockDestroy),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufCreate),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufDestroy),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufFinishList),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufClearDepthStencil),
        reinterpret_cast<std::uintptr_t>(&dkCmdBufBindDepthStencilState),
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
