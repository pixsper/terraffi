use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::{fmt, slice};

#[cfg(feature = "alloc")]
use {alloc::boxed::Box, alloc::vec::Vec};

#[cfg(feature = "serde")]
use {
    serde::de::{SeqAccess, Visitor},
    serde::{Deserialize, Deserializer, Serialize, Serializer},
};

/// Transparent wrapper around a raw C const array pointer to indicate that this represents an array pointer
/// This wrapper implies no ownership, no memory will be freed upon drop.
#[repr(transparent)]
#[derive(Debug)]
pub struct CArrayPointer<T>(*const T);

unsafe impl<T: Send> Send for CArrayPointer<T> {}
unsafe impl<T: Sync> Sync for CArrayPointer<T> {}

impl<T> CArrayPointer<T> {
    /// Returns `true` if the pointer is null.
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Forms a slice from the pointer and a length.
    ///
    /// Returns an empty slice if the pointer is null or `len` is zero.
    ///
    /// # Safety
    ///
    /// If non-null, the pointer must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for the lifetime of the returned slice.
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        unsafe {
            if !self.0.is_null() && len > 0 {
                slice::from_raw_parts(self.0, len)
            } else {
                &[]
            }
        }
    }

    /// Returns the raw const pointer.
    ///
    /// The caller must ensure that the array outlives the pointer this function returns,
    /// or else it will end up dangling.
    pub fn as_ptr(&self) -> *const T {
        self.0
    }
}

impl<T> Default for CArrayPointer<T> {
    fn default() -> Self {
        Self(core::ptr::null())
    }
}

impl<T> Copy for CArrayPointer<T> {}

impl<T> Clone for CArrayPointer<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> From<*const T> for CArrayPointer<T> {
    fn from(value: *const T) -> Self {
        Self(value)
    }
}

impl<T> From<*mut T> for CArrayPointer<T> {
    fn from(value: *mut T) -> Self {
        Self(value)
    }
}

/// Transparent wrapper around a raw C mut array pointer to indicate that this represents an array pointer
/// This wrapper implies no ownership, no memory will be freed upon drop.
#[repr(transparent)]
#[derive(Debug)]
pub struct CArrayMutPointer<T>(*mut T);

unsafe impl<T: Send> Send for CArrayMutPointer<T> {}
unsafe impl<T: Sync> Sync for CArrayMutPointer<T> {}

impl<T> CArrayMutPointer<T> {
    /// Returns `true` if the pointer is null.
    ///
    /// A null pointer indicates that there is no backing array.
    pub fn is_empty(&self) -> bool {
        self.0.is_null()
    }

    /// Forms a shared slice from the pointer and a length.
    ///
    /// Returns an empty slice if the pointer is null or `len` is zero.
    ///
    /// # Safety
    ///
    /// If non-null, the pointer must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for the lifetime of the returned slice.
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        unsafe {
            if !self.0.is_null() && len > 0 {
                slice::from_raw_parts(self.0, len)
            } else {
                &[]
            }
        }
    }

    /// Forms a mutable slice from the pointer and a length.
    ///
    /// Returns an empty slice if the pointer is null or `len` is zero.
    ///
    /// # Safety
    ///
    /// If non-null, the pointer must be valid for reads and writes of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid and exclusively accessible for the
    /// lifetime of the returned slice.
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        unsafe {
            if !self.0.is_null() && len > 0 {
                slice::from_raw_parts_mut(self.0, len)
            } else {
                &mut []
            }
        }
    }

    /// Returns the underlying pointer as a raw const pointer.
    ///
    /// The caller must ensure that the array outlives the pointer this function returns,
    /// or else it will end up dangling.
    pub fn as_ptr(&self) -> *const T {
        self.0
    }

    /// Returns the underlying pointer as a raw mut pointer.
    ///
    /// The caller must ensure that the array outlives the pointer this function returns,
    /// or else it will end up dangling.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.0
    }
}

impl<T> Default for CArrayMutPointer<T> {
    fn default() -> Self {
        Self(core::ptr::null_mut())
    }
}

impl<T> Copy for CArrayMutPointer<T> {}

impl<T> Clone for CArrayMutPointer<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> From<*const T> for CArrayMutPointer<T> {
    fn from(value: *const T) -> Self {
        Self(value as *mut T)
    }
}

impl<T> From<*mut T> for CArrayMutPointer<T> {
    fn from(value: *mut T) -> Self {
        Self(value)
    }
}

impl<T> From<CArrayMutPointer<T>> for *mut T {
    fn from(value: CArrayMutPointer<T>) -> Self {
        value.0
    }
}

impl<T> From<CArrayMutPointer<T>> for CArrayPointer<T> {
    fn from(value: CArrayMutPointer<T>) -> Self {
        Self(value.0)
    }
}

/// An owned C-compatible array, for use in FFI interop.
/// Array data will be freed when dropped.
#[repr(C)]
pub struct CSlice<T> {
    len: usize,
    ptr: CArrayMutPointer<T>,
}

impl<T> CSlice<T> {
    /// Creates a `CSlice` from a raw mutable pointer and a length.
    ///
    /// # Safety
    ///
    /// `data` must point to `len` consecutive, initialized elements of type `T`.
    /// The `CSlice` takes ownership and will free the memory on drop via [`Box`].
    /// The memory must have been allocated in a way compatible with `Box<[T]>` deallocation.
    pub unsafe fn from_raw_parts_mut(data: *mut T, len: usize) -> Self {
        Self {
            len,
            ptr: CArrayMutPointer::from(data),
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

    /// Returns a shared reference view over this slice's contents.
    ///
    /// The returned [`CSliceRef`] borrows from `self` and does not own the data.
    pub fn as_ref(&self) -> CSliceRef<'_, T> {
        CSliceRef {
            _marker: PhantomData,
            len: self.len,
            ptr: self.ptr.into(),
        }
    }

    /// Returns a mutable reference view over this slice's contents.
    ///
    /// The returned [`CSliceMutRef`] borrows from `self` and does not own the data.
    pub fn as_mut(&mut self) -> CSliceMutRef<'_, T> {
        CSliceMutRef {
            _marker: PhantomData,
            len: self.len,
            ptr: self.ptr,
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
        unsafe { self.ptr.as_slice(self.len) }
    }

    /// Returns the contents as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { self.ptr.as_mut_slice(self.len) }
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
        if !self.ptr.is_empty() && self.len > 0 {
            unsafe {
                let len = self.len;
                self.len = 0;
                let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(
                    self.ptr.as_ptr() as *mut T,
                    len,
                ));
            }
        }
    }
}

impl<T> Default for CSlice<T> {
    fn default() -> Self {
        Self {
            len: 0,
            ptr: Default::default(),
        }
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

#[cfg(feature = "alloc")]
impl<T> From<Box<[T]>> for CSlice<T> {
    fn from(value: Box<[T]>) -> Self {
        let raw = Box::into_raw(value);
        Self {
            len: raw.len(),
            ptr: CArrayMutPointer::from(raw as *mut T),
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

/// An referenced C-compatible array, for use in FFI interop.
/// Array data will be NOT freed when dropped.
#[repr(C)]
#[derive(Clone, Default)]
pub struct CSliceRef<'a, T> {
    _marker: PhantomData<&'a [T]>,
    len: usize,
    ptr: CArrayPointer<T>,
}

impl<'a, T> CSliceRef<'a, T> {
    /// Creates a `CSliceRef` borrowing the contents of a [`CSlice`].
    pub fn from_c_slice(c_slice: &'a CSlice<T>) -> Self {
        Self {
            _marker: PhantomData,
            len: c_slice.len(),
            ptr: c_slice.ptr.into(),
        }
    }

    /// Creates a `CSliceRef` from a raw const pointer and a length.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for lifetime `'a`.
    pub unsafe fn from_raw_parts(len: usize, ptr: *const T) -> Self {
        Self {
            _marker: PhantomData,
            len,
            ptr: ptr.into(),
        }
    }

    /// Creates a `CSliceRef` from a raw mutable pointer and a length.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for lifetime `'a`.
    pub unsafe fn from_raw_parts_mut(len: usize, ptr: *mut T) -> Self {
        Self {
            _marker: PhantomData,
            len,
            ptr: ptr.into(),
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
        unsafe { self.ptr.as_slice(self.len) }
    }
}

impl<'a, T: PartialEq> PartialEq for CSliceRef<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<'a, T: Eq> Eq for CSliceRef<'a, T> {}

impl<'a, T: Debug> Debug for CSliceRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// A referenced C-compatible mutable array, for use in FFI interop.
/// Array data will be NOT freed when dropped.
#[repr(C)]
#[derive(Default)]
pub struct CSliceMutRef<'a, T> {
    _marker: PhantomData<&'a [T]>,
    len: usize,
    ptr: CArrayMutPointer<T>,
}

impl<'a, T> CSliceMutRef<'a, T> {
    /// Creates a `CSliceMutRef` borrowing the contents of a [`CSlice`] mutably.
    pub fn from_c_mut_slice(c_slice: &'a CSlice<T>) -> Self {
        Self {
            _marker: PhantomData,
            len: c_slice.len(),
            ptr: c_slice.ptr,
        }
    }

    /// Creates a `CSliceMutRef` from a raw mutable pointer and a length.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, valid for reads and writes of `len * size_of::<T>()` bytes,
    /// properly aligned, and exclusively accessible for lifetime `'a`.
    pub unsafe fn from_raw_parts_mut(len: usize, ptr: *mut T) -> Self {
        Self {
            _marker: PhantomData,
            len,
            ptr: ptr.into(),
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
        unsafe { self.ptr.as_slice(self.len) }
    }

    /// Returns the contents as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { self.ptr.as_mut_slice(self.len) }
    }
}

impl<'a, T: PartialEq> PartialEq for CSliceMutRef<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<'a, T: Eq> Eq for CSliceMutRef<'a, T> {}

impl<'a, T: Debug> Debug for CSliceMutRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// An owned C-compatible resizable array, for use in FFI interop.
/// Array data will be freed when dropped.
#[repr(C)]
pub struct CVec<T> {
    len: usize,
    capacity: usize,
    ptr: CArrayMutPointer<T>,
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
        CSliceRef {
            _marker: PhantomData,
            len: self.len,
            ptr: self.ptr.into(),
        }
    }

    /// Returns a mutable reference view over this vector's initialized elements and capacity.
    ///
    /// The returned [`CVecMutRef`] borrows from `self` and does not own the data.
    pub fn as_mut(&mut self) -> CVecMutRef<'_, T> {
        CVecMutRef {
            _marker: PhantomData,
            len: self.len,
            capacity: self.capacity,
            ptr: self.ptr,
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
        unsafe { self.ptr.as_slice(self.len) }
    }

    /// Returns the initialized elements as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { self.ptr.as_mut_slice(self.len) }
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
        let parts = (self.ptr.0, self.len, self.capacity);
        core::mem::forget(self);
        parts
    }

    #[cfg(feature = "alloc")]
    fn with_vec<F: Fn(&mut Vec<T>)>(&mut self, f: F) {
        let mut v = if self.is_empty() {
            Vec::new()
        } else {
            unsafe { Vec::from_raw_parts(self.ptr.as_mut_ptr(), self.len, self.capacity) }
        };
        f(&mut v);
        let (ptr, len, capacity) = v.into_raw_parts();
        self.capacity = capacity;
        self.len = len;
        self.ptr = CArrayMutPointer::from(ptr);
    }
}

/// Creates a [`CVec`] from a list of elements, mirroring the [`vec!`] macro.
///
/// - `cvec![a, b, c]` — creates a `CVec` containing the given elements.
/// - `cvec![va
/// l; n]` — creates a `CVec` with `n` copies of `val`.
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
/// - `cvec![va
/// l; n]` — creates a `CVec` with `n` copies of `val`.
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
        if !self.ptr.is_empty() && self.capacity > 0 && self.len <= self.capacity {
            let _ = unsafe { Vec::from_raw_parts(self.ptr.0, self.len, self.capacity) };
        }
    }
}

impl<T> Default for CVec<T> {
    fn default() -> Self {
        Self {
            len: 0,
            capacity: 0,
            ptr: Default::default(),
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
            ptr: CArrayMutPointer(ptr),
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
    _marker: PhantomData<&'a [T]>,
    len: usize,
    capacity: usize,
    ptr: CArrayPointer<T>,
}

impl<'a, T> CVecRef<'a, T> {
    /// Creates a `CVecRef` borrowing the contents of a [`CVec`].
    pub fn from_c_vec(c_vec: &'a CVec<T>) -> Self {
        Self {
            _marker: PhantomData,
            len: c_vec.len(),
            capacity: c_vec.capacity(),
            ptr: c_vec.ptr.into(),
        }
    }

    /// Creates a `CVecRef` from a raw const pointer, a length, and a capacity.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for lifetime `'a`.
    /// `capacity` must be the actual allocation capacity behind `ptr`.
    pub unsafe fn from_raw_parts(len: usize, capacity: usize, ptr: *const T) -> Self {
        Self {
            _marker: PhantomData,
            len,
            capacity,
            ptr: ptr.into(),
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
        unsafe { self.ptr.as_slice(self.len) }
    }
}

impl<'a, T> Default for CVecRef<'a, T> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
            len: 0,
            capacity: 0,
            ptr: Default::default(),
        }
    }
}

impl<'a, T: PartialEq> PartialEq for CVecRef<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<'a, T: Eq> Eq for CVecRef<'a, T> {}

impl<'a, T: Debug> Debug for CVecRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// A referenced C-compatible resizable mutable array, for use in FFI interop.
/// Array data will NOT be freed when dropped.
#[repr(C)]
pub struct CVecMutRef<'a, T> {
    _marker: PhantomData<&'a [T]>,
    len: usize,
    capacity: usize,
    ptr: CArrayMutPointer<T>,
}

impl<'a, T> CVecMutRef<'a, T> {
    /// Creates a `CVecMutRef` borrowing the contents of a [`CVec`] mutably.
    pub fn from_c_vec(c_vec: &'a CVec<T>) -> Self {
        Self {
            _marker: PhantomData,
            len: c_vec.len(),
            capacity: c_vec.capacity(),
            ptr: c_vec.ptr,
        }
    }

    /// Creates a `CVecMutRef` from a raw mutable pointer, a length, and a capacity.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, valid for reads and writes of `len * size_of::<T>()` bytes,
    /// properly aligned, and exclusively accessible for lifetime `'a`.
    /// `capacity` must be the actual allocation capacity behind `ptr`.
    pub unsafe fn from_raw_parts_mut(len: usize, capacity: usize, ptr: *mut T) -> Self {
        Self {
            _marker: PhantomData,
            len,
            capacity,
            ptr: ptr.into(),
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
        unsafe { self.ptr.as_slice(self.len) }
    }

    /// Returns the initialized elements as a mutable slice `&mut [T]`.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { self.ptr.as_mut_slice(self.len) }
    }
}

impl<'a, T> Default for CVecMutRef<'a, T> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
            len: 0,
            capacity: 0,
            ptr: Default::default(),
        }
    }
}

impl<'a, T: PartialEq> PartialEq for CVecMutRef<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<'a, T: Eq> Eq for CVecMutRef<'a, T> {}

impl<'a, T: Debug> Debug for CVecMutRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
    use super::*;
    use alloc::vec;

    // --- CSlice ---

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

    // --- CVec ---

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
