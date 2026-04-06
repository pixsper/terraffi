use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::{fmt, slice};

#[cfg(feature = "alloc")]
use {alloc::boxed::Box, alloc::vec::Vec};

#[cfg(feature = "serde")]
use {
    serde::de::{SeqAccess, Visitor},
    serde::{Deserialize, Deserializer, Serialize, Serializer},
};

use crate::{CArrayPtr, CArrayPtrMut, CSliceRef};

/// An owned C-compatible resizable array, for use in FFI interop.
/// Array data will be freed when dropped.
#[repr(C)]
pub struct CVec<T> {
    ptr: Option<CArrayPtrMut<T>>,
    len: usize,
    capacity: usize,
}

impl<T> CVec<T> {
    /// Creates a `CVec<T>` directly from a pointer, a length, and a capacity.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. See [`Vec::from_raw_parts`] for the full set of invariants
    /// that must be upheld.
    #[cfg(feature = "alloc")]
    pub unsafe fn from_raw_parts(ptr: *mut T, len: usize, capacity: usize) -> Self {
        unsafe { Vec::from_raw_parts(ptr, len, capacity) }.into()
    }

    /// Constructs a new, empty `CVec<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    pub fn new() -> Self {
        Default::default()
    }

    /// Constructs a new, empty `CVec<T>` with at least the specified capacity.
    ///
    /// The vector will be able to hold at least `capacity` elements without reallocating.
    #[cfg(feature = "alloc")]
    pub fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity).into()
    }

    /// Returns the number of elements in the vector.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the total number of elements the vector can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a shared reference view over this vector's initialized elements.
    ///
    /// The returned [`CSliceRef`] borrows from `self` and does not own the data.
    pub fn as_ref(&self) -> CSliceRef<'_, T> {
        let ptr = self
            .ptr
            .as_ref()
            .map(|p| p.as_non_null().as_ptr() as *const T)
            .unwrap_or(core::ptr::null());
        // SAFETY: `self` upholds CVec's invariants for `(ptr, len)`; the returned
        // `CSliceRef` borrows from `self` for `'_` so the memory remains valid.
        unsafe { CSliceRef::from_raw_parts(ptr, self.len) }
    }

    /// Returns a mutable reference view over this vector's initialized elements and capacity.
    ///
    /// The returned [`CVecMutRef`] borrows from `self` and does not own the data.
    pub fn as_mut(&mut self) -> CVecMutRef<'_, T> {
        CVecMutRef {
            len: self.len,
            capacity: self.capacity,
            // SAFETY: the new `CArrayPtrMut` aliases `self.ptr` for the duration
            // of the `CVecMutRef`'s `'_` borrow of `self`, which the borrow
            // checker enforces — no double-free can occur because `CArrayPtrMut`
            // does not free on drop.
            ptr: self
                .ptr
                .as_ref()
                .map(|p| CArrayPtrMut::from_non_null(p.as_non_null())),
            _marker: PhantomData,
        }
    }

    /// Returns an iterator over the initialized elements of the vector.
    ///
    /// The iterator yields all items from start to end.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns an iterator that allows modifying each value.
    ///
    /// The iterator yields all items from start to end.
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    /// Returns the initialized elements as a shared slice `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ref ptr) => unsafe { ptr.as_slice(self.len) },
            None => &[],
        }
    }

    /// Returns the initialized elements as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self.ptr {
            Some(ref mut ptr) => unsafe { ptr.as_mut_slice(self.len) },
            None => &mut [],
        }
    }

    /// Clears the vector, removing all values.
    #[cfg(feature = "alloc")]
    pub fn clear(&mut self) {
        self.with_vec(|v| v.clear());
    }

    /// Reserves capacity for at least `additional` more elements to be inserted in the given `CVec<T>`.
    /// The collection may reserve more space to speculatively avoid frequent reallocations.
    /// After calling `reserve`, capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient
    #[cfg(feature = "alloc")]
    pub fn reserve(&mut self, additional: usize) {
        if self.len + additional <= self.capacity {
            return;
        }

        self.with_vec(|v| v.reserve(additional));
    }

    /// Reserves the minimum capacity for at least `additional` more elements to be inserted in the given `CVec<T>`.
    /// Unlike reserve, this will not deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if the capacity is already sufficient.
    #[cfg(feature = "alloc")]
    pub fn reserve_exact(&mut self, additional: usize) {
        if self.len + additional <= self.capacity {
            return;
        }
        self.with_vec(|v| v.reserve_exact(additional));
    }

    /// Shrinks the capacity of the vector as much as possible.
    #[cfg(feature = "alloc")]
    pub fn shrink_to_fit(&mut self) {
        if self.capacity <= self.len {
            return;
        }
        self.with_vec(|v| v.shrink_to_fit());
    }

    /// Decomposes a `CVec<T>` into its raw components: (pointer, length, capacity).
    pub fn into_raw_parts(self) -> (*mut T, usize, usize) {
        let ptr = match &self.ptr {
            Some(p) => p.as_non_null().as_ptr(),
            None => core::ptr::null_mut(),
        };
        let parts = (ptr, self.len, self.capacity);
        core::mem::forget(self);
        parts
    }

    #[cfg(feature = "alloc")]
    fn with_vec<F: Fn(&mut Vec<T>)>(&mut self, f: F) {
        let mut v = match &self.ptr {
            Some(ptr) if self.len > 0 || self.capacity > 0 => unsafe {
                Vec::from_raw_parts(ptr.as_non_null().as_ptr(), self.len, self.capacity)
            },
            _ => Vec::new(),
        };
        // Detach `self` from the allocation before calling `f`, so that if `f`
        // panics the allocation is freed exactly once (by `v`'s drop) and `self`
        // is left in a valid empty state.
        self.ptr = None;
        self.len = 0;
        self.capacity = 0;
        f(&mut v);
        let (ptr, len, capacity) = v.into_raw_parts();
        self.ptr = NonNull::new(ptr).map(CArrayPtrMut::from_non_null);
        self.len = len;
        self.capacity = capacity;
    }
}

/// Creates a [`CVec`] from a list of elements, mirroring the [`vec!`] macro.
///
/// - `cvec![a, b, c]` — creates a `CVec` containing the given elements.
/// - `cvec![val; n]` — creates a `CVec` with `n` copies of `val`.
#[cfg(feature = "alloc")]
#[cfg(feature = "std")]
#[macro_export]
macro_rules! cvec {
    () => {
        $crate::CVec::default()
    };
    ($elem:expr; $n:expr) => {
        $crate::CVec::from(std::vec![$elem; $n])
    };
    ($($x:expr),+ $(,)?) => {
        $crate::CVec::from(std::vec![$($x),+])
    };
}

/// Creates a [`CVec`] from a list of elements, mirroring the [`vec!`] macro.
///
/// - `cvec![a, b, c]` — creates a `CVec` containing the given elements.
/// - `cvec![val; n]` — creates a `CVec` with `n` copies of `val`.
#[cfg(feature = "alloc")]
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! cvec {
    () => {
        $crate::CVec::default()
    };
    ($elem:expr; $n:expr) => {
        $crate::CVec::from(alloc::vec![$elem; $n])
    };
    ($($x:expr),+ $(,)?) => {
        $crate::CVec::from(alloc::vec![$($x),+])
    };
}

#[cfg(feature = "alloc")]
impl<T> Drop for CVec<T> {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr.take()
            && self.capacity > 0
            && self.len <= self.capacity
        {
            let _ = unsafe {
                Vec::from_raw_parts(ptr.into_non_null().as_ptr(), self.len, self.capacity)
            };
        }
    }
}

impl<T> Default for CVec<T> {
    fn default() -> Self {
        Self {
            len: 0,
            capacity: 0,
            ptr: None,
        }
    }
}

impl<T: Clone> Clone for CVec<T> {
    fn clone(&self) -> Self {
        self.iter().cloned().collect()
    }
}

impl<T: PartialEq> PartialEq for CVec<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for CVec<T> {}

impl<T: Debug> Debug for CVec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(feature = "alloc")]
impl<T> From<Box<[T]>> for CVec<T> {
    fn from(value: Box<[T]>) -> Self {
        Vec::from(value).into()
    }
}

#[cfg(feature = "alloc")]
impl<T> From<Vec<T>> for CVec<T> {
    fn from(value: Vec<T>) -> Self {
        let (ptr, len, capacity) = value.into_raw_parts();
        Self {
            len,
            capacity,
            ptr: NonNull::new(ptr).map(CArrayPtrMut::from_non_null),
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> FromIterator<T> for CVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Vec::from_iter(iter).into()
    }
}

#[cfg(feature = "serde")]
impl<T: Serialize> Serialize for CVec<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_slice().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
#[cfg(feature = "alloc")]
impl<'de, T> Deserialize<'de> for CVec<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CVecVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for CVecVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = CVec<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a sequence to be deserialized into a CVec")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut v = match seq.size_hint() {
                    None => Vec::new(),
                    Some(s) => Vec::with_capacity(s),
                };
                while let Some(elem) = seq.next_element()? {
                    v.push(elem);
                }

                Ok(v.into())
            }
        }

        deserializer.deserialize_seq(CVecVisitor(PhantomData))
    }
}

/// A referenced C-compatible resizable array, for use in FFI interop.
/// Array data will NOT be freed when dropped.
#[repr(C)]
pub struct CVecRef<'a, T> {
    ptr: Option<CArrayPtr<T>>,
    len: usize,
    capacity: usize,
    _marker: PhantomData<&'a [T]>,
}

impl<'a, T> CVecRef<'a, T> {
    /// Creates a `CVecRef` borrowing the contents of a [`CVec`].
    pub fn from_c_vec(c_vec: &'a CVec<T>) -> Self {
        Self {
            len: c_vec.len(),
            capacity: c_vec.capacity(),
            ptr: c_vec.ptr.as_ref().map(CArrayPtr::from),
            _marker: PhantomData,
        }
    }

    /// Creates a `CVecRef` from a raw const pointer, a length, and a capacity.
    ///
    /// # Safety
    ///
    /// If non-null, `ptr` must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for lifetime `'a`.
    /// `capacity` must be the actual allocation capacity behind `ptr`.
    pub unsafe fn from_raw_parts(ptr: *const T, len: usize, capacity: usize) -> Self {
        Self {
            _marker: PhantomData,
            len,
            capacity,
            ptr: NonNull::new(ptr as *mut T).map(CArrayPtr::from_non_null),
        }
    }

    /// Returns the number of initialized elements.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the total number of elements the allocation can hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns `true` if there are no initialized elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the initialized elements.
    ///
    /// The iterator yields all items from start to end.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns the initialized elements as a shared slice `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ref ptr) => unsafe { ptr.as_slice(self.len) },
            None => &[],
        }
    }
}

impl<T> Default for CVecRef<'_, T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            len: 0,
            capacity: 0,
            ptr: None,
        }
    }
}

impl<T: PartialEq> PartialEq for CVecRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for CVecRef<'_, T> {}

impl<T: Debug> Debug for CVecRef<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// A referenced C-compatible resizable mutable array, for use in FFI interop.
/// Array data will NOT be freed when dropped.
#[repr(C)]
pub struct CVecMutRef<'a, T> {
    ptr: Option<CArrayPtrMut<T>>,
    len: usize,
    capacity: usize,
    _marker: PhantomData<&'a mut [T]>,
}

impl<'a, T> CVecMutRef<'a, T> {
    /// Creates a `CVecMutRef` borrowing the contents of a [`CVec`] mutably.
    pub fn from_c_vec(c_vec: &'a mut CVec<T>) -> Self {
        let len = c_vec.len();
        let capacity = c_vec.capacity();
        Self {
            // SAFETY: the new `CArrayPtrMut` aliases `c_vec.ptr` for the duration
            // of the `'a` borrow; `CArrayPtrMut` does not free on drop, so no
            // double-free can occur.
            ptr: c_vec
                .ptr
                .as_ref()
                .map(|p| CArrayPtrMut::from_non_null(p.as_non_null())),
            len,
            capacity,
            _marker: PhantomData,
        }
    }

    /// Creates a `CVecMutRef` from a raw mutable pointer, a length, and a capacity.
    ///
    /// # Safety
    ///
    /// If non-null, `ptr` must be valid for reads and writes of `len * size_of::<T>()` bytes,
    /// properly aligned, and exclusively accessible for lifetime `'a`.
    /// `capacity` must be the actual allocation capacity behind `ptr`.
    pub unsafe fn from_raw_parts_mut(ptr: *mut T, len: usize, capacity: usize) -> Self {
        Self {
            ptr: NonNull::new(ptr).map(CArrayPtrMut::from_non_null),
            len,
            capacity,
            _marker: PhantomData,
        }
    }

    /// Returns the number of initialized elements.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the total number of elements the allocation can hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns `true` if there are no initialized elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the initialized elements.
    ///
    /// The iterator yields all items from start to end.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns an iterator that allows modifying each value.
    ///
    /// The iterator yields all items from start to end.
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    /// Returns the initialized elements as a shared slice `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ref ptr) => unsafe { ptr.as_slice(self.len) },
            None => &[],
        }
    }

    /// Returns the initialized elements as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self.ptr {
            Some(ref mut ptr) => unsafe { ptr.as_mut_slice(self.len) },
            None => &mut [],
        }
    }
}

impl<T> Default for CVecMutRef<'_, T> {
    fn default() -> Self {
        Self {
            len: 0,
            capacity: 0,
            ptr: None,
            _marker: PhantomData,
        }
    }
}

impl<T: PartialEq> PartialEq for CVecMutRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for CVecMutRef<'_, T> {}

impl<T: Debug> Debug for CVecMutRef<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_are_correctly_sized() {
        assert_eq!(size_of::<CVec<i32>>(), size_of::<usize>() * 3);
        assert_eq!(size_of::<CVecRef<i32>>(), size_of::<usize>() * 3);
        assert_eq!(size_of::<CVecMutRef<i32>>(), size_of::<usize>() * 3);
    }

    #[test]
    fn test_cvec_new_is_empty() {
        let v: CVec<i32> = CVec::new();
        assert_eq!(v.len(), 0);
        assert_eq!(v.capacity(), 0);
        assert!(v.is_empty());
    }

    #[test]
    fn test_cvec_with_capacity() {
        let v: CVec<i32> = CVec::with_capacity(10);
        assert_eq!(v.len(), 0);
        assert!(v.capacity() >= 10);
    }

    #[test]
    fn test_cvec_from_vec() {
        let v: CVec<i32> = vec![1, 2, 3].into();
        assert_eq!(v.len(), 3);
        assert_eq!(v.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn test_cvec_reserve_grows_capacity() {
        let mut v: CVec<i32> = CVec::new();
        v.reserve(10);
        assert!(v.capacity() >= 10);
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn test_cvec_reserve_noop_when_sufficient() {
        let mut v: CVec<i32> = CVec::with_capacity(20);
        let cap_before = v.capacity();
        v.reserve(5);
        assert_eq!(v.capacity(), cap_before);
    }

    #[test]
    fn test_cvec_reserve_exact() {
        let mut v: CVec<i32> = CVec::new();
        v.reserve_exact(10);
        assert!(v.capacity() >= 10);
    }

    #[test]
    fn test_cvec_shrink_to_fit() {
        let mut v: CVec<i32> = vec![1, 2, 3].into();
        v.reserve(100);
        assert!(v.capacity() >= 100);
        v.shrink_to_fit();
        assert_eq!(v.capacity(), v.len());
        assert_eq!(v.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn test_cvec_clear_empties_but_keeps_capacity() {
        let mut v: CVec<i32> = vec![1, 2, 3].into();
        let cap = v.capacity();
        v.clear();
        assert_eq!(v.len(), 0);
        assert_eq!(v.capacity(), cap);
        assert_eq!(v.as_slice(), &[]);
    }

    #[test]
    fn test_cvec_iter_mut() {
        let mut v: CVec<i32> = vec![1, 2, 3].into();
        for x in v.iter_mut() {
            *x *= 10;
        }
        assert_eq!(v.as_slice(), &[10, 20, 30]);
    }

    #[test]
    fn test_cvec_as_mut_slice() {
        let mut v: CVec<i32> = vec![1, 2, 3].into();
        v.as_mut_slice()[0] = 42;
        assert_eq!(v.as_slice(), &[42, 2, 3]);
    }

    #[test]
    fn test_cvec_clone_is_independent() {
        let a: CVec<i32> = vec![1, 2, 3].into();
        let mut b = a.clone();
        b.as_mut_slice()[0] = 99;
        assert_eq!(a.as_slice(), &[1, 2, 3]);
        assert_eq!(b.as_slice(), &[99, 2, 3]);
    }

    #[test]
    fn test_cvec_eq() {
        let a: CVec<i32> = vec![1, 2, 3].into();
        let b: CVec<i32> = vec![1, 2, 3].into();
        let c: CVec<i32> = vec![4, 5, 6].into();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
