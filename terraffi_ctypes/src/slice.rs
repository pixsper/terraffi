use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
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

use crate::{CArrayPtr, CArrayPtrMut, CArrayPtrMutRef};

/// An owned C-compatible array, for use in FFI interop.
/// Array data will be freed when dropped.
#[repr(C)]
pub struct CSlice<T> {
    ptr: Option<CArrayPtrMut<T>>,
    len: usize,
}

impl<T> CSlice<T> {
    /// Creates a `CSlice` from a raw mutable pointer and a length.
    ///
    /// # Safety
    ///
    /// - When `len > 0`, `data` must be non-null and point to `len` consecutive,
    ///   initialized elements of type `T`. Passing a null pointer with `len > 0`
    ///   is undefined behaviour.
    /// - When `len == 0`, `data` is ignored and the resulting `CSlice` will not
    ///   take ownership of any allocation; callers passing a non-null pointer
    ///   with `len == 0` are responsible for freeing it themselves.
    /// - The `CSlice` takes ownership and will free the memory on drop via [`Box`];
    ///   the memory must have been allocated in a way compatible with `Box<[T]>`
    ///   deallocation.
    pub unsafe fn from_raw_parts_mut(data: *mut T, len: usize) -> Self {
        let ptr = if len > 0 {
            Some(unsafe { CArrayPtrMut::from_raw(data) })
        } else {
            None
        };

        Self { ptr, len }
    }

    /// Returns `true` if the slice contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns a shared reference view over this slice's contents.
    ///
    /// The returned [`CSliceRef`] borrows from `self` and does not own the data.
    pub fn as_ref(&self) -> CSliceRef<'_, T> {
        CSliceRef {
            ptr: self.ptr.as_ref().map(CArrayPtr::from),
            len: self.len,
            _marker: PhantomData,
        }
    }

    /// Returns a mutable reference view over this slice's contents.
    ///
    /// The returned [`CSliceMutRef`] borrows from `self` and does not own the data.
    pub fn as_mut(&mut self) -> CSliceMutRef<'_, T> {
        CSliceMutRef {
            ptr: self.ptr.as_mut().map(CArrayPtrMutRef::from),
            len: self.len,
        }
    }

    /// Returns an iterator over the slice.
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

    /// Returns the contents as a shared slice `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ref ptr) => unsafe { ptr.as_slice(self.len) },
            None => &[],
        }
    }

    /// Returns the contents as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self.ptr {
            Some(ref mut ptr) => unsafe { ptr.as_mut_slice(self.len) },
            None => &mut [],
        }
    }
}

/// Creates a [`CSlice`] from a list of elements, mirroring the [`vec!`] macro.
///
/// - `cslice![a, b, c]` — creates a `CSlice` containing the given elements.
/// - `cslice![val; n]` — creates a `CSlice` with `n` copies of `val`.
#[cfg(feature = "alloc")]
#[cfg(feature = "std")]
#[macro_export]
macro_rules! cslice {
    () => {
        $crate::CSlice::default()
    };
    ($elem:expr; $n:expr) => {
        $crate::CSlice::from(std::vec![$elem; $n])
    };
    ($($x:expr),+ $(,)?) => {
        $crate::CSlice::from(std::vec![$($x),+])
    };
}

#[cfg(feature = "alloc")]
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! cslice {
    () => {
        $crate::CSlice::default()
    };
    ($elem:expr; $n:expr) => {
        $crate::CSlice::from(alloc::vec![$elem; $n])
    };
    ($($x:expr),+ $(,)?) => {
        $crate::CSlice::from(alloc::vec![$($x),+])
    };
}

#[cfg(feature = "alloc")]
impl<T> Drop for CSlice<T> {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr.take()
            && self.len > 0
        {
            unsafe {
                let len = self.len;
                self.len = 0;
                let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(
                    ptr.into_non_null().as_ptr(),
                    len,
                ));
            }
        }
    }
}

impl<T> Default for CSlice<T> {
    fn default() -> Self {
        Self { len: 0, ptr: None }
    }
}

impl<T: Clone> Clone for CSlice<T> {
    fn clone(&self) -> Self {
        self.iter().cloned().collect()
    }
}

impl<T: PartialEq> PartialEq for CSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for CSlice<T> {}

impl<T: Debug> Debug for CSlice<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Hash> Hash for CSlice<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

#[cfg(feature = "alloc")]
impl<T> From<Box<[T]>> for CSlice<T> {
    fn from(value: Box<[T]>) -> Self {
        let raw = Box::into_raw(value);
        let len = raw.len();
        Self {
            ptr: NonNull::new(raw as *mut T).map(CArrayPtrMut::from_non_null),
            len,
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> From<Vec<T>> for CSlice<T> {
    fn from(value: Vec<T>) -> Self {
        value.into_boxed_slice().into()
    }
}

#[cfg(feature = "alloc")]
impl<T> FromIterator<T> for CSlice<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from(Vec::from_iter(iter))
    }
}

#[cfg(feature = "serde")]
impl<T: Serialize> Serialize for CSlice<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_slice().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
#[cfg(feature = "alloc")]
impl<'de, T> Deserialize<'de> for CSlice<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CSliceVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for CSliceVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = CSlice<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a sequence to be deserialized into a CSlice")
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

        deserializer.deserialize_seq(CSliceVisitor(PhantomData))
    }
}

/// A referenced C-compatible array, for use in FFI interop.
/// Array data will NOT be freed when dropped.
#[repr(C)]
#[derive(Clone)]
pub struct CSliceRef<'a, T> {
    ptr: Option<CArrayPtr<T>>,
    len: usize,
    _marker: PhantomData<&'a [T]>,
}

impl<'a, T> CSliceRef<'a, T> {
    /// Creates a `CSliceRef` borrowing the contents of a [`CSlice`].
    pub fn from_c_slice(c_slice: &'a CSlice<T>) -> Self {
        Self {
            len: c_slice.len(),
            ptr: c_slice.ptr.as_ref().map(CArrayPtr::from),
            _marker: PhantomData,
        }
    }

    /// Creates a `CSliceRef` from a raw const pointer and a length.
    ///
    /// # Safety
    ///
    /// If non-null, `ptr` must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for lifetime `'a`.
    pub unsafe fn from_raw_parts(ptr: *const T, len: usize) -> Self {
        Self {
            len,
            ptr: NonNull::new(ptr as *mut T).map(CArrayPtr::from_non_null),
            _marker: PhantomData,
        }
    }

    /// Creates a `CSliceRef` from a raw mutable pointer and a length.
    ///
    /// # Safety
    ///
    /// If non-null, `ptr` must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for lifetime `'a`.
    pub unsafe fn from_raw_parts_mut(ptr: *mut T, len: usize) -> Self {
        Self {
            len,
            ptr: NonNull::new(ptr).map(CArrayPtr::from_non_null),
            _marker: PhantomData,
        }
    }

    /// Returns `true` if the slice contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns an iterator over the slice.
    ///
    /// The iterator yields all items from start to end.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns the contents as a shared slice `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ref ptr) => unsafe { ptr.as_slice(self.len) },
            None => &[],
        }
    }
}

impl<T> Default for CSliceRef<'_, T> {
    fn default() -> Self {
        Self {
            len: 0,
            ptr: None,
            _marker: PhantomData,
        }
    }
}

impl<T: PartialEq> PartialEq for CSliceRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for CSliceRef<'_, T> {}

impl<T: Debug> Debug for CSliceRef<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Hash> Hash for CSliceRef<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

/// A referenced C-compatible mutable array, for use in FFI interop.
/// Array data will NOT be freed when dropped.
#[repr(C)]
pub struct CSliceMutRef<'a, T> {
    ptr: Option<CArrayPtrMutRef<'a, T>>,
    len: usize,
}

impl<'a, T> CSliceMutRef<'a, T> {
    /// Creates a `CSliceMutRef` borrowing the contents of a [`CSlice`] mutably.
    pub fn from_c_slice(c_slice: &'a mut CSlice<T>) -> Self {
        let len = c_slice.len();
        Self {
            ptr: c_slice.ptr.as_mut().map(CArrayPtrMutRef::from),
            len,
        }
    }

    /// Creates a `CSliceMutRef` from a raw mutable pointer and a length.
    ///
    /// # Safety
    ///
    /// If non-null, `ptr` must be valid for reads and writes of `len * size_of::<T>()` bytes,
    /// properly aligned, and exclusively accessible for lifetime `'a`.
    pub unsafe fn from_raw_parts_mut(ptr: *mut T, len: usize) -> Self {
        Self {
            ptr: NonNull::new(ptr).map(|_| unsafe { CArrayPtrMutRef::from_raw(ptr) }),
            len,
        }
    }

    /// Returns `true` if the slice contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns an iterator over the slice.
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

    /// Returns the contents as a shared slice `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ref ptr) => unsafe { ptr.as_slice(self.len) },
            None => &[],
        }
    }

    /// Returns the contents as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self.ptr {
            Some(ref mut ptr) => unsafe { ptr.as_mut_slice(self.len) },
            None => &mut [],
        }
    }
}

impl<T: PartialEq> PartialEq for CSliceMutRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for CSliceMutRef<'_, T> {}

impl<T: Debug> Debug for CSliceMutRef<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Hash> Hash for CSliceMutRef<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_are_correctly_sized() {
        assert_eq!(size_of::<CSlice<i32>>(), size_of::<usize>() * 2);
        assert_eq!(size_of::<CSliceRef<i32>>(), size_of::<usize>() * 2);
        assert_eq!(size_of::<CSliceMutRef<i32>>(), size_of::<usize>() * 2);
    }

    #[test]
    fn test_collect() {
        let v = vec![1, 2, 3, 4, 5];
        let cslice: CSlice<i32> = v.into_iter().collect();
        assert_eq!(cslice.len(), 5);
        assert_eq!(cslice.as_slice(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_cslice_default_is_empty() {
        let s: CSlice<i32> = CSlice::default();
        assert_eq!(s.len(), 0);
        assert_eq!(s.as_slice(), &[]);
    }

    #[test]
    fn test_cslice_from_vec() {
        let s: CSlice<i32> = vec![10, 20, 30].into();
        assert_eq!(s.len(), 3);
        assert_eq!(s.as_slice(), &[10, 20, 30]);
    }

    #[test]
    fn test_cslice_iter_mut() {
        let mut s: CSlice<i32> = vec![1, 2, 3].into();
        for x in s.iter_mut() {
            *x *= 2;
        }
        assert_eq!(s.as_slice(), &[2, 4, 6]);
    }

    #[test]
    fn test_cslice_as_mut_slice() {
        let mut s: CSlice<i32> = vec![1, 2, 3].into();
        s.as_mut_slice()[1] = 99;
        assert_eq!(s.as_slice(), &[1, 99, 3]);
    }

    #[test]
    fn test_cslice_clone_is_independent() {
        let a: CSlice<i32> = vec![1, 2, 3].into();
        let mut b = a.clone();
        b.as_mut_slice()[0] = 99;
        assert_eq!(a.as_slice(), &[1, 2, 3]);
        assert_eq!(b.as_slice(), &[99, 2, 3]);
    }

    #[test]
    fn test_cslice_eq() {
        let a: CSlice<i32> = vec![1, 2, 3].into();
        let b: CSlice<i32> = vec![1, 2, 3].into();
        let c: CSlice<i32> = vec![1, 2, 4].into();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
