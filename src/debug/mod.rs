pub mod log;



use std::{alloc::{GlobalAlloc, Layout}, sync::atomic::{Ordering, AtomicU64}};

/// Overrides the global allocator with a memory tracking wrapper. Should be disabled in optimized builds
#[cfg(debug_assertions)]
#[global_allocator]
static GLOBAL_ALLOCATOR: TrackingAllocator<std::alloc::System> = TrackingAllocator::new(std::alloc::System);

pub struct TrackingAllocator<A: GlobalAlloc>(pub A, AtomicU64);

unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        self.1.fetch_add(l.size() as u64, Ordering::SeqCst);
        self.0.alloc(l)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, l: Layout) {
        self.0.dealloc(ptr, l);
        self.1.fetch_sub(l.size() as u64, Ordering::SeqCst);
    }
}

impl<A: GlobalAlloc> TrackingAllocator<A> {
    pub const fn new(a: A) -> Self {
        TrackingAllocator(a, AtomicU64::new(0))
    }

    pub fn reset_tracking(&self) {
        self.1.store(0, Ordering::SeqCst);
    }
    pub fn get_stats(&self) -> u64 {
        self.1.load(Ordering::SeqCst)
    }
}

/// Prints the current memory use to stdout, compiles to NOP in release builds
pub fn print_global_alloc_mem_use() {
    #[cfg(debug_assertions)]
    {
        let mem_used = GLOBAL_ALLOCATOR.get_stats() as f64;
        println!("mem: {:.2} MB", mem_used / 1024f64 / 1024f64);
    }
}

#[inline(always)]
pub fn dump_backtrace() {
    #[cfg(debug_assertions)]
    {
        let backtrace = std::backtrace::Backtrace::force_capture();
        dbg!(&backtrace);
        std::fs::write("backtrace.txt", format!("{}", backtrace)).expect("Failed to write backtrace.txt");
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(debug_assertions)]
    #[test]
    #[ignore]
    fn test_backtrace_dump() {
        dump_backtrace()
    }

    #[cfg(debug_assertions)]
    #[test]
    #[ignore]
    fn test_print_global_alloc_mem_use() {
        print_global_alloc_mem_use()
    }
}
