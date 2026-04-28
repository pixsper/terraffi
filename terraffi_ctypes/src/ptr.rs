#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use core::ptr::NonNull;

#[cfg(feature = "alloc")]
use crate::PtrError;

pub type RefPtr<'a, T> = Option<&'a T>;

pub type MutRefPtr<'a, T> = Option<&'a mut T>;

pub type BoxPtr<T> = Option<Box<T>>;

/// A FFI-safe handle to a heap-allocated `T`.
///
/// Exports as `T* const*` in C headers. The outer pointer indicates whether the
/// handle itself is valid, and the inner pointer indicates whether a value is
/// currently held.
///
/// This type is pointer-sized: `Option<NonNull<Option<Box<T>>>>` where both
/// `Option` layers exploit niche optimisation.
///
/// # Ownership and lifetime
///
/// `CHandle` deliberately does **not** implement [`Drop`]. The handle's
/// allocation is owned by the C side of the FFI boundary and must be released
/// explicitly by the caller (typically via a dedicated `*_free` function that
/// calls [`CHandle::take`] and discards the result). Letting a `CHandle` go
/// out of scope on the Rust side leaks both the outer `Box<Option<Box<T>>>`
/// allocation and any value currently held — this is intentional, since the
/// C side may still hold a copy of the handle pointer.
#[cfg(feature = "alloc")]
#[repr(transparent)]
pub struct CHandle<T>(Option<NonNull<Option<Box<T>>>>);

#[cfg(feature = "alloc")]
impl<T> CHandle<T> {
    /// Creates an uninitialised handle (outer pointer is null).
    #[cfg(test)]
    fn new() -> Self {
        CHandle(None)
    }

    /// Initialises the handle, allocating the outer pointer with the inner
    /// pointer set to `None`.
    ///
    /// Fails with `UnexpectedNotNull` if the outer pointer is already set.
    #[cfg(test)]
    fn init(&mut self) -> Result<(), PtrError> {
        if self.0.is_some() {
            return Err(PtrError::HandleTargetNonNull);
        }
        let outer = Box::into_raw(Box::new(None::<Box<T>>));
        self.0 = Some(unsafe { NonNull::new_unchecked(outer) });
        Ok(())
    }

    /// Allocates a value into the handle.
    ///
    /// Fails with `UnexpectedNull` if the outer pointer is `None`, or with
    /// `UnexpectedNotNull` if the inner pointer already holds a value.
    pub fn alloc(&mut self, value: T) -> Result<(), PtrError> {
        match self.0 {
            None => Err(PtrError::NullHandle),
            Some(ptr) => {
                let inner = unsafe { &mut *ptr.as_ptr() };
                if inner.is_some() {
                    Err(PtrError::HandleTargetNonNull)
                } else {
                    *inner = Some(Box::new(value));
                    Ok(())
                }
            }
        }
    }

    /// Takes the value out of the handle, leaving the inner pointer as `None`.
    ///
    /// Fails with `UnexpectedNull` if the outer pointer is `None` or if the
    /// inner pointer does not hold a value.
    pub fn take(&mut self) -> Result<T, PtrError> {
        match self.0 {
            None => Err(PtrError::NullHandle),
            Some(ptr) => {
                let inner = unsafe { &mut *ptr.as_ptr() };
                match inner.take() {
                    Some(boxed) => Ok(*boxed),
                    None => Err(PtrError::HandleTargetNull),
                }
            }
        }
    }

    /// Returns `true` if the outer pointer is valid (the handle is initialised).
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// Returns `true` if the outer pointer is null (the handle is uninitialised).
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Returns a reference to the held value, or `None` if the outer or inner
    /// pointer is null.
    pub fn get(&self) -> Option<&T> {
        self.0.and_then(|ptr| unsafe { (*ptr.as_ptr()).as_deref() })
    }

    /// Returns a mutable reference to the held value, or `None` if the outer or
    /// inner pointer is null.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.0
            .and_then(|ptr| unsafe { (*ptr.as_ptr()).as_deref_mut() })
    }
}

#[cfg(feature = "alloc")]
impl<T: Default> CHandle<T> {
    pub fn alloc_default(&mut self) -> Result<(), PtrError> {
        self.alloc(T::default())
    }
}

// Manual Debug impl since we can't derive through the raw pointer.
#[cfg(feature = "alloc")]
impl<T: core::fmt::Debug> core::fmt::Debug for CHandle<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            None => f.write_str("CHandle(None)"),
            Some(ptr) => {
                let inner = unsafe { &*ptr.as_ptr() };
                f.debug_tuple("CHandle").field(inner).finish()
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
    use super::*;
    use core::mem;

    #[test]
    fn handle_is_pointer_sized() {
        assert_eq!(
            mem::size_of::<CHandle<u32>>(),
            mem::size_of::<*mut *mut u32>()
        );
    }

    #[test]
    fn default_handle_is_uninitialised() {
        let handle: CHandle<u32> = CHandle::new();
        assert!(handle.is_none());
    }

    #[test]
    fn default_handle_get_returns_none() {
        let handle: CHandle<u32> = CHandle::new();
        assert!(handle.get().is_none());
    }

    #[test]
    fn default_handle_get_mut_returns_none() {
        let mut handle: CHandle<u32> = CHandle::new();
        assert!(handle.get_mut().is_none());
    }

    #[test]
    fn default_handle_take_returns_err() {
        let mut handle: CHandle<u32> = CHandle::new();
        assert!(handle.take().is_err());
    }

    #[test]
    fn default_handle_alloc_returns_err() {
        let mut handle: CHandle<u32> = CHandle::new();
        assert!(handle.alloc(42).is_err());
    }

    #[test]
    fn init_then_is_some() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        assert!(handle.is_some());
    }

    #[test]
    fn double_init_returns_err() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        assert!(handle.init().is_err());
    }

    #[test]
    fn alloc_then_get_returns_value() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        assert_eq!(handle.get(), Some(&42));
    }

    #[test]
    fn alloc_then_get_mut_returns_value() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        assert_eq!(handle.get_mut(), Some(&mut 42));
    }

    #[test]
    fn get_mut_can_modify_value() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        *handle.get_mut().unwrap() = 99;
        assert_eq!(handle.get(), Some(&99));
    }

    #[test]
    fn take_returns_value_and_nullifies_inner() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        assert_eq!(handle.take(), Ok(42));
        assert!(handle.get().is_none());
        assert!(handle.is_some()); // outer pointer still valid
    }

    #[test]
    fn double_take_returns_err() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        assert!(handle.take().is_ok());
        assert!(handle.take().is_err());
    }

    #[test]
    fn double_alloc_returns_err() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        assert!(handle.alloc(99).is_err());
    }

    #[test]
    fn inner_null_get_returns_none() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        unsafe { *handle.0.unwrap().as_ptr() = None };
        assert!(handle.get().is_none());
    }

    #[test]
    fn inner_null_get_mut_returns_none() {
        let mut handle: CHandle<u32> = CHandle::new();
        handle.init().unwrap();
        handle.alloc(42).unwrap();
        unsafe { *handle.0.unwrap().as_ptr() = None };
        assert!(handle.get_mut().is_none());
    }
}
