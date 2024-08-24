use alloc::boxed::Box;
use core::{
    error::Error,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFull, RangeTo},
    slice,
};

const PAGE_BITS: u32 = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS;

pub struct Dma<T> {
    pub virt: *mut T,
    pub phys: usize,
    pub size: usize,
}

// should be safe
impl<T> Deref for Dma<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.virt }
    }
}

impl<T> DerefMut for Dma<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.virt }
    }
}

// Trait for types that can be viewed as DMA slices
pub trait DmaSlice {
    type Item;

    fn chunks(&self, bytes: usize) -> DmaChunks<u8>;
    fn slice(&self, range: Range<usize>) -> Self::Item;
}

// mildly overengineered lol
pub struct DmaChunks<'a, T> {
    current_offset: usize,
    chunk_size: usize,
    dma: &'a Dma<T>,
}

impl<'a, T> Iterator for DmaChunks<'a, T> {
    type Item = DmaChunk<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_offset >= self.dma.size {
            None
        } else {
            let chunk_phys_addr = self.dma.phys + self.current_offset * core::mem::size_of::<T>();
            let offset_ptr = unsafe { self.dma.virt.add(self.current_offset) };
            let len = core::cmp::min(
                self.chunk_size,
                (self.dma.size - self.current_offset) / core::mem::size_of::<T>(),
            );

            self.current_offset += len;

            Some(DmaChunk {
                phys_addr: chunk_phys_addr,
                slice: unsafe { core::slice::from_raw_parts_mut(offset_ptr, len) },
            })
        }
    }
}

// Represents a chunk obtained from a Dma<T>, with physical address and slice.
pub struct DmaChunk<'a, T> {
    pub phys_addr: usize,
    pub slice: &'a mut [T],
}

impl DmaSlice for Dma<u8> {
    type Item = Dma<u8>;
    fn chunks(&self, bytes: usize) -> DmaChunks<u8> {
        DmaChunks {
            current_offset: 0,
            chunk_size: bytes,
            dma: self,
        }
    }

    fn slice(&self, index: Range<usize>) -> Self::Item {
        assert!(index.end <= self.size, "Index out of bounds");

        unsafe {
            Dma {
                virt: self.virt.add(index.start),
                phys: self.phys + index.start,
                size: (index.end - index.start),
            }
        }
    }
}

impl Index<Range<usize>> for Dma<u8> {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        assert!(index.end <= self.size, "Index out of bounds");

        unsafe { slice::from_raw_parts(self.virt.add(index.start), index.end - index.start) }
    }
}

impl IndexMut<Range<usize>> for Dma<u8> {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        assert!(index.end <= self.size, "Index out of bounds");
        unsafe { slice::from_raw_parts_mut(self.virt.add(index.start), index.end - index.start) }
    }
}

impl Index<RangeTo<usize>> for Dma<u8> {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self[0..index.end]
    }
}

impl IndexMut<RangeTo<usize>> for Dma<u8> {
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        &mut self[0..index.end]
    }
}

impl Index<RangeFull> for Dma<u8> {
    type Output = [u8];

    fn index(&self, _: RangeFull) -> &Self::Output {
        &self[0..self.size]
    }
}

impl IndexMut<RangeFull> for Dma<u8> {
    fn index_mut(&mut self, _: RangeFull) -> &mut Self::Output {
        let len = self.size;
        &mut self[0..len]
    }
}

extern "Rust" {
    fn alloc_for_dma(size: usize) -> (usize, usize);
}

impl<T> Dma<T> {
    pub fn allocate(size: usize) -> Result<Dma<T>, Box<dyn Error>> {
        let size = if size % 4096 != 0 {
            ((size >> PAGE_BITS) + 1) << PAGE_BITS
        } else {
            size
        };

        let (paddr, vaddr) = unsafe { alloc_for_dma(size / PAGE_SIZE) };

        Ok(Dma {
            virt: vaddr as *mut T,
            phys: paddr,
            size,
        })
    }
}
