//! Reading and writing game memory through CheatEngine-style pointer chains.

use std::ffi::c_void;
use std::fmt::Debug;
use std::mem::{self, MaybeUninit};
use std::ops::{BitAnd, BitOr, BitXor, Not};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::Threading::GetCurrentProcess;

pub mod widgets;

extern "C" {
    /// Defined in `guarded_read.c`. Copies `len` bytes from `src` to `dst`,
    /// returning 0 instead of crashing if `src` is not readable.
    fn practice_tool_guarded_read(src: *const c_void, dst: *mut c_void, len: usize) -> i32;
}

/// Lowest address that can ever be mapped on Windows: the first 64 KiB are
/// always reserved. Catches null pointers plus an offset without faulting.
const MIN_USER_ADDRESS: usize = 0x1_0000;
/// Highest user-mode address on x64 Windows. Anything above is kernel space
/// or non-canonical, and can never be read from user mode.
const MAX_USER_ADDRESS: usize = 0x7FFF_FFFE_FFFF;

/// Reads a `T` from `addr` without crashing if the memory isn't readable.
///
/// Addresses that can never be mapped are rejected upfront, as handling an
/// access violation is much more expensive than a successful read.
///
/// `T` must be valid for any bit pattern.
fn guarded_read<T>(addr: usize) -> Option<T> {
    let len = mem::size_of::<T>();
    let last = addr.checked_add(len.max(1) - 1)?;
    if addr < MIN_USER_ADDRESS || last > MAX_USER_ADDRESS {
        return None;
    }

    let mut value = MaybeUninit::<T>::uninit();
    // SAFETY: `value` has room for `len` bytes, and the C side catches access
    // violations on `addr` instead of letting them unwind.
    let ok = unsafe { practice_tool_guarded_read(addr as _, value.as_mut_ptr() as _, len) };

    // SAFETY: all `len` bytes were written, and `T` is valid for any bit
    // pattern.
    (ok != 0).then(|| unsafe { value.assume_init() })
}

/// Wraps CheatEngine's concept of pointer with nested offsets. Evaluates,
/// if the evaluation does not fail, to a mutable pointer of type `T`.
///
/// At runtime, it evaluates the final address of the chain by reading the
/// base pointer, then recursively reading the next memory address in the
/// chain at an offset from there. For example,
///
/// ```text
/// PointerChain::<T>::new(&[a, b, c, d, e])
/// ```
///
/// evaluates to
///
/// ```text
/// *(*(*(*(*a + b) + c) + d) + e)
/// ```
///
/// This is useful for managing reverse engineered structures which are not
/// fully known.
#[derive(Clone, Debug)]
pub struct PointerChain<T> {
    proc: HANDLE,
    base: *mut T,
    // Stored inline, so that creating and evaluating a chain never touches
    // the heap.
    offsets: [usize; MAX_OFFSETS],
    len: usize,
}
unsafe impl<T> Send for PointerChain<T> {}
unsafe impl<T> Sync for PointerChain<T> {}

/// Maximum number of offsets in a `PointerChain`, after the base address.
/// `PointerChain::new` panics on longer chains; raise this if a tool needs
/// them. Every chain stores this many offsets inline.
pub const MAX_OFFSETS: usize = 8;

impl<T> PointerChain<T> {
    /// Creates a new pointer chain given an array of addresses.
    ///
    /// Panics if `chain` is empty or has more than `MAX_OFFSETS` offsets after
    /// the base address.
    pub fn new(chain: &[usize]) -> PointerChain<T> {
        let (&base, offsets) = chain.split_first().expect("empty pointer chain");
        assert!(
            offsets.len() <= MAX_OFFSETS,
            "pointer chain has {} offsets, more than the maximum of {MAX_OFFSETS}",
            offsets.len()
        );

        let mut inline = [0; MAX_OFFSETS];
        inline[..offsets.len()].copy_from_slice(offsets);

        PointerChain {
            proc: unsafe { GetCurrentProcess() },
            base: base as *mut T,
            offsets: inline,
            len: offsets.len(),
        }
    }

    /// Safely evaluates the pointer chain.
    /// Relies on guarded reads instead of plain pointer dereferencing for
    /// crash safety. Returns `None` if the evaluation failed.
    pub fn eval(&self) -> Option<*mut T> {
        self.offsets[..self.len]
            .iter()
            .try_fold(self.base as usize, |addr, &offs| {
                guarded_read::<usize>(addr).map(|value| value.wrapping_add(offs))
            })
            .map(|addr| addr as *mut T)
    }

    /// Evaluates the pointer chain and attempts to read the datum.
    /// Returns `None` if either the evaluation or the read failed.
    pub fn read(&self) -> Option<T> {
        guarded_read(self.eval()? as usize)
    }

    /// Evaluates the pointer chain and attempts to write the datum.
    /// Returns `None` if either the evaluation or the write failed.
    ///
    /// Uses `WriteProcessMemory`, which also succeeds on read-only pages (e.g.
    /// code patches) by temporarily changing their protection.
    pub fn write(&self, mut value: T) -> Option<()> {
        let ptr = self.eval()?;
        unsafe {
            WriteProcessMemory(
                self.proc,
                ptr as _,
                &mut value as *mut _ as _,
                std::mem::size_of::<T>(),
                None,
            )
            .ok()
            .map(|_| ())
        }
    }

    pub fn cast<S>(&self) -> PointerChain<S> {
        PointerChain {
            proc: self.proc,
            base: self.base as *mut S,
            offsets: self.offsets,
            len: self.len,
        }
    }
}

/// A flag backed by game memory.
pub trait FlagToggler: Send + Sync + Debug {
    fn get(&self) -> Option<bool>;
    fn set(&self, flag: bool);
    /// Flips the flag, returning its new state.
    fn toggle(&self) -> Option<bool>;
}

impl<T: FlagToggler + ?Sized> FlagToggler for &T {
    fn get(&self) -> Option<bool> {
        (**self).get()
    }

    fn set(&self, flag: bool) {
        (**self).set(flag)
    }

    fn toggle(&self) -> Option<bool> {
        (**self).toggle()
    }
}

impl<T: FlagToggler + ?Sized> FlagToggler for Box<T> {
    fn get(&self) -> Option<bool> {
        (**self).get()
    }

    fn set(&self, flag: bool) {
        (**self).set(flag)
    }

    fn toggle(&self) -> Option<bool> {
        (**self).toggle()
    }
}

#[derive(Clone, Debug)]
pub struct Bitflag<T>(PointerChain<T>, T);

impl<T> Bitflag<T> {
    pub fn new(c: PointerChain<T>, mask: T) -> Self {
        Bitflag(c, mask)
    }
}

impl<T> FlagToggler for Bitflag<T>
where
    T: Send
        + Sync
        + Copy
        + Debug
        + BitXor<Output = T>
        + BitAnd<Output = T>
        + BitOr<Output = T>
        + Not<Output = T>
        + PartialEq,
{
    fn get(&self) -> Option<bool> {
        self.0.read().map(|x| (x & self.1) == self.1)
    }

    fn set(&self, flag: bool) {
        if let Some(x) = self.0.read() {
            let value = if flag { x | self.1 } else { x & !self.1 };
            // Writes are far more expensive than reads: skip them when the
            // flag is already in the requested state.
            if value != x {
                self.0.write(value);
            }
        }
    }

    fn toggle(&self) -> Option<bool> {
        let value = self.0.read()? ^ self.1;
        self.0.write(value);
        Some((value & self.1) == self.1)
    }
}

/// Swaps between two byte patterns: `bytes_on`, and the bytes found in
/// memory when the patch is created.
#[derive(Clone, Debug)]
pub struct BytesPatch<const N: usize>(PointerChain<[u8; N]>, [u8; N], [u8; N]);

impl<const N: usize> BytesPatch<N> {
    pub fn new(c: PointerChain<[u8; N]>, bytes_on: [u8; N]) -> Self {
        if let Some(x) = c.read() {
            BytesPatch(c, bytes_on, x)
        } else {
            BytesPatch(c, bytes_on, [0; N])
        }
    }
}

impl<const N: usize> FlagToggler for BytesPatch<N> {
    fn get(&self) -> Option<bool> {
        self.0.read().map(|x| x == self.1)
    }

    fn set(&self, flag: bool) {
        if let Some(x) = self.0.read() {
            if (x == self.1) != flag {
                self.0.write(if flag { self.1 } else { self.2 });
            }
        }
    }

    fn toggle(&self) -> Option<bool> {
        if let Some(x) = self.0.read() {
            let was_on = x == self.1;
            self.0.write(if was_on { self.2 } else { self.1 });
            Some(!was_on)
        } else {
            None
        }
    }
}

#[macro_export]
macro_rules! pointer_chain {
    ($($e:expr),+) => { $crate::PointerChain::new(&[$($e,)*]) }
}

#[macro_export]
macro_rules! bitflag {
    ($b:expr; $($e:expr),+) => { $crate::Bitflag::new($crate::PointerChain::new(&[$($e,)*]), $b) }
}

#[macro_export]
macro_rules! bytes_patch {
    ($b:expr; $($e:expr),+) => {
        $crate::BytesPatch::new($crate::PointerChain::new(&[$($e,)*]), $b)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::cell::UnsafeCell;
    use std::ptr;

    use windows::Win32::System::Memory::{
        GetWriteWatch, VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
        PAGE_NOACCESS, PAGE_READWRITE, VIRTUAL_ALLOCATION_TYPE,
    };
    use windows::Win32::System::SystemServices::{MEM_WRITE_WATCH, WRITE_WATCH_FLAG_RESET};

    use super::*;

    /// Number of pages written in `[base, base + len)` since the last call.
    unsafe fn written_pages(base: *mut c_void, len: usize) -> usize {
        let mut addresses = [ptr::null_mut(); 16];
        let mut count = addresses.len();
        let mut granularity = 0;
        let r = GetWriteWatch(
            WRITE_WATCH_FLAG_RESET,
            base,
            len,
            Some(addresses.as_mut_ptr()),
            Some(&mut count),
            Some(&mut granularity),
        );
        assert_eq!(r, 0);
        count
    }

    #[test]
    fn test_guarded_read_valid() {
        let value: u64 = 0x0123_4567_89AB_CDEF;
        assert_eq!(guarded_read::<u64>(&value as *const u64 as usize), Some(value));
    }

    #[test]
    fn test_guarded_read_unmappable() {
        assert_eq!(guarded_read::<u64>(0), None);
        assert_eq!(guarded_read::<u64>(0x80), None);
        assert_eq!(guarded_read::<u64>(MIN_USER_ADDRESS - 1), None);
        assert_eq!(guarded_read::<u64>(MAX_USER_ADDRESS - 4), None);
        assert_eq!(guarded_read::<u64>(0xFFFF_F800_0000_0000), None);
        assert_eq!(guarded_read::<u64>(usize::MAX - 2), None);
    }

    #[test]
    fn test_guarded_read_access_violation() {
        unsafe {
            // Not rejected upfront: these reads fault and must be caught.
            let reserved = VirtualAlloc(None, 0x1000, MEM_RESERVE, PAGE_NOACCESS);
            assert!(!reserved.is_null());
            assert_eq!(guarded_read::<u64>(reserved as usize), None);
            VirtualFree(reserved, 0, MEM_RELEASE).unwrap();

            let no_access = VirtualAlloc(None, 0x1000, MEM_RESERVE | MEM_COMMIT, PAGE_NOACCESS);
            assert!(!no_access.is_null());
            assert_eq!(guarded_read::<[u8; 16]>(no_access as usize), None);
            VirtualFree(no_access, 0, MEM_RELEASE).unwrap();
        }
    }

    #[test]
    fn test_pointer_chain() {
        let value: u32 = 0xDEAD_BEEF;
        let inner: [usize; 2] = [0, &value as *const u32 as usize - 8];
        let outer: usize = inner.as_ptr() as usize;

        // *(*(&outer) + 8) + 8
        let chain = PointerChain::<u32>::new(&[&outer as *const usize as usize, 8, 8]);
        assert_eq!(chain.read(), Some(value));

        // Second deref reads inner[0], a null pointer, and the third reads from
        // it.
        let broken = PointerChain::<u32>::new(&[&outer as *const usize as usize, 0, 0, 8]);
        assert_eq!(broken.eval(), None);
        assert_eq!(broken.read(), None);
    }

    #[test]
    fn test_pointer_chain_unmapped() {
        assert_eq!(PointerChain::<u32>::new(&[0]).read(), None);
        assert_eq!(PointerChain::<u32>::new(&[0, 8]).eval(), None);
        assert_eq!(PointerChain::<u32>::new(&[0, 8]).write(1), None);

        unsafe {
            let reserved = VirtualAlloc(None, 0x1000, MEM_RESERVE, PAGE_NOACCESS);
            assert!(!reserved.is_null());
            assert_eq!(PointerChain::<u32>::new(&[reserved as usize, 0]).eval(), None);
            VirtualFree(reserved, 0, MEM_RELEASE).unwrap();
        }
    }

    #[test]
    fn test_pointer_chain_write() {
        // Written behind the compiler's back, so it must be interior-mutable.
        let value = UnsafeCell::new(0u32);
        let base: usize = value.get() as usize - 8;

        // *(&base) + 8
        let chain = PointerChain::<u32>::new(&[&base as *const usize as usize, 8]);
        assert_eq!(chain.write(0xDEAD_BEEF), Some(()));
        assert_eq!(unsafe { ptr::read_volatile(value.get()) }, 0xDEAD_BEEF);
        assert_eq!(chain.read(), Some(0xDEAD_BEEF));
    }

    #[test]
    fn test_pointer_chain_max_offsets() {
        // Each deref lands on the next element; the last one points to `value`.
        let value: u32 = 0xDEAD_BEEF;
        let mut links = [0usize; MAX_OFFSETS];
        links[MAX_OFFSETS - 1] = &value as *const u32 as usize;
        for i in (0..MAX_OFFSETS - 1).rev() {
            links[i] = &links[i + 1] as *const usize as usize;
        }

        let mut chain = vec![&links[0] as *const usize as usize];
        chain.extend([0; MAX_OFFSETS]);
        assert_eq!(PointerChain::<u32>::new(&chain).read(), Some(value));
        assert_eq!(PointerChain::<u32>::new(&chain).cast::<u16>().read(), Some(0xBEEF));
    }

    #[test]
    #[should_panic(expected = "more than the maximum")]
    fn test_pointer_chain_too_many_offsets() {
        PointerChain::<u32>::new(&[0x1000; MAX_OFFSETS + 2]);
    }

    #[test]
    fn test_bitflag_set() {
        // Written behind the compiler's back, so it must be interior-mutable.
        let value = UnsafeCell::new(0b1010_0000u8);
        let flag = Bitflag::new(PointerChain::<u8>::new(&[value.get() as usize]), 0b100);
        let read = || unsafe { ptr::read_volatile(value.get()) };

        flag.set(false);
        assert_eq!(read(), 0b1010_0000);
        flag.set(true);
        assert_eq!(read(), 0b1010_0100);
        assert_eq!(flag.get(), Some(true));
        flag.set(true);
        assert_eq!(read(), 0b1010_0100);
        flag.set(false);
        assert_eq!(read(), 0b1010_0000);
        assert_eq!(flag.get(), Some(false));
        assert_eq!(flag.toggle(), Some(true));
        assert_eq!(read(), 0b1010_0100);
        assert_eq!(flag.toggle(), Some(false));
        assert_eq!(read(), 0b1010_0000);
    }

    #[test]
    fn test_bitflag_set_unchanged() {
        unsafe {
            // Write-watched memory records the pages written to, so it shows
            // whether `set` wrote at all.
            let page = VirtualAlloc(
                None,
                0x1000,
                MEM_RESERVE | MEM_COMMIT | VIRTUAL_ALLOCATION_TYPE(MEM_WRITE_WATCH),
                PAGE_READWRITE,
            );
            assert!(!page.is_null());
            ptr::write_volatile(page as *mut u8, 0b100);
            written_pages(page, 0x1000);

            let flag = Bitflag::new(PointerChain::<u8>::new(&[page as usize]), 0b100u8);
            flag.set(true);
            assert_eq!(written_pages(page, 0x1000), 0);
            flag.set(false);
            assert_eq!(written_pages(page, 0x1000), 1);

            VirtualFree(page, 0, MEM_RELEASE).unwrap();
        }
    }

    #[test]
    fn test_bytes_patch() {
        let bytes = UnsafeCell::new([1u8, 2, 3, 4]);
        let patch = BytesPatch::new(PointerChain::new(&[bytes.get() as usize]), [5, 6, 7, 8]);
        let read = || unsafe { ptr::read_volatile(bytes.get()) };

        assert_eq!(patch.get(), Some(false));
        assert_eq!(patch.toggle(), Some(true));
        assert_eq!(read(), [5, 6, 7, 8]);
        assert_eq!(patch.get(), Some(true));
        assert_eq!(patch.toggle(), Some(false));
        assert_eq!(read(), [1, 2, 3, 4]);
        patch.set(true);
        assert_eq!(read(), [5, 6, 7, 8]);
        patch.set(false);
        assert_eq!(read(), [1, 2, 3, 4]);
    }
}
