use core::borrow::Borrow;
use core::cmp::Ordering;
use core::convert::Infallible;
use core::ffi::{CStr, c_char};
use core::fmt;
use core::fmt::{Debug, Display, Formatter};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::num::NonZeroUsize;
use core::ops::Deref;
use core::ptr::{NonNull, null};
use core::slice;
use core::str::FromStr;

#[cfg(feature = "alloc")]
use {
    alloc::borrow::Cow,
    alloc::boxed::Box,
    alloc::ffi::{CString, FromVecWithNulError, IntoStringError, NulError},
    alloc::string::String,
    alloc::vec::Vec,
};

#[cfg(feature = "serde")]
use {
    serde::de::Visitor,
    serde::{Deserialize, Deserializer, Serialize, Serializer, de},
};

/// An owned null-terminated C-compatible UTF8 string, for use in FFI interop. This is guaranteed to be the size of a pointer.
/// Similar to CString the interior pointer can never be null, so when used as part of FFI interop it should be used as
/// `Option<CStringPtr>` when non-null values cannot be guaranteed. `Option<CStringPtr>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CStringPtr(NonNull<c_char>);

unsafe impl Send for CStringPtr {}
unsafe impl Sync for CStringPtr {}

impl CStringPtr {
    /// Creates a new C-compatible string from a container of bytes.
    ///
    /// This function will consume the provided data and use the underlying bytes to
    /// construct a new string, ensuring that there is a trailing 0 byte. This trailing
    /// 0 byte will be appended by this function; the provided data should *not*
    /// contain any 0 bytes in it.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes contain an interior null byte (`\0`).
    #[cfg(feature = "alloc")]
    pub fn new<T: Into<Vec<u8>>>(t: T) -> Result<Self, NulError> {
        match CString::new(t) {
            Ok(s) => Ok(Self(unsafe { NonNull::new_unchecked(s.into_raw()) })),
            Err(e) => Err(e),
        }
    }

    /// Creates a `CStringPtr` from a byte vector without checking for interior null bytes.
    ///
    /// A trailing null byte will be appended if not already present.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the byte vector does not contain any interior
    /// null bytes. Providing a vector with interior nulls produces undefined behavior
    /// when the string is later passed to C code.
    #[cfg(feature = "alloc")]
    pub unsafe fn from_vec_unchecked(v: Vec<u8>) -> Self {
        unsafe {
            Self(NonNull::new_unchecked(
                CString::from_vec_unchecked(v).into_raw(),
            ))
        }
    }

    /// Converts the `CStringPtr` into a [`String`] if the contents are valid UTF-8.
    ///
    /// This method consumes `self` and transfers ownership of the string to the returned
    /// [`String`]. The trailing null byte is not included in the returned string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string contains bytes that are not valid UTF-8.
    #[cfg(feature = "alloc")]
    pub fn into_string(self) -> Result<String, IntoStringError> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_string() }
    }

    /// Converts the `CStringPtr` into a byte vector, excluding the trailing null byte.
    ///
    /// This method consumes `self`.
    #[cfg(feature = "alloc")]
    pub fn into_bytes(self) -> Vec<u8> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_bytes() }
    }

    /// Converts the `CStringPtr` into a byte vector, including the trailing null byte.
    ///
    /// This method consumes `self`.
    #[cfg(feature = "alloc")]
    pub fn into_bytes_with_nul(self) -> Vec<u8> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_bytes_with_nul() }
    }

    /// Returns the contents of this `CStringPtr` as a byte slice, without the trailing
    /// null byte.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.0.as_ref()).to_bytes() }
    }

    /// Returns the contents of this `CStringPtr` as a byte slice, **including** the
    /// trailing null byte.
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.0.as_ref()).to_bytes_with_nul() }
    }

    /// Borrows the contents of this `CStringPtr` as a [`CStr`].
    pub fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.0.as_ref()) }
    }

    /// Converts this `CStringPtr` into a boxed [`CStr`], consuming `self`.
    #[cfg(feature = "alloc")]
    pub fn into_boxed_c_str(self) -> Box<CStr> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_boxed_c_str() }
    }

    /// Creates a `CStringPtr` from a byte vector that already ends with a null byte,
    /// without checking that the null byte is the only one.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - The last byte of `v` is `\0`.
    /// - There are no other `\0` bytes in `v`.
    #[cfg(feature = "alloc")]
    pub unsafe fn from_vec_with_nul_unchecked(v: Vec<u8>) -> Self {
        unsafe {
            Self(NonNull::new_unchecked(
                CString::from_vec_with_nul_unchecked(v).into_raw(),
            ))
        }
    }

    /// Creates a `CStringPtr` from a byte vector that already ends with a null byte.
    ///
    /// Unlike [`CStringPtr::new`], the provided vector must already contain a trailing
    /// null byte; one will not be appended automatically.
    ///
    /// # Errors
    ///
    /// Returns an error if the vector does not end with `\0`, or if there is more than
    /// one null byte (i.e. an interior null is present).
    #[cfg(feature = "alloc")]
    pub fn from_vec_with_null(v: Vec<u8>) -> Result<Self, FromVecWithNulError> {
        match CString::from_vec_with_nul(v) {
            Ok(s) => Ok(CStringPtr(unsafe { NonNull::new_unchecked(s.into_raw()) })),
            Err(e) => Err(e),
        }
    }
}

#[cfg(feature = "alloc")]
impl Drop for CStringPtr {
    fn drop(&mut self) {
        let _ = unsafe { CString::from_raw(self.0.as_ptr()) };
    }
}

impl AsRef<CStr> for CStringPtr {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Borrow<CStr> for CStringPtr {
    fn borrow(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Deref for CStringPtr {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.as_c_str()
    }
}

impl Debug for CStringPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for CStringPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", unsafe {
            CStr::from_ptr(self.0.as_ref()).to_string_lossy()
        })
    }
}

#[cfg(feature = "alloc")]
impl Clone for CStringPtr {
    fn clone(&self) -> Self {
        unsafe { Self::new(CStr::from_ptr(self.0.as_ref()).to_bytes()).unwrap() }
    }
}

#[cfg(feature = "alloc")]
impl Default for CStringPtr {
    fn default() -> Self {
        Self(unsafe { NonNull::new_unchecked(CString::default().into_raw()) })
    }
}

impl PartialEq<Self> for CStringPtr {
    fn eq(&self, other: &Self) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl Eq for CStringPtr {}

impl Hash for CStringPtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_c_str().hash(state);
    }
}

impl PartialOrd for CStringPtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CStringPtr {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_c_str().cmp(other.as_c_str())
    }
}

#[cfg(feature = "alloc")]
impl From<&CStr> for CStringPtr {
    fn from(value: &CStr) -> Self {
        unsafe {
            Self(NonNull::new_unchecked(
                CString::from_vec_unchecked(value.to_bytes().to_vec()).into_raw(),
            ))
        }
    }
}

#[cfg(feature = "alloc")]
impl From<CString> for CStringPtr {
    fn from(value: CString) -> Self {
        Self(unsafe { NonNull::new_unchecked(value.into_raw()) })
    }
}

#[cfg(feature = "alloc")]
impl From<&CString> for CStringPtr {
    fn from(value: &CString) -> Self {
        Self(unsafe { NonNull::new_unchecked(value.clone().into_raw()) })
    }
}

#[cfg(feature = "alloc")]
impl From<CStringPtr> for CString {
    fn from(value: CStringPtr) -> Self {
        let ptr = value.0.as_ptr();
        core::mem::forget(value);
        unsafe { CString::from_raw(ptr) }
    }
}

#[cfg(feature = "serde")]
impl Serialize for CStringPtr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_c_str().to_string_lossy().as_ref())
    }
}

#[cfg(feature = "serde")]
#[cfg(feature = "alloc")]
impl<'de> Deserialize<'de> for CStringPtr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CStringPtrVisitor;

        impl<'de> Visitor<'de> for CStringPtrVisitor {
            type Value = CStringPtr;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("a UTF-8 string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CString::new(v).map_err(E::custom)?.into())
            }
        }

        deserializer.deserialize_str(CStringPtrVisitor)
    }
}

/// A borrowed null-terminated C-compatible UTF8 string, for use in FFI interop. This is guaranteed to be the size of a pointer.
/// Unlike [`CStringPtr`], this does not own the string data and will not free it on drop.
/// Similar to `CStringPtr` the interior pointer can never be null, so when used as part of FFI interop it should be used as
/// `Option<CStringPtrRef>` when non-null values cannot be guaranteed. `Option<CStringPtrRef>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CStringPtrRef<'a> {
    ptr: NonNull<c_char>,
    _marker: PhantomData<&'a CStr>,
}

unsafe impl Send for CStringPtrRef<'_> {}
unsafe impl Sync for CStringPtrRef<'_> {}

impl<'a> CStringPtrRef<'a> {
    /// Creates a `CStringPtrRef` from a raw non-null pointer.
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid, null-terminated C string that remains valid
    /// for the lifetime `'a`.
    pub unsafe fn from_ptr(ptr: NonNull<c_char>) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Returns the contents of this `CStringPtrRef` as a byte slice, without the trailing
    /// null byte.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.ptr.as_ref()).to_bytes() }
    }

    /// Returns the contents of this `CStringPtrRef` as a byte slice, **including** the
    /// trailing null byte.
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.ptr.as_ref()).to_bytes_with_nul() }
    }

    /// Borrows the contents of this `CStringPtrRef` as a [`CStr`].
    pub fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.ptr.as_ref()) }
    }
}

impl Copy for CStringPtrRef<'_> {}

impl Clone for CStringPtrRef<'_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl AsRef<CStr> for CStringPtrRef<'_> {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Borrow<CStr> for CStringPtrRef<'_> {
    fn borrow(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Deref for CStringPtrRef<'_> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.as_c_str()
    }
}

impl Debug for CStringPtrRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for CStringPtrRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", unsafe {
            CStr::from_ptr(self.ptr.as_ref()).to_string_lossy()
        })
    }
}

impl PartialEq<Self> for CStringPtrRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl Eq for CStringPtrRef<'_> {}

impl Hash for CStringPtrRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_c_str().hash(state);
    }
}

impl PartialOrd for CStringPtrRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CStringPtrRef<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_c_str().cmp(other.as_c_str())
    }
}

impl<'a> From<&'a CStringPtr> for CStringPtrRef<'a> {
    fn from(value: &'a CStringPtr) -> Self {
        Self {
            ptr: value.0,
            _marker: PhantomData,
        }
    }
}

impl<'a> From<&'a CStr> for CStringPtrRef<'a> {
    fn from(value: &'a CStr) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(value.as_ptr() as *mut c_char) },
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<&'a CString> for CStringPtrRef<'a> {
    fn from(value: &'a CString) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(value.as_ptr() as *mut c_char) },
            _marker: PhantomData,
        }
    }
}

/// An owned null-terminated mutable C-compatible UTF8 string, for use in FFI interop. This is guaranteed to be the size of a pointer.
/// Similar to [`CStringPtr`] but wraps a mutable pointer. The interior pointer can never be null,
/// so when used as part of FFI interop it should be used as `Option<CStringPtrMut>` when non-null
/// values cannot be guaranteed. `Option<CStringPtrMut>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CStringPtrMut(NonNull<c_char>);

unsafe impl Send for CStringPtrMut {}
unsafe impl Sync for CStringPtrMut {}

impl CStringPtrMut {
    /// Creates a new C-compatible string from a container of bytes.
    ///
    /// This function will consume the provided data and use the underlying bytes to
    /// construct a new string, ensuring that there is a trailing 0 byte. This trailing
    /// 0 byte will be appended by this function; the provided data should *not*
    /// contain any 0 bytes in it.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes contain an interior null byte (`\0`).
    #[cfg(feature = "alloc")]
    pub fn new<T: Into<Vec<u8>>>(t: T) -> Result<Self, NulError> {
        match CString::new(t) {
            Ok(s) => Ok(Self(unsafe { NonNull::new_unchecked(s.into_raw()) })),
            Err(e) => Err(e),
        }
    }

    /// Creates a `CStringPtrMut` from a byte vector without checking for interior null bytes.
    ///
    /// A trailing null byte will be appended if not already present.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the byte vector does not contain any interior
    /// null bytes. Providing a vector with interior nulls produces undefined behavior
    /// when the string is later passed to C code.
    #[cfg(feature = "alloc")]
    pub unsafe fn from_vec_unchecked(v: Vec<u8>) -> Self {
        unsafe {
            Self(NonNull::new_unchecked(
                CString::from_vec_unchecked(v).into_raw(),
            ))
        }
    }

    /// Converts the `CStringPtrMut` into a [`String`] if the contents are valid UTF-8.
    ///
    /// This method consumes `self` and transfers ownership of the string to the returned
    /// [`String`]. The trailing null byte is not included in the returned string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string contains bytes that are not valid UTF-8.
    #[cfg(feature = "alloc")]
    pub fn into_string(self) -> Result<String, IntoStringError> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_string() }
    }

    /// Converts the `CStringPtrMut` into a byte vector, excluding the trailing null byte.
    ///
    /// This method consumes `self`.
    #[cfg(feature = "alloc")]
    pub fn into_bytes(self) -> Vec<u8> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_bytes() }
    }

    /// Converts the `CStringPtrMut` into a byte vector, including the trailing null byte.
    ///
    /// This method consumes `self`.
    #[cfg(feature = "alloc")]
    pub fn into_bytes_with_nul(self) -> Vec<u8> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_bytes_with_nul() }
    }

    /// Returns the contents of this `CStringPtrMut` as a byte slice, without the trailing
    /// null byte.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.0.as_ref()).to_bytes() }
    }

    /// Returns the contents of this `CStringPtrMut` as a byte slice, **including** the
    /// trailing null byte.
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.0.as_ref()).to_bytes_with_nul() }
    }

    /// Borrows the contents of this `CStringPtrMut` as a [`CStr`].
    pub fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.0.as_ref()) }
    }

    /// Returns the raw mutable pointer.
    ///
    /// The caller must ensure that the string outlives the pointer this function returns,
    /// or else it will end up dangling.
    pub fn as_mut_ptr(&mut self) -> *mut c_char {
        self.0.as_ptr()
    }

    /// Converts this `CStringPtrMut` into a boxed [`CStr`], consuming `self`.
    #[cfg(feature = "alloc")]
    pub fn into_boxed_c_str(self) -> Box<CStr> {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        unsafe { CString::from_raw(ptr).into_boxed_c_str() }
    }

    /// Creates a `CStringPtrMut` from a byte vector that already ends with a null byte,
    /// without checking that the null byte is the only one.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - The last byte of `v` is `\0`.
    /// - There are no other `\0` bytes in `v`.
    #[cfg(feature = "alloc")]
    pub unsafe fn from_vec_with_nul_unchecked(v: Vec<u8>) -> Self {
        unsafe {
            Self(NonNull::new_unchecked(
                CString::from_vec_with_nul_unchecked(v).into_raw(),
            ))
        }
    }

    /// Creates a `CStringPtrMut` from a byte vector that already ends with a null byte.
    ///
    /// Unlike [`CStringPtrMut::new`], the provided vector must already contain a trailing
    /// null byte; one will not be appended automatically.
    ///
    /// # Errors
    ///
    /// Returns an error if the vector does not end with `\0`, or if there is more than
    /// one null byte (i.e. an interior null is present).
    #[cfg(feature = "alloc")]
    pub fn from_vec_with_null(v: Vec<u8>) -> Result<Self, FromVecWithNulError> {
        match CString::from_vec_with_nul(v) {
            Ok(s) => Ok(Self(unsafe { NonNull::new_unchecked(s.into_raw()) })),
            Err(e) => Err(e),
        }
    }
}

#[cfg(feature = "alloc")]
impl Drop for CStringPtrMut {
    fn drop(&mut self) {
        let _ = unsafe { CString::from_raw(self.0.as_ptr()) };
    }
}

impl AsRef<CStr> for CStringPtrMut {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Borrow<CStr> for CStringPtrMut {
    fn borrow(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Deref for CStringPtrMut {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.as_c_str()
    }
}

impl Debug for CStringPtrMut {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for CStringPtrMut {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", unsafe {
            CStr::from_ptr(self.0.as_ref()).to_string_lossy()
        })
    }
}

#[cfg(feature = "alloc")]
impl Clone for CStringPtrMut {
    fn clone(&self) -> Self {
        unsafe { Self::new(CStr::from_ptr(self.0.as_ref()).to_bytes()).unwrap() }
    }
}

#[cfg(feature = "alloc")]
impl Default for CStringPtrMut {
    fn default() -> Self {
        Self(unsafe { NonNull::new_unchecked(CString::default().into_raw()) })
    }
}

impl PartialEq<Self> for CStringPtrMut {
    fn eq(&self, other: &Self) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl Eq for CStringPtrMut {}

impl Hash for CStringPtrMut {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_c_str().hash(state);
    }
}

impl PartialOrd for CStringPtrMut {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CStringPtrMut {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_c_str().cmp(other.as_c_str())
    }
}

#[cfg(feature = "alloc")]
impl From<&CStr> for CStringPtrMut {
    fn from(value: &CStr) -> Self {
        unsafe {
            Self(NonNull::new_unchecked(
                CString::from_vec_unchecked(value.to_bytes().to_vec()).into_raw(),
            ))
        }
    }
}

#[cfg(feature = "alloc")]
impl From<CString> for CStringPtrMut {
    fn from(value: CString) -> Self {
        Self(unsafe { NonNull::new_unchecked(value.into_raw()) })
    }
}

#[cfg(feature = "alloc")]
impl From<&CString> for CStringPtrMut {
    fn from(value: &CString) -> Self {
        Self(unsafe { NonNull::new_unchecked(value.clone().into_raw()) })
    }
}

#[cfg(feature = "alloc")]
impl From<CStringPtrMut> for CString {
    fn from(value: CStringPtrMut) -> Self {
        let ptr = value.0.as_ptr();
        core::mem::forget(value);
        unsafe { CString::from_raw(ptr) }
    }
}

impl<'a> From<&'a CStringPtrMut> for CStringPtrRef<'a> {
    fn from(value: &'a CStringPtrMut) -> Self {
        Self {
            ptr: value.0,
            _marker: PhantomData,
        }
    }
}

impl<'a> From<&'a CStringPtrMut> for CStringPtrMutRef<'a> {
    fn from(value: &'a CStringPtrMut) -> Self {
        Self {
            ptr: value.0,
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for CStringPtrMut {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_c_str().to_string_lossy().as_ref())
    }
}

#[cfg(feature = "serde")]
#[cfg(feature = "alloc")]
impl<'de> Deserialize<'de> for CStringPtrMut {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CStringPtrMutVisitor;

        impl<'de> Visitor<'de> for CStringPtrMutVisitor {
            type Value = CStringPtrMut;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("a UTF-8 string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CString::new(v).map_err(E::custom)?.into())
            }
        }

        deserializer.deserialize_str(CStringPtrMutVisitor)
    }
}

/// A borrowed null-terminated mutable C-compatible UTF8 string, for use in FFI interop. This is guaranteed to be the size of a pointer.
/// Unlike [`CStringPtrMut`], this does not own the string data and will not free it on drop.
/// Similar to `CStringPtrMut` the interior pointer can never be null, so when used as part of FFI interop it should be used as
/// `Option<CStringPtrMutRef>` when non-null values cannot be guaranteed. `Option<CStringPtrMutRef>` is also guaranteed to be the size of a pointer.
#[repr(transparent)]
pub struct CStringPtrMutRef<'a> {
    ptr: NonNull<c_char>,
    _marker: PhantomData<&'a mut CStr>,
}

unsafe impl Send for CStringPtrMutRef<'_> {}
unsafe impl Sync for CStringPtrMutRef<'_> {}

impl<'a> CStringPtrMutRef<'a> {
    /// Creates a `CStringPtrMutRef` from a raw non-null pointer.
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid, null-terminated C string that remains valid
    /// and exclusively accessible for the lifetime `'a`.
    pub unsafe fn from_ptr(ptr: NonNull<c_char>) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Returns the contents of this `CStringPtrMutRef` as a byte slice, without the trailing
    /// null byte.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.ptr.as_ref()).to_bytes() }
    }

    /// Returns the contents of this `CStringPtrMutRef` as a byte slice, **including** the
    /// trailing null byte.
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.ptr.as_ref()).to_bytes_with_nul() }
    }

    /// Borrows the contents of this `CStringPtrMutRef` as a [`CStr`].
    pub fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.ptr.as_ref()) }
    }

    /// Returns the raw mutable pointer.
    ///
    /// The caller must ensure that the string outlives the pointer this function returns,
    /// or else it will end up dangling.
    pub fn as_mut_ptr(&mut self) -> *mut c_char {
        self.ptr.as_ptr()
    }
}

impl AsRef<CStr> for CStringPtrMutRef<'_> {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Borrow<CStr> for CStringPtrMutRef<'_> {
    fn borrow(&self) -> &CStr {
        self.as_c_str()
    }
}

impl Deref for CStringPtrMutRef<'_> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.as_c_str()
    }
}

impl Debug for CStringPtrMutRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for CStringPtrMutRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", unsafe {
            CStr::from_ptr(self.ptr.as_ref()).to_string_lossy()
        })
    }
}

impl PartialEq<Self> for CStringPtrMutRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl Eq for CStringPtrMutRef<'_> {}

impl Hash for CStringPtrMutRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_c_str().hash(state);
    }
}

impl PartialOrd for CStringPtrMutRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CStringPtrMutRef<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_c_str().cmp(other.as_c_str())
    }
}

impl<'a> From<CStringPtrMutRef<'a>> for CStringPtrRef<'a> {
    fn from(value: CStringPtrMutRef<'a>) -> Self {
        Self {
            ptr: value.ptr,
            _marker: PhantomData,
        }
    }
}

impl<'a> From<&'a CStr> for CStringPtrMutRef<'a> {
    fn from(value: &'a CStr) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(value.as_ptr() as *mut c_char) },
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<&'a CString> for CStringPtrMutRef<'a> {
    fn from(value: &'a CString) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(value.as_ptr() as *mut c_char) },
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
    use super::*;
    use alloc::string::ToString;

    // --- CStringPtr layout ---

    #[test]
    fn cstring_ptr_is_pointer_sized() {
        assert_eq!(size_of::<CStringPtr>(), size_of::<*mut c_char>());
    }

    #[test]
    fn cstring_ptr_option_is_pointer_sized() {
        assert_eq!(size_of::<Option<CStringPtr>>(), size_of::<*mut c_char>());
    }

    // --- CStringPtr construction ---

    #[test]
    fn cstring_ptr_new_valid() {
        let s = CStringPtr::new("hello").unwrap();
        assert_eq!(s.as_bytes(), b"hello");
    }

    #[test]
    fn cstring_ptr_new_interior_null_fails() {
        assert!(CStringPtr::new("hel\0lo").is_err());
    }

    #[test]
    fn cstring_ptr_default_is_empty() {
        let s = CStringPtr::default();
        assert_eq!(s.as_bytes(), b"");
    }

    #[test]
    fn cstring_ptr_from_cstring() {
        let cs = CString::new("world").unwrap();
        let s = CStringPtr::from(cs);
        assert_eq!(s.as_bytes(), b"world");
    }

    #[test]
    fn cstring_ptr_from_cstr() {
        let cs = CString::new("world").unwrap();
        let s = CStringPtr::from(cs.as_c_str());
        assert_eq!(s.as_bytes(), b"world");
    }

    // --- CStringPtr byte and string access ---

    #[test]
    fn cstring_ptr_as_bytes_excludes_nul() {
        let s = CStringPtr::new("hi").unwrap();
        assert_eq!(s.as_bytes(), b"hi");
    }

    #[test]
    fn cstring_ptr_as_bytes_with_nul_includes_nul() {
        let s = CStringPtr::new("hi").unwrap();
        assert_eq!(s.as_bytes_with_nul(), b"hi\0");
    }

    #[test]
    fn cstring_ptr_as_c_str() {
        let s = CStringPtr::new("test").unwrap();
        assert_eq!(s.as_c_str().to_bytes(), b"test");
    }

    // --- CStringPtr consuming conversions ---

    #[test]
    fn cstring_ptr_into_string() {
        let s = CStringPtr::new("hello").unwrap();
        assert_eq!(s.into_string().unwrap(), "hello");
    }

    #[test]
    fn cstring_ptr_into_bytes() {
        let s = CStringPtr::new("abc").unwrap();
        assert_eq!(s.into_bytes(), b"abc");
    }

    #[test]
    fn cstring_ptr_into_bytes_with_nul() {
        let s = CStringPtr::new("abc").unwrap();
        assert_eq!(s.into_bytes_with_nul(), b"abc\0");
    }

    #[test]
    fn cstring_ptr_into_cstring() {
        let s = CStringPtr::new("hello").unwrap();
        let cs: CString = s.into();
        assert_eq!(cs.to_bytes(), b"hello");
    }

    #[test]
    fn cstring_ptr_into_boxed_c_str() {
        let s = CStringPtr::new("boxed").unwrap();
        let b = s.into_boxed_c_str();
        assert_eq!(b.to_bytes(), b"boxed");
    }

    // --- CStringPtr clone and equality ---

    #[test]
    fn cstring_ptr_clone_is_independent() {
        let a = CStringPtr::new("foo").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
        // Both can be independently dropped without double-free
        drop(a);
        drop(b);
    }

    #[test]
    fn cstring_ptr_eq() {
        let a = CStringPtr::new("foo").unwrap();
        let b = CStringPtr::new("foo").unwrap();
        let c = CStringPtr::new("bar").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn cstring_ptr_ord() {
        let a = CStringPtr::new("apple").unwrap();
        let b = CStringPtr::new("banana").unwrap();
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn cstring_ptr_display() {
        let s = CStringPtr::new("display").unwrap();
        assert_eq!(s.to_string(), "display");
    }

    // --- OptionStringBuffer construction ---

    #[test]
    fn option_string_buffer_none() {
        let s = CStringBuffer::new_none();
        assert!(s.is_none());
        assert!(!s.is_some());
        assert!(s.is_none_or_empty());
        assert_eq!(s.as_str(), None);
        assert_eq!(s.as_bytes(), None);
    }

    #[test]
    fn option_string_buffer_empty() {
        let s = CStringBuffer::new_empty();
        assert!(s.is_some());
        assert!(!s.is_none());
        assert!(s.is_none_or_empty());
        assert_eq!(s.as_str(), Some(""));
    }

    #[test]
    fn option_string_buffer_default_is_none() {
        let s = CStringBuffer::default();
        assert!(s.is_none());
    }

    #[test]
    fn option_string_buffer_from_str() {
        let s = CStringBuffer::from("hello");
        assert_eq!(s.as_str(), Some("hello"));
        assert!(!s.is_none_or_empty());
    }

    #[test]
    fn option_string_buffer_from_string() {
        let s = CStringBuffer::from("world".to_string());
        assert_eq!(s.as_str(), Some("world"));
    }

    // --- OptionStringBuffer byte access ---

    #[test]
    fn option_string_buffer_as_bytes_includes_nul() {
        let s = CStringBuffer::from("hi");
        let bytes = s.as_bytes().unwrap();
        assert_eq!(bytes, b"hi\0");
    }

    #[test]
    fn option_string_buffer_as_str_or_empty_on_none() {
        let s = CStringBuffer::new_none();
        assert_eq!(s.as_str_or_empty(), "");
    }

    // --- OptionStringBuffer equality ---

    #[test]
    fn option_string_buffer_eq() {
        let a = CStringBuffer::from("foo");
        let b = CStringBuffer::from("foo");
        let c = CStringBuffer::from("bar");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn option_string_buffer_eq_str() {
        let s = CStringBuffer::from("hello");
        assert_eq!(s, "hello");
        assert_ne!(s, "world");
    }

    #[test]
    fn option_string_buffer_none_ne_some() {
        let none = CStringBuffer::new_none();
        let some = CStringBuffer::from("x");
        assert_ne!(none, some);
    }

    // --- OptionStringBuffer clone ---

    #[test]
    fn option_string_buffer_clone_is_independent() {
        let a = CStringBuffer::from("clone me");
        let b = a.clone();
        assert_eq!(a, b);
        drop(a);
        // b should still be valid after a is dropped
        assert_eq!(b.as_str(), Some("clone me"));
    }

    #[test]
    fn option_string_buffer_none_clone() {
        let a = CStringBuffer::new_none();
        let b = a.clone();
        assert!(b.is_none());
    }

    // --- OptionStringBuffer as_option / StringBufferRef ---

    #[test]
    fn option_string_buffer_as_option_none() {
        let s = CStringBuffer::new_none();
        assert!(s.as_option().is_none());
    }

    #[test]
    fn option_string_buffer_as_option_some() {
        let s = CStringBuffer::from("ref me");
        let r = s.as_option().unwrap();
        assert_eq!(r.as_str(), "ref me");
        assert_eq!(&*r, "ref me");
    }

    #[test]
    fn string_buffer_ref_as_bytes_includes_nul() {
        let s = CStringBuffer::from("hi");
        let r = s.as_option().unwrap();
        assert_eq!(r.as_bytes(), b"hi\0");
    }

    #[test]
    fn string_buffer_ref_ord() {
        let a = CStringBuffer::from("apple");
        let b = CStringBuffer::from("banana");
        let ra = a.as_option().unwrap();
        let rb = b.as_option().unwrap();
        assert!(ra < rb);
    }

    // --- OptionStringBuffer into_string ---

    #[test]
    fn option_string_buffer_into_string_some() {
        let s = CStringBuffer::from("hello");
        assert_eq!(s.into_string(), Some("hello".to_string()));
    }

    #[test]
    fn option_string_buffer_into_string_none() {
        let s = CStringBuffer::new_none();
        assert_eq!(s.into_string(), None);
    }

    #[test]
    fn option_string_buffer_into_string_empty() {
        let s = CStringBuffer::new_empty();
        assert_eq!(s.into_string(), Some(String::new()));
    }
}

/// An owned null-terminated C-compatible UTF8 string buffer, for use in FFI interop.
/// Because the string pointer could be null, it is equivalent to `Option<String>`.
/// Unlike `CString` and `CStringPtr` interior-null bytes *are* permitted.
/// The buffer *must* end with a null byte to allow compatibility with C code expected null terminated strings.
#[repr(C)]
pub struct CStringBuffer {
    /// Pointer to null-terminated UTF-8 string array
    ptr: *const c_char,
    /// Length of string array in bytes including the null terminator
    len: usize,
}
unsafe impl Send for CStringBuffer {}
unsafe impl Sync for CStringBuffer {}

#[cfg(feature = "alloc")]
impl Drop for CStringBuffer {
    fn drop(&mut self) {
        let len = self.len;
        self.len = 0;
        let ptr = self.ptr as *mut u8;
        self.ptr = null();
        if !ptr.is_null() {
            let _ = unsafe { Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr, len)) };
        }
    }
}

impl CStringBuffer {
    /// Creates an `OptionStringBuffer` that holds an empty string (`""`).
    #[cfg(feature = "alloc")]
    pub fn new_empty() -> Self {
        Self {
            ptr: Box::into_raw(Box::new([b'\0'])) as *const c_char,
            len: 1,
        }
    }

    /// Creates an `OptionStringBuffer` representing the absent (`None`) state.
    pub fn new_none() -> Self {
        Self {
            ptr: null(),
            len: 0,
        }
    }

    /// Constructs an `OptionStringBuffer` from a raw pointer and byte length,
    /// taking ownership of the allocation.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `ptr` points to a valid heap allocation of exactly `len` bytes (or is null
    ///   with `len == 0`).
    /// - The allocation was originally created as a `Box<[u8]>` of length `len`.
    /// - The last byte of the allocation is `b'\0'`.
    /// - After this call the caller must not use or free the allocation; `Drop` will
    ///   handle deallocation.
    pub unsafe fn from_raw_parts(ptr: *const c_char, len: usize) -> Self {
        Self { ptr, len }
    }

    /// Returns `true` if the buffer holds a string value (including empty strings).
    pub fn is_some(&self) -> bool {
        self.len > 0
    }

    /// Returns `true` if the buffer is in the absent (`None`) state.
    pub fn is_none(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` if the buffer is absent (`None`) or holds an empty string (`""`).
    pub fn is_none_or_empty(&self) -> bool {
        self.len < 2
    }

    /// Decomposes the buffer into its raw parts (pointer and byte length), transferring
    /// ownership of the allocation to the caller.
    ///
    /// After this call `Drop` will **not** free the allocation; the caller is responsible
    /// for freeing it via [`CStringBuffer::from_raw_parts`] or equivalent.
    pub fn into_raw_parts(self) -> (*const c_char, usize) {
        let parts = (self.ptr, self.len);
        core::mem::forget(self);
        parts
    }

    /// Converts the buffer into an `Option<String>`, consuming `self`.
    ///
    /// Returns `None` if the buffer is absent, otherwise returns the string content
    /// (without the trailing null byte) as a `String`.
    #[cfg(feature = "alloc")]
    pub fn into_string(self) -> Option<String> {
        let (ptr, len) = self.into_raw_parts();
        if ptr.is_null() {
            None
        } else {
            // Reconstruct the owned allocation (len bytes including trailing null),
            // then strip the null before converting to String.
            let mut bytes =
                unsafe { Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len)) }
                    .into_vec();
            bytes.pop(); // remove trailing null
            Some(unsafe { String::from_utf8_unchecked(bytes) })
        }
    }

    /// Returns the raw bytes of the buffer as a slice, **including** the trailing null
    /// byte, or `None` if the buffer is absent.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self.ptr.is_null() || self.len == 0 {
            true => None,
            false => Some(unsafe { slice::from_raw_parts(self.ptr as *const u8, self.len) }),
        }
    }

    /// Borrows the buffer as an [`Option<CStringBufferRef>`], returning `None` if the
    /// buffer is absent.
    pub fn as_option(&self) -> Option<CStringBufferRef<'_>> {
        match self.len {
            0 => None,
            _ => Some(unsafe {
                CStringBufferRef {
                    _marker: Default::default(),
                    ptr: NonNull::new_unchecked(self.ptr as *mut c_char),
                    length_bytes: NonZeroUsize::new_unchecked(self.len),
                }
            }),
        }
    }

    /// Returns the string content as an `Option<&str>`, or `None` if the buffer is absent.
    ///
    /// The returned string does not include the trailing null byte.
    pub fn as_str(&self) -> Option<&str> {
        match self.len {
            0 => None,
            1 => Some(""),
            _ => Some(unsafe {
                str::from_utf8_unchecked(slice::from_raw_parts(self.ptr as *const u8, self.len - 1))
            }),
        }
    }

    /// Returns the string content as a `&str`, falling back to `""` if the buffer is
    /// absent.
    pub fn as_str_or_empty(&self) -> &str {
        self.as_str().unwrap_or("")
    }
}

#[cfg(feature = "alloc")]
impl From<&str> for CStringBuffer {
    fn from(value: &str) -> Self {
        let mut bytes = Vec::with_capacity(value.len() + 1);
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(b'\0');
        let length_bytes = bytes.len();
        let ptr = Box::into_raw(bytes.into_boxed_slice()) as *const c_char;
        Self {
            ptr,
            len: length_bytes,
        }
    }
}

#[cfg(feature = "alloc")]
impl From<String> for CStringBuffer {
    fn from(value: String) -> Self {
        let mut bytes = value.into_bytes();
        bytes.push(b'\0');
        let length_bytes = bytes.len();
        let ptr = Box::into_raw(bytes.into_boxed_slice()) as *const c_char;
        Self {
            ptr,
            len: length_bytes,
        }
    }
}

#[cfg(feature = "alloc")]
impl From<Option<String>> for CStringBuffer {
    fn from(value: Option<String>) -> Self {
        match value {
            None => CStringBuffer::new_none(),
            Some(s) => s.into(),
        }
    }
}

#[cfg(feature = "alloc")]
impl From<CStringBuffer> for Option<String> {
    fn from(value: CStringBuffer) -> Self {
        value.into_string()
    }
}

impl FromStr for CStringBuffer {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

#[cfg(feature = "alloc")]
impl Clone for CStringBuffer {
    fn clone(&self) -> Self {
        match self.as_bytes() {
            None => Self::new_none(),
            Some(bytes) => {
                let length_bytes = bytes.len();
                let ptr = Box::into_raw(bytes.to_vec().into_boxed_slice()) as *const c_char;
                Self {
                    ptr,
                    len: length_bytes,
                }
            }
        }
    }
}

impl Default for CStringBuffer {
    fn default() -> Self {
        Self::new_none()
    }
}

impl Debug for CStringBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_option().fmt(f)
    }
}

impl Display for CStringBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_option().fmt(f)
    }
}

impl PartialEq for CStringBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CStringBuffer {}

impl PartialEq<&str> for CStringBuffer {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(other)
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<String> for CStringBuffer {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other)
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<Cow<'_, str>> for CStringBuffer {
    fn eq(&self, other: &Cow<str>) -> bool {
        self.as_str() == Some(other.as_ref())
    }
}

impl PartialOrd<CStringBuffer> for CStringBuffer {
    fn partial_cmp(&self, other: &CStringBuffer) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CStringBuffer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(&other.as_str())
    }
}

impl Hash for CStringBuffer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(feature = "serde")]
impl Serialize for CStringBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
#[cfg(feature = "alloc")]
impl<'de> Deserialize<'de> for CStringBuffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(match opt {
            None => Self::new_none(),
            Some(s) => Self::from(s.as_str()),
        })
    }
}

/// An referenced null-terminated C-compatible UTF8 string buffer, for use in FFI interop.
/// Unlike `CString` and `CStringPtr` interior-null bytes *are* permitted.
/// The buffer is guaranteed to end with a null byte to allow compatibility with C code expected null terminated strings.
#[repr(C)]
pub struct CStringBufferRef<'a> {
    _marker: PhantomData<&'a [c_char]>,
    /// Non-null pointer to null-terminated UTF-8 string array
    ptr: NonNull<c_char>,
    /// Length of string array in bytes including the null terminator
    length_bytes: NonZeroUsize,
}

impl CStringBufferRef<'_> {
    /// Returns the raw bytes of the buffer as a slice, **including** the trailing null
    /// byte.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.ptr.as_ref() as *const c_char as *const u8,
                self.length_bytes.get(),
            )
        }
    }

    /// Returns the string content as a `&str`.
    ///
    /// The trailing null byte is not included in the returned string.
    pub fn as_str(&self) -> &str {
        unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(
                self.ptr.as_ref() as *const c_char as *const u8,
                self.length_bytes.get() - 1,
            ))
        }
    }
}

impl Debug for CStringBufferRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.as_str(), f)
    }
}

impl Display for CStringBufferRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.as_str(), f)
    }
}

impl PartialEq for CStringBufferRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CStringBufferRef<'_> {}

impl PartialOrd for CStringBufferRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CStringBufferRef<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for CStringBufferRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl Deref for CStringBufferRef<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<'a> From<&'a CStringBuffer> for Option<CStringBufferRef<'a>> {
    fn from(value: &'a CStringBuffer) -> Self {
        value.as_option()
    }
}
