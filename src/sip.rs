//! An implementation of SipHash copied from stdlib with modifications to support deterministic
//! seeding in tests.

#![allow(deprecated)] // the types in this module are deprecated

use std::cell::RefCell;
use std::hash::BuildHasher;
use std::marker::PhantomData;
use std::rc::Rc;
use std::{cmp, ptr};

use rand::RngExt;
use rand::rngs::StdRng;

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct SipHasher13 {
    hasher: Hasher<Sip13Rounds>,
}

#[derive(Debug)]
struct Hasher<S: Sip> {
    k0: u64,
    k1: u64,
    length: usize, // how many bytes we've processed
    state: State,  // hash State
    tail: u64,     // unprocessed bytes le
    ntail: usize,  // how many bytes in tail are valid
    _marker: PhantomData<S>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct State {
    // v0, v2 and v1, v3 show up in pairs in the algorithm,
    // and simd implementations of SipHash will use vectors
    // of v02 and v13. By placing them in this order in the struct,
    // the compiler can pick up on just a few simd optimizations by itself.
    v0: u64,
    v2: u64,
    v1: u64,
    v3: u64,
}

macro_rules! compress {
    ($state:expr) => {{ compress!($state.v0, $state.v1, $state.v2, $state.v3) }};
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {{
        $v0 = $v0.wrapping_add($v1);
        $v2 = $v2.wrapping_add($v3);
        $v1 = $v1.rotate_left(13);
        $v1 ^= $v0;
        $v3 = $v3.rotate_left(16);
        $v3 ^= $v2;
        $v0 = $v0.rotate_left(32);

        $v2 = $v2.wrapping_add($v1);
        $v0 = $v0.wrapping_add($v3);
        $v1 = $v1.rotate_left(17);
        $v1 ^= $v2;
        $v3 = $v3.rotate_left(21);
        $v3 ^= $v0;
        $v2 = $v2.rotate_left(32);
    }};
}

/// Loads an integer of the desired type from a byte stream, in LE order. Uses
/// `copy_nonoverlapping` to let the compiler generate the most efficient way
/// to load it from a possibly unaligned address.
///
/// Safety: this performs unchecked indexing of `$buf` at
/// `$i..$i+size_of::<$int_ty>()`, so that must be in-bounds.
macro_rules! load_int_le {
    ($buf:expr, $i:expr, $int_ty:ident) => {{
        debug_assert!($i + size_of::<$int_ty>() <= $buf.len());
        let mut data = 0 as $int_ty;
        ptr::copy_nonoverlapping(
            $buf.as_ptr().add($i),
            &mut data as *mut _ as *mut u8,
            size_of::<$int_ty>(),
        );
        data.to_le()
    }};
}

/// Loads a u64 using up to 7 bytes of a byte slice. It looks clumsy but the
/// `copy_nonoverlapping` calls that occur (via `load_int_le!`) all have fixed
/// sizes and avoid calling `memcpy`, which is good for speed.
///
/// Safety: this performs unchecked indexing of `buf` at `start..start+len`, so
/// that must be in-bounds.
#[inline]
unsafe fn u8to64_le(buf: &[u8], start: usize, len: usize) -> u64 {
    debug_assert!(len < 8);
    let mut i = 0; // current byte index (from LSB) in the output u64
    let mut out = 0;
    if i + 3 < len {
        // SAFETY: `i` cannot be greater than `len`, and the caller must guarantee
        // that the index start..start+len is in bounds.
        out = unsafe { load_int_le!(buf, start + i, u32) } as u64;
        i += 4;
    }
    if i + 1 < len {
        // SAFETY: same as above.
        out |= (unsafe { load_int_le!(buf, start + i, u16) } as u64) << (i * 8);
        i += 2
    }
    if i < len {
        // SAFETY: same as above.
        out |= (unsafe { *buf.get_unchecked(start + i) } as u64) << (i * 8);
        i += 1;
    }
    debug_assert_eq!(i, len);
    out
}

impl SipHasher13 {
    /// Creates a `SipHasher13` that is keyed off the provided keys.
    #[inline]
    pub const fn new_with_keys(key0: u64, key1: u64) -> SipHasher13 {
        SipHasher13 {
            hasher: Hasher::new_with_keys(key0, key1),
        }
    }
}

impl<S: Sip> Hasher<S> {
    #[inline]
    const fn new_with_keys(key0: u64, key1: u64) -> Hasher<S> {
        let mut state = Hasher {
            k0: key0,
            k1: key1,
            length: 0,
            state: State {
                v0: 0,
                v1: 0,
                v2: 0,
                v3: 0,
            },
            tail: 0,
            ntail: 0,
            _marker: PhantomData,
        };
        state.reset();
        state
    }

    #[inline]
    const fn reset(&mut self) {
        self.length = 0;
        self.state.v0 = self.k0 ^ 0x736f6d6570736575;
        self.state.v1 = self.k1 ^ 0x646f72616e646f6d;
        self.state.v2 = self.k0 ^ 0x6c7967656e657261;
        self.state.v3 = self.k1 ^ 0x7465646279746573;
        self.ntail = 0;
    }
}

impl std::hash::Hasher for SipHasher13 {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.hasher.write(msg)
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hasher.finish()
    }
}

impl<S: Sip> std::hash::Hasher for Hasher<S> {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        let length = msg.len();
        self.length += length;

        let mut needed = 0;

        if self.ntail != 0 {
            needed = 8 - self.ntail;
            // SAFETY: `cmp::min(length, needed)` is guaranteed to not be over `length`
            self.tail |= unsafe { u8to64_le(msg, 0, cmp::min(length, needed)) } << (8 * self.ntail);
            if length < needed {
                self.ntail += length;
                return;
            } else {
                self.state.v3 ^= self.tail;
                S::c_rounds(&mut self.state);
                self.state.v0 ^= self.tail;
                self.ntail = 0;
            }
        }

        // Buffered tail is now flushed, process new input.
        let len = length - needed;
        let left = len & 0x7; // len % 8

        let mut i = needed;
        while i < len - left {
            // SAFETY: because `len - left` is the biggest multiple of 8 under
            // `len`, and because `i` starts at `needed` where `len` is `length - needed`,
            // `i + 8` is guaranteed to be less than or equal to `length`.
            let mi = unsafe { load_int_le!(msg, i, u64) };

            self.state.v3 ^= mi;
            S::c_rounds(&mut self.state);
            self.state.v0 ^= mi;

            i += 8;
        }

        // SAFETY: `i` is now `needed + len.div_euclid(8) * 8`,
        // so `i + left` = `needed + len` = `length`, which is by
        // definition equal to `msg.len()`.
        self.tail = unsafe { u8to64_le(msg, i, left) };
        self.ntail = left;
    }

    #[inline]
    fn finish(&self) -> u64 {
        let mut state = self.state;

        let b: u64 = ((self.length as u64 & 0xff) << 56) | self.tail;

        state.v3 ^= b;
        S::c_rounds(&mut state);
        state.v0 ^= b;

        state.v2 ^= 0xff;
        S::d_rounds(&mut state);

        state.v0 ^ state.v1 ^ state.v2 ^ state.v3
    }
}

impl<S: Sip> Clone for Hasher<S> {
    #[inline]
    fn clone(&self) -> Hasher<S> {
        Hasher {
            k0: self.k0,
            k1: self.k1,
            length: self.length,
            state: self.state,
            tail: self.tail,
            ntail: self.ntail,
            _marker: self._marker,
        }
    }
}

#[doc(hidden)]
trait Sip {
    fn c_rounds(_: &mut State);
    fn d_rounds(_: &mut State);
}

#[derive(Debug, Clone, Default)]
struct Sip13Rounds;

impl Sip for Sip13Rounds {
    #[inline]
    fn c_rounds(state: &mut State) {
        compress!(state);
    }

    #[inline]
    fn d_rounds(state: &mut State) {
        compress!(state);
        compress!(state);
        compress!(state);
    }
}

/// All `HashSet`/`HashMap` collections in tested code need to use this `HasherBuilder` for the
/// tests to be reproducible!
///
/// ```
/// use std::collections::HashSet;
/// use falsify::{make_rng_with_seed, HasherBuilder};
///
/// let builder = HasherBuilder::new(make_rng_with_seed(0x12345678));
/// let mut input = HashSet::with_hasher(builder);
/// input.insert(1);
/// ```
#[derive(Clone)]
pub struct HasherBuilder {
    k0: u64,
    k1: u64,
}

impl HasherBuilder {
    pub fn new(rng: Rc<RefCell<StdRng>>) -> HasherBuilder {
        let (k0, k1) = {
            let mut r = rng.borrow_mut();
            let k0: u64 = r.random();
            let k1: u64 = r.random();
            (k0, k1)
        };
        HasherBuilder { k0, k1 }
    }
}

impl BuildHasher for HasherBuilder {
    type Hasher = SipHasher13;

    fn build_hasher(&self) -> SipHasher13 {
        SipHasher13::new_with_keys(self.k0, self.k1)
    }
}

impl std::fmt::Debug for HasherBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RandomState").finish_non_exhaustive()
    }
}
