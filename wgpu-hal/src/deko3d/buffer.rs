use alloc::vec::Vec;
use core::{cell::UnsafeCell, ops::Range, ptr};

#[cfg(target_os = "horizon")]
use deko3d_sys as dk;

cfg_if::cfg_if! {
    if #[cfg(supports_ptr_atomics)] {
        use alloc::sync::Arc;
    } else if #[cfg(feature = "portable-atomic")] {
        use portable_atomic_util::Arc;
    }
}

#[derive(Clone, Debug)]
pub struct Buffer {
    /// This data is potentially accessed mutably in arbitrary non-overlapping slices,
    /// so we must store it in `UnsafeCell` to avoid making any too-strong no-aliasing claims.
    storage: Arc<UnsafeCell<[u8]>>,

    /// Size of the allocation.
    ///
    /// This is redundant with `storage.get().len()`, but that method is not
    /// available until our MSRV is 1.79 or greater.
    size: usize,

    #[cfg(target_os = "horizon")]
    gpu: Arc<GpuBuffer>,
}

#[cfg(target_os = "horizon")]
#[derive(Debug)]
struct GpuBuffer {
    mem_block: dk::DkMemBlock,
    gpu_addr: dk::DkGpuAddr,
}

/// SAFETY:
/// This shared mutable data will not be accessed in a way which causes data races;
/// the obligation to do so is on the caller of the HAL API.
/// For safe code, `wgpu-core` validation manages appropriate access.
unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Buffer {
    #[cfg(target_os = "horizon")]
    pub(super) fn new(
        desc: &crate::BufferDescriptor,
        raw_device: dk::DkDevice,
    ) -> Result<Self, crate::DeviceError> {
        let mut buffer = Self::new_host(desc)?;
        let allocation_size = align_up(
            u32::try_from(buffer.size.max(1)).map_err(|_| crate::DeviceError::OutOfMemory)?,
            dk::DK_MEMBLOCK_ALIGNMENT,
        );
        let mut mem_block_maker = dk::DkMemBlockMaker::defaults(raw_device, allocation_size);
        mem_block_maker.flags = dk::DkMemBlockFlags_CpuUncached
            | dk::DkMemBlockFlags_GpuCached
            | dk::DkMemBlockFlags_ZeroFillInit;
        let mem_block = unsafe { dk::dkMemBlockCreate(&mem_block_maker) };
        if mem_block.is_null() {
            return Err(crate::DeviceError::OutOfMemory);
        }
        let gpu_addr = unsafe { dk::dkMemBlockGetGpuAddr(mem_block) };
        if gpu_addr == u64::MAX {
            unsafe { dk::dkMemBlockDestroy(mem_block) };
            return Err(crate::DeviceError::Lost);
        }
        buffer.gpu = Arc::new(GpuBuffer {
            mem_block,
            gpu_addr,
        });
        Ok(buffer)
    }

    #[cfg(not(target_os = "horizon"))]
    pub(super) fn new(desc: &crate::BufferDescriptor) -> Result<Self, crate::DeviceError> {
        Self::new_host(desc)
    }

    fn new_host(desc: &crate::BufferDescriptor) -> Result<Self, crate::DeviceError> {
        let &crate::BufferDescriptor {
            label: _,
            size,
            usage: _,
            memory_flags: _,
        } = desc;

        let size = usize::try_from(size).map_err(|_| crate::DeviceError::OutOfMemory)?;

        let mut vector: Vec<u8> = Vec::new();
        vector
            .try_reserve_exact(size)
            .map_err(|_| crate::DeviceError::OutOfMemory)?;
        vector.resize(size, 0);
        let storage: Arc<[u8]> = Arc::from(vector);
        debug_assert_eq!(storage.len(), size);

        // SAFETY: `UnsafeCell<[u8]>` and `[u8]` have the same layout.
        // This is just adding a wrapper type without changing any layout,
        // because there is not currently a safe language/`std` way to accomplish this.
        let storage: Arc<UnsafeCell<[u8]>> =
            unsafe { Arc::from_raw(Arc::into_raw(storage) as *mut UnsafeCell<[u8]>) };

        Ok(Buffer {
            storage,
            size,
            #[cfg(target_os = "horizon")]
            gpu: Arc::new(GpuBuffer {
                mem_block: ptr::null_mut(),
                gpu_addr: u64::MAX,
            }),
        })
    }

    /// Returns a pointer to the memory owned by this buffer within the given `range`.
    ///
    /// This may be used to create any number of simultaneous pointers;
    /// aliasing is only a concern when actually reading, writing, or converting the pointer
    /// to a reference.
    pub(super) fn get_slice_ptr(&self, range: crate::MemoryRange) -> *mut [u8] {
        let base_ptr = self.storage.get();
        let range = range_to_usize(range, self.size);

        // We must obtain a slice pointer without ever creating a slice reference
        // that could alias with another slice.
        ptr::slice_from_raw_parts_mut(
            // SAFETY: `range_to_usize` bounds checks this addition.
            unsafe { base_ptr.cast::<u8>().add(range.start) },
            range.len(),
        )
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn upload_to_gpu(&self) -> Result<(), crate::DeviceError> {
        if self.gpu.mem_block.is_null() {
            return Err(crate::DeviceError::Lost);
        }
        let dst = unsafe { dk::dkMemBlockGetCpuAddr(self.gpu.mem_block) };
        if dst.is_null() {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            ptr::copy_nonoverlapping(self.storage.get().cast::<u8>(), dst.cast::<u8>(), self.size);
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn gpu_binding(
        &self,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
    ) -> Result<(dk::DkGpuAddr, u32), crate::DeviceError> {
        if self.gpu.mem_block.is_null() {
            return Err(crate::DeviceError::Lost);
        }
        let offset_usize = usize::try_from(offset).map_err(|_| crate::DeviceError::Lost)?;
        if offset_usize > self.size {
            return Err(crate::DeviceError::Lost);
        }
        let size = match size {
            Some(size) => usize::try_from(size.get()).map_err(|_| crate::DeviceError::Lost)?,
            None => self.size - offset_usize,
        };
        if offset_usize + size > self.size {
            return Err(crate::DeviceError::Lost);
        }
        let size = u32::try_from(size).map_err(|_| crate::DeviceError::Lost)?;
        Ok((self.gpu.gpu_addr + offset, size))
    }
}

#[cfg(target_os = "horizon")]
impl Drop for GpuBuffer {
    fn drop(&mut self) {
        if !self.mem_block.is_null() {
            unsafe { dk::dkMemBlockDestroy(self.mem_block) };
        }
    }
}

/// Convert a [`crate::MemoryRange`] to `Range<usize>` and bounds check it.
fn range_to_usize(range: crate::MemoryRange, upper_bound: usize) -> Range<usize> {
    // Note: these assertions should be impossible to trigger from safe code.
    // We're doing them anyway since this entire backend is for testing
    // (except for when it is an unused placeholder)
    let start = usize::try_from(range.start).expect("range too large");
    let end = usize::try_from(range.end).expect("range too large");
    assert!(start <= end && end <= upper_bound, "range out of bounds");
    start..end
}

#[cfg(target_os = "horizon")]
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}
