use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::{fmt, slice};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Transparent wrapper around a non-null C const array pointer to indicate that this represents an array pointer.
/// This is guaranteed to be the size of a pointer.
/// This wrapper implies no ownership, no memory will be freed upon drop.
/// The interior pointer can never be null, so when used as part of FFI interop
/// it should be used as `Option<CArrayPtr<T>>` when non-null values cannot be guaranteed.
/// `Option<CArrayPtr<T>>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CArrayPtr<T>(NonNull<T>);

unsafe impl<T: Sync> Send for CArrayPtr<T> {}
unsafe impl<T: Sync> Sync for CArrayPtr<T> {}

impl<T> CArrayPtr<T> {
    /// Creates a `CArrayPtr` from a raw pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, properly aligned, and the memory it points to must remain
    /// valid for the intended lifetime of this `CArrayPtr`.
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        Self(unsafe { NonNull::new_unchecked(ptr as *mut T) })
    }

    /// Creates a `CArrayPtr` from a [`NonNull`] pointer.
    ///
    /// Construction itself is safe; the methods that read through the wrapper are
    /// `unsafe` and require the pointer to be valid for the intended use.
    pub fn from_non_null(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }

    /// Returns the underlying [`NonNull`] pointer.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.0
    }

    /// Forms a slice from the pointer and a length.
    ///
    /// Returns an empty slice if `len` is zero.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for the lifetime of the returned slice.
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        unsafe {
            if len > 0 {
                slice::from_raw_parts(self.0.as_ptr(), len)
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
        self.0.as_ptr()
    }
}

impl<T> Copy for CArrayPtr<T> {}

impl<T> Clone for CArrayPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Debug> Debug for CArrayPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CArrayPtr").field(&self.0).finish()
    }
}

/// Transparent wrapper around a non-null C const array pointer to indicate that this represents an array pointer used as a reference.
/// This is guaranteed to be the size of a pointer.
/// This wrapper implies no ownership, no memory will be freed upon drop.
/// Similar to [`CStringPtrRef`](crate::CStringPtrRef) the interior pointer can never be null, so when used as part of FFI interop
/// it should be used as `Option<CArrayPtrRef<T>>` when non-null values cannot be guaranteed.
/// `Option<CArrayPtrRef<T>>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CArrayPtrRef<'a, T> {
    ptr: NonNull<T>,
    _marker: PhantomData<&'a T>,
}

unsafe impl<T: Sync> Send for CArrayPtrRef<'_, T> {}
unsafe impl<T: Sync> Sync for CArrayPtrRef<'_, T> {}

impl<'a, T> CArrayPtrRef<'a, T> {
    /// Creates a `CArrayPtrRef` from a raw pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, properly aligned, and the memory it points to must remain
    /// valid for the lifetime `'a`.
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr as *mut T) },
            _marker: PhantomData,
        }
    }

    /// Creates a `CArrayPtrRef` from a [`NonNull`] pointer.
    ///
    /// Construction itself is safe; the methods that read through the wrapper are
    /// `unsafe` and require the pointer to be valid for `'a`.
    pub fn from_non_null(ptr: NonNull<T>) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Returns the underlying [`NonNull`] pointer.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.ptr
    }

    pub fn from_slice(slice: &'a [T]) -> Self {
        slice.into()
    }

    /// Forms a slice from the pointer and a length.
    ///
    /// Returns an empty slice if `len` is zero.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for the lifetime of the returned slice.
    pub unsafe fn as_slice(&self, len: usize) -> &'a [T] {
        unsafe {
            if len > 0 {
                slice::from_raw_parts(self.ptr.as_ptr(), len)
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
        self.ptr.as_ptr()
    }
}

impl<T> Copy for CArrayPtrRef<'_, T> {}

impl<T> Clone for CArrayPtrRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Debug> Debug for CArrayPtrRef<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CArrayPtrRef").field(&self.ptr).finish()
    }
}

impl<'a, T> From<&'a [T]> for CArrayPtrRef<'a, T> {
    fn from(value: &'a [T]) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(value.as_ptr() as *mut T) },
            _marker: PhantomData,
        }
    }
}

impl<'a, T> From<&'a mut [T]> for CArrayPtrRef<'a, T> {
    fn from(value: &'a mut [T]) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(value.as_mut_ptr()) },
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, T> From<&'a Vec<T>> for CArrayPtrRef<'a, T> {
    fn from(value: &'a Vec<T>) -> Self {
        Self::from(value.as_slice())
    }
}

#[cfg(feature = "alloc")]
impl<'a, T> From<&'a mut Vec<T>> for CArrayPtrRef<'a, T> {
    fn from(value: &'a mut Vec<T>) -> Self {
        Self::from(value.as_mut_slice())
    }
}

impl<'a, T> From<&'a CArrayPtrMut<T>> for CArrayPtrRef<'a, T> {
    fn from(value: &'a CArrayPtrMut<T>) -> Self {
        Self {
            ptr: value.as_non_null(),
            _marker: PhantomData,
        }
    }
}

impl<'a, T> From<CArrayPtrMutRef<'a, T>> for CArrayPtrRef<'a, T> {
    fn from(value: CArrayPtrMutRef<'a, T>) -> Self {
        Self {
            ptr: value.as_non_null(),
            _marker: PhantomData,
        }
    }
}

impl<T> From<&CArrayPtrMut<T>> for CArrayPtr<T> {
    fn from(value: &CArrayPtrMut<T>) -> Self {
        Self(value.as_non_null())
    }
}

/// Transparent wrapper around a non-null C mut array pointer to indicate that this represents an array pointer.
/// This is guaranteed to be the size of a pointer.
/// This wrapper implies no ownership, no memory will be freed upon drop.
/// The interior pointer can never be null, so when used as part of FFI interop
/// it should be used as `Option<CArrayPtrMut<T>>` when non-null values cannot be guaranteed.
/// `Option<CArrayPtrMut<T>>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CArrayPtrMut<T>(NonNull<T>);

unsafe impl<T: Send> Send for CArrayPtrMut<T> {}
unsafe impl<T: Sync> Sync for CArrayPtrMut<T> {}

impl<T> CArrayPtrMut<T> {
    /// Creates a `CArrayPtrMut` from a raw pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, properly aligned, and the memory it points to must remain
    /// valid for the intended lifetime of this `CArrayPtrMut`.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Creates a `CArrayPtrMut` from a [`NonNull`] pointer.
    ///
    /// Construction itself is safe; the methods that read through the wrapper are
    /// `unsafe` and require the pointer to be valid for the intended use.
    pub fn from_non_null(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }

    /// Returns the underlying [`NonNull`] pointer.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.0
    }

    /// Consumes the wrapper, returning the underlying [`NonNull`] pointer.
    pub fn into_non_null(self) -> NonNull<T> {
        self.0
    }

    /// Forms a shared slice from the pointer and a length.
    ///
    /// Returns an empty slice if `len` is zero.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for the lifetime of the returned slice.
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        unsafe {
            if len > 0 {
                slice::from_raw_parts(self.0.as_ptr(), len)
            } else {
                &[]
            }
        }
    }

    /// Forms a mutable slice from the pointer and a length.
    ///
    /// Returns an empty slice if `len` is zero.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for reads and writes of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid and exclusively accessible for the
    /// lifetime of the returned slice.
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        unsafe {
            if len > 0 {
                slice::from_raw_parts_mut(self.0.as_ptr(), len)
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
        self.0.as_ptr()
    }

    /// Returns the underlying pointer as a raw mut pointer.
    ///
    /// The caller must ensure that the array outlives the pointer this function returns,
    /// or else it will end up dangling.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.0.as_ptr()
    }
}

impl<T: Debug> Debug for CArrayPtrMut<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CArrayPtrMut").field(&self.0).finish()
    }
}

impl<T> From<CArrayPtrMut<T>> for *const T {
    fn from(value: CArrayPtrMut<T>) -> Self {
        value.as_ptr()
    }
}

impl<T> From<CArrayPtrMut<T>> for CArrayPtr<T> {
    fn from(value: CArrayPtrMut<T>) -> Self {
        Self(value.into_non_null())
    }
}

/// Transparent wrapper around a non-null C mut array pointer to indicate that this represents an array pointer used as a reference.
/// This is guaranteed to be the size of a pointer.
/// This wrapper implies no ownership, no memory will be freed upon drop.
/// The interior pointer can never be null, so when used as part of FFI interop
/// it should be used as `Option<CArrayPtrMutRef<T>>` when non-null values cannot be guaranteed.
/// `Option<CArrayPtrMutRef<T>>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CArrayPtrMutRef<'a, T> {
    ptr: NonNull<T>,
    _marker: PhantomData<&'a mut T>,
}

unsafe impl<T: Send> Send for CArrayPtrMutRef<'_, T> {}
unsafe impl<T: Sync> Sync for CArrayPtrMutRef<'_, T> {}

impl<'a, T> CArrayPtrMutRef<'a, T> {
    /// Creates a `CArrayPtrMutRef` from a raw pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, properly aligned, and the memory it points to must remain
    /// valid and exclusively accessible for the lifetime `'a`.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            _marker: PhantomData,
        }
    }

    /// Creates a `CArrayPtrMutRef` from a [`NonNull`] pointer.
    ///
    /// Construction itself is safe; the methods that read or write through the wrapper
    /// are `unsafe` and require the pointer to be valid and exclusively accessible for `'a`.
    pub fn from_non_null(ptr: NonNull<T>) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Returns the underlying [`NonNull`] pointer.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.ptr
    }

    /// Forms a shared slice from the pointer and a length.
    ///
    /// Returns an empty slice if `len` is zero.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for reads of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid for the lifetime of the returned slice.
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        unsafe {
            if len > 0 {
                slice::from_raw_parts(self.ptr.as_ptr(), len)
            } else {
                &[]
            }
        }
    }

    /// Forms a mutable slice from the pointer and a length.
    ///
    /// Returns an empty slice if `len` is zero.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for reads and writes of `len * size_of::<T>()` bytes,
    /// properly aligned, and the memory must remain valid and exclusively accessible for the
    /// lifetime of the returned slice.
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        unsafe {
            if len > 0 {
                slice::from_raw_parts_mut(self.ptr.as_ptr(), len)
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
        self.ptr.as_ptr()
    }

    /// Returns the underlying pointer as a raw mut pointer.
    ///
    /// The caller must ensure that the array outlives the pointer this function returns,
    /// or else it will end up dangling.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T: Debug> Debug for CArrayPtrMutRef<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CArrayPtrMutRef").field(&self.ptr).finish()
    }
}

impl<T> From<CArrayPtrMutRef<'_, T>> for *mut T {
    fn from(value: CArrayPtrMutRef<'_, T>) -> Self {
        value.ptr.as_ptr()
    }
}

impl<'a, T> From<&'a mut [T]> for CArrayPtrMutRef<'a, T> {
    fn from(value: &'a mut [T]) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(value.as_mut_ptr()) },
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, T> From<&'a mut Vec<T>> for CArrayPtrMutRef<'a, T> {
    fn from(value: &'a mut Vec<T>) -> Self {
        Self::from(value.as_mut_slice())
    }
}

impl<'a, T> From<&'a mut CArrayPtrMut<T>> for CArrayPtrMutRef<'a, T> {
    fn from(value: &'a mut CArrayPtrMut<T>) -> Self {
        Self {
            ptr: value.0,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
    use super::*;
    #[test]
    fn test_are_ptr_sized() {
        assert_eq!(
            core::mem::size_of::<CArrayPtr<i32>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<CArrayPtrMut<i32>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<CArrayPtrRef<i32>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<CArrayPtrMutRef<i32>>(),
            core::mem::size_of::<usize>()
        );
    }

    #[test]
    fn test_option_are_ptr_sized() {
        assert_eq!(
            core::mem::size_of::<Option<CArrayPtr<i32>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<CArrayPtrMut<i32>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<CArrayPtrRef<i32>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<CArrayPtrMutRef<i32>>>(),
            core::mem::size_of::<usize>()
        );
    }
}
