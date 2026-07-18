use alloc::vec::Vec;
use core::{cell::UnsafeCell, ops::Range, ptr};
#[cfg(target_os = "horizon")]
use std::sync::Mutex;

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
    #[cfg(target_os = "horizon")]
    id: u64,
    #[cfg(target_os = "horizon")]
    label: Option<alloc::string::String>,
    /// This data is potentially accessed mutably in arbitrary non-overlapping slices,
    /// so we must store it in `UnsafeCell` to avoid making any too-strong no-aliasing claims.
    storage: Arc<UnsafeCell<[u8]>>,

    /// Size of the allocation.
    ///
    /// This is redundant with `storage.get().len()`, but that method is not
    /// available until our MSRV is 1.79 or greater.
    size: usize,

    #[cfg(target_os = "horizon")]
    gpu: Option<Arc<GpuBuffer>>,
}

#[cfg(target_os = "horizon")]
#[derive(Debug)]
struct GpuBuffer {
    pool: Arc<GpuBufferPool>,
    block: usize,
    mem_block: dk::DkMemBlock,
    offset: u32,
    size: u32,
    gpu_addr: dk::DkGpuAddr,
}

#[cfg(target_os = "horizon")]
#[derive(Debug)]
pub(super) struct GpuBufferPool {
    raw_device: dk::DkDevice,
    blocks: Mutex<Vec<GpuBufferBlock>>,
}

#[cfg(target_os = "horizon")]
#[derive(Debug)]
struct GpuBufferBlock {
    mem_block: dk::DkMemBlock,
    size: u32,
    free: Vec<Range<u32>>,
}

#[cfg(target_os = "horizon")]
const GPU_BUFFER_POOL_SIZE: u32 = 32 * 1024 * 1024;

#[cfg(target_os = "horizon")]
impl GpuBufferPool {
    pub(super) fn new(raw_device: dk::DkDevice) -> Result<Arc<Self>, crate::DeviceError> {
        let mut maker = dk::DkMemBlockMaker::defaults(raw_device, GPU_BUFFER_POOL_SIZE);
        maker.flags = dk::DkMemBlockFlags_CpuUncached
            | dk::DkMemBlockFlags_GpuCached
            | dk::DkMemBlockFlags_ZeroFillInit;
        let mem_block = unsafe { dk::dkMemBlockCreate(&maker) };
        if mem_block.is_null() {
            return Err(crate::DeviceError::OutOfMemory);
        }
        Ok(Arc::new(Self {
            raw_device,
            blocks: Mutex::new(vec![GpuBufferBlock {
                mem_block,
                size: GPU_BUFFER_POOL_SIZE,
                free: vec![0..GPU_BUFFER_POOL_SIZE],
            }]),
        }))
    }

    fn allocate(&self, size: u32) -> Result<(usize, dk::DkMemBlock, u32), crate::DeviceError> {
        let size = align_up(size.max(1), 256);
        let mut blocks = self
            .blocks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for (block_index, block) in blocks.iter_mut().enumerate() {
            for range_index in 0..block.free.len() {
                let range = block.free[range_index].clone();
                if range.end - range.start >= size {
                    let offset = range.start;
                    block.free[range_index].start += size;
                    if block.free[range_index].is_empty() {
                        block.free.remove(range_index);
                    }
                    return Ok((block_index, block.mem_block, offset));
                }
            }
        }
        let available = blocks
            .iter()
            .flat_map(|block| &block.free)
            .map(Range::len)
            .sum::<usize>();
        super::trace::record(format_args!(
            "event kind=buffer_pool_grow size={size} available={available} blocks={}",
            blocks.len()
        ));
        let block_size = size
            .max(GPU_BUFFER_POOL_SIZE)
            .checked_next_power_of_two()
            .ok_or(crate::DeviceError::OutOfMemory)?;
        let mut maker = dk::DkMemBlockMaker::defaults(self.raw_device, block_size);
        maker.flags = dk::DkMemBlockFlags_CpuUncached
            | dk::DkMemBlockFlags_GpuCached
            | dk::DkMemBlockFlags_ZeroFillInit;
        let mem_block = unsafe { dk::dkMemBlockCreate(&maker) };
        if mem_block.is_null() {
            super::trace::record(format_args!(
                "failure kind=buffer_pool_grow_failed size={block_size}"
            ));
            super::trace::dump("buffer_pool_grow_failed");
            return Err(crate::DeviceError::OutOfMemory);
        }
        let block_index = blocks.len();
        blocks.push(GpuBufferBlock {
            mem_block,
            size: block_size,
            free: if size == block_size {
                Vec::new()
            } else {
                vec![size..block_size]
            },
        });
        Ok((block_index, mem_block, 0))
    }

    fn release(&self, block_index: usize, offset: u32, size: u32) {
        let mut blocks = self
            .blocks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let block = &mut blocks[block_index];
        debug_assert!(offset + size <= block.size);
        let free = &mut block.free;
        let mut index = free.partition_point(|range| range.start < offset);
        free.insert(index, offset..offset + size);
        if index > 0 && free[index - 1].end == free[index].start {
            let end = free[index].end;
            free[index - 1].end = end;
            free.remove(index);
            index -= 1;
        }
        if index + 1 < free.len() && free[index].end == free[index + 1].start {
            let end = free[index + 1].end;
            free[index].end = end;
            free.remove(index + 1);
        }
    }
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
        pool: Arc<GpuBufferPool>,
    ) -> Result<Self, crate::DeviceError> {
        let mut buffer = Self::new_host(desc)?;
        let allocation_size =
            u32::try_from(buffer.size.max(1)).map_err(|_| crate::DeviceError::OutOfMemory)?;
        let (block, mem_block, offset) = pool.allocate(allocation_size)?;
        let base_gpu_addr = unsafe { dk::dkMemBlockGetGpuAddr(mem_block) };
        let gpu_addr = base_gpu_addr + u64::from(offset);
        if gpu_addr == u64::MAX {
            return Err(crate::DeviceError::Lost);
        }
        buffer.gpu = Some(Arc::new(GpuBuffer {
            pool,
            block,
            mem_block,
            offset,
            size: align_up(allocation_size.max(1), 256),
            gpu_addr,
        }));
        super::trace::record_resource(format_args!(
            "resource kind=buffer id={} label={:?} size={} usage={:?}",
            buffer.id, buffer.label, desc.size, desc.usage
        ));
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
            #[cfg(target_os = "horizon")]
            id: super::trace::resource_id(),
            #[cfg(target_os = "horizon")]
            label: desc.label.map(alloc::string::String::from),
            storage,
            size,
            #[cfg(target_os = "horizon")]
            gpu: None,
        })
    }

    /// Returns a pointer to the memory owned by this buffer within the given `range`.
    ///
    /// This may be used to create any number of simultaneous pointers;
    /// aliasing is only a concern when actually reading, writing, or converting the pointer
    /// to a reference.
    pub(super) fn get_slice_ptr(
        &self,
        range: crate::MemoryRange,
    ) -> super::DeviceResult<*mut [u8]> {
        let base_ptr = self.storage.get();
        let range = range_to_usize(range, self.size)?;

        // We must obtain a slice pointer without ever creating a slice reference
        // that could alias with another slice.
        Ok(ptr::slice_from_raw_parts_mut(
            // SAFETY: `range_to_usize` bounds checks this addition.
            unsafe { base_ptr.cast::<u8>().add(range.start) },
            range.len(),
        ))
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn read_u32(&self, offset: wgt::BufferAddress) -> Result<u32, crate::DeviceError> {
        let offset = usize::try_from(offset).map_err(|_| crate::DeviceError::Lost)?;
        let end = offset.checked_add(4).ok_or(crate::DeviceError::Lost)?;
        if end > self.size {
            return Err(crate::DeviceError::Lost);
        }

        let gpu = self.gpu.as_ref().ok_or(crate::DeviceError::Lost)?;
        let base = unsafe { dk::dkMemBlockGetCpuAddr(gpu.mem_block) }.cast::<u8>();
        if base.is_null() {
            return Err(crate::DeviceError::Lost);
        }
        let mut bytes = [0; 4];
        unsafe {
            let src = base.add(gpu.offset as usize).add(offset);
            ptr::copy_nonoverlapping(src, bytes.as_mut_ptr(), bytes.len());
        }
        Ok(u32::from_ne_bytes(bytes))
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn upload_range_to_gpu(
        &self,
        range: Range<wgt::BufferAddress>,
    ) -> Result<(), crate::DeviceError> {
        let start = usize::try_from(range.start).map_err(|_| crate::DeviceError::Lost)?;
        let end = usize::try_from(range.end).map_err(|_| crate::DeviceError::Lost)?;
        if start > end || end > self.size {
            return Err(crate::DeviceError::Lost);
        }
        let gpu = self.gpu.as_ref().ok_or(crate::DeviceError::Lost)?;
        let dst = unsafe { dk::dkMemBlockGetCpuAddr(gpu.mem_block) };
        if dst.is_null() {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            ptr::copy_nonoverlapping(
                self.storage.get().cast::<u8>().add(start),
                dst.cast::<u8>().add(gpu.offset as usize + start),
                end - start,
            );
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn download_from_gpu(
        &self,
        range: Range<wgt::BufferAddress>,
    ) -> Result<(), crate::DeviceError> {
        let start = usize::try_from(range.start).map_err(|_| crate::DeviceError::Lost)?;
        let end = usize::try_from(range.end).map_err(|_| crate::DeviceError::Lost)?;
        if start > end || end > self.size {
            return Err(crate::DeviceError::Lost);
        }
        let gpu = self.gpu.as_ref().ok_or(crate::DeviceError::Lost)?;
        let src = unsafe { dk::dkMemBlockGetCpuAddr(gpu.mem_block) };
        if src.is_null() {
            return Err(crate::DeviceError::Lost);
        }
        unsafe {
            ptr::copy_nonoverlapping(
                src.cast::<u8>().add(gpu.offset as usize + start),
                self.storage.get().cast::<u8>().add(start),
                end - start,
            );
        }
        Ok(())
    }

    #[cfg(target_os = "horizon")]
    pub(super) fn gpu_binding(
        &self,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
    ) -> Result<(dk::DkGpuAddr, u32), crate::DeviceError> {
        let offset_usize = usize::try_from(offset).map_err(|_| crate::DeviceError::Lost)?;
        let gpu = self.gpu.as_ref().ok_or(crate::DeviceError::Lost)?;
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
        Ok((gpu.gpu_addr + offset, size))
    }

    #[cfg(target_os = "horizon")]
    pub(super) unsafe fn gpu_slice(
        &self,
        range: Range<wgt::BufferAddress>,
    ) -> Result<&[u8], crate::DeviceError> {
        let start = usize::try_from(range.start).map_err(|_| crate::DeviceError::Lost)?;
        let end = usize::try_from(range.end).map_err(|_| crate::DeviceError::Lost)?;
        if start > end || end > self.size {
            return Err(crate::DeviceError::Lost);
        }
        let gpu = self.gpu.as_ref().ok_or(crate::DeviceError::Lost)?;
        let base = unsafe { dk::dkMemBlockGetCpuAddr(gpu.mem_block) };
        if base.is_null() {
            return Err(crate::DeviceError::Lost);
        }
        Ok(unsafe {
            core::slice::from_raw_parts(
                base.cast::<u8>().add(gpu.offset as usize + start),
                end - start,
            )
        })
    }
}

#[cfg(target_os = "horizon")]
impl Drop for GpuBuffer {
    fn drop(&mut self) {
        self.pool.release(self.block, self.offset, self.size);
    }
}

#[cfg(target_os = "horizon")]
impl Drop for GpuBufferPool {
    fn drop(&mut self) {
        let blocks = self
            .blocks
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner());
        for block in blocks {
            if !block.mem_block.is_null() {
                unsafe { dk::dkMemBlockDestroy(block.mem_block) };
            }
        }
    }
}

/// Convert a [`crate::MemoryRange`] to `Range<usize>` and bounds check it.
fn range_to_usize(
    range: crate::MemoryRange,
    upper_bound: usize,
) -> super::DeviceResult<Range<usize>> {
    let start = usize::try_from(range.start).map_err(|_| crate::DeviceError::Lost)?;
    let end = usize::try_from(range.end).map_err(|_| crate::DeviceError::Lost)?;
    if start > end || end > upper_bound {
        return Err(crate::DeviceError::Lost);
    }
    Ok(start..end)
}

#[cfg(target_os = "horizon")]
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_mapping_ranges_return_errors() {
        assert!(range_to_usize(0..4, 4).is_ok());
        let reversed_start = 3;
        let reversed_end = 2;
        assert!(range_to_usize(reversed_start..reversed_end, 4).is_err());
        assert!(range_to_usize(0..5, 4).is_err());
    }
}
