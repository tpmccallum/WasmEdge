//! Defines WasmEdge Memory and MemType structs.
//!
//! A WasmEdge `Memory` defines a linear memory as described by `MemType`.
//! `MemType` specifies the limits on the size of a memory by a range. The start of
//! the limit range specifies min size (initial size) of that memory, while the end
//! restricts the size to which the memory can grow later.

use crate::{
    error::{check, MemError, WasmEdgeError},
    ffi, WasmEdgeResult,
};
use std::ops::RangeInclusive;

/// Struct of WasmEdge Memory.
///
/// A WasmEdge [Memory] defines a linear memory as described by [MemType].
#[derive(Debug)]
pub struct Memory {
    pub(crate) inner: InnerMemory,
    pub(crate) registered: bool,
}
impl Memory {
    /// Create a new [Memory] to be associated with the given capacity limit.
    ///
    /// # Arguments
    ///
    /// - `ty` specifies the type of the new [Memory] instance.
    ///
    /// # Errors
    ///
    /// If fail to create a [Memory], then an error is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use wasmedge_sys::{MemType, Memory};
    ///
    /// let ty = MemType::create(10..=20).expect("fail to create memory type");
    ///
    /// let memory = Memory::create(&ty);
    ///
    /// ```
    ///
    ///
    pub fn create(ty: &MemType) -> WasmEdgeResult<Self> {
        let ctx = unsafe { ffi::WasmEdge_MemoryInstanceCreate(ty.inner.0 as *const _) };

        match ctx.is_null() {
            true => Err(WasmEdgeError::Mem(MemError::Create)),
            false => Ok(Memory {
                inner: InnerMemory(ctx),
                registered: false,
            }),
        }
    }

    /// Returns the type of the [Memory].
    ///
    /// # Errors
    ///
    /// If fail to get the type from the [Memory], then an error is returned.
    ///
    pub fn ty(&self) -> WasmEdgeResult<MemType> {
        let ty_ctx = unsafe { ffi::WasmEdge_MemoryInstanceGetMemoryType(self.inner.0) };
        match ty_ctx.is_null() {
            true => Err(WasmEdgeError::Mem(MemError::Type)),
            false => Ok(MemType {
                inner: InnerMemType(ty_ctx as *mut _),
                registered: true,
            }),
        }
    }

    /// Copies the data from the [Memory] to the output buffer.
    ///
    /// # Arguments
    ///
    /// - `offset` specifies the data start offset in the [Memory].
    ///
    /// - `len` specifies the requested data length.
    ///
    /// # Errors
    ///
    /// If the `offset + len` is larger than the data size in the [Memory], then an error is returned.
    ///
    pub fn get_data(&self, offset: u32, len: u32) -> WasmEdgeResult<Vec<u8>> {
        let mut data = Vec::with_capacity(len as usize);
        unsafe {
            check(ffi::WasmEdge_MemoryInstanceGetData(
                self.inner.0,
                data.as_mut_ptr(),
                offset,
                len,
            ))?;
            data.set_len(len as usize);
        }

        Ok(data.into_iter().collect())
    }

    /// Copies the data from the given input buffer into the [Memory].
    ///
    /// # Arguments
    ///
    /// - `data` specifies the data buffer to copy.
    ///
    /// - `offset` specifies the data start offset in the [Memory].
    ///
    /// # Errors
    ///
    /// If the sum of the `offset` and the data length is larger than the size of the [Memory],
    /// then an error is returned.
    ///
    /// ```
    /// use wasmedge_sys::{error::{CoreError, CoreExecutionError, WasmEdgeError}, Memory, MemType};
    ///
    /// // create a Memory: the min size 1 and the max size 2
    /// let ty = MemType::create(1..=2).expect("fail to create a memory type");
    /// let mut mem = Memory::create(&ty).expect("fail to create a Memory");
    ///
    /// // set data and the data length is larger than the data size in the memory
    /// let result = mem.set_data(vec![1; 10], u32::pow(2, 16) - 9);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err(), WasmEdgeError::Core(CoreError::Execution(CoreExecutionError::MemoryOutOfBounds)));
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use wasmedge_sys::{MemType, Memory};
    ///
    /// // create a Memory: the min size 1 and the max size 2
    /// let ty = MemType::create(1..=2).expect("fail to create a memory type");
    /// let mut mem = Memory::create(&ty).expect("fail to create a Memory");
    /// // page count
    /// let count = mem.size();
    /// assert_eq!(count, 1);
    ///
    /// // set data
    /// mem.set_data(vec![1; 10], 10).expect("fail to set data");
    ///
    /// // get data
    /// let data = mem.get_data(10, 10).expect("fail to get data");
    /// assert_eq!(data, vec![1; 10]);
    /// ```
    ///
    pub fn set_data(
        &mut self,
        data: impl IntoIterator<Item = u8>,
        offset: u32,
    ) -> WasmEdgeResult<()> {
        let data = data.into_iter().collect::<Vec<u8>>();
        unsafe {
            check(ffi::WasmEdge_MemoryInstanceSetData(
                self.inner.0,
                data.as_ptr() as *mut _,
                offset,
                data.len() as u32,
            ))
        }
    }

    /// Returns the const data pointer to the [Memory].
    ///
    /// # Arguments
    ///
    /// - `offset` specifies the data start offset in the [Memory].
    ///
    /// - `len` specifies the requested data length. If the size of `offset` + `len` is larger
    /// than the data size in the [Memory]
    ///   
    ///
    /// # Errors
    ///
    /// If fail to get the data pointer, then an error is returned.
    ///
    pub fn data_pointer(&self, offset: u32, len: u32) -> WasmEdgeResult<&u8> {
        let ptr = unsafe { ffi::WasmEdge_MemoryInstanceGetPointerConst(self.inner.0, offset, len) };
        match ptr.is_null() {
            true => Err(WasmEdgeError::Mem(MemError::ConstPtr)),
            false => {
                let result = unsafe { ptr.as_ref() };
                match result {
                    Some(ptr) => Ok(ptr),
                    None => Err(WasmEdgeError::Mem(MemError::Ptr2Ref)),
                }
            }
        }
    }

    /// Returns the data pointer to the [Memory].
    ///
    /// # Arguments
    ///
    /// - `offset` specifies the data start offset in the [Memory].
    ///
    /// - `len` specifies the requested data length. If the size of `offset` + `len` is larger
    /// than the data size in the [Memory]
    ///
    /// # Errors
    ///
    /// If fail to get the data pointer, then an error is returned.
    ///
    pub fn data_pointer_mut(&mut self, offset: u32, len: u32) -> WasmEdgeResult<&mut u8> {
        let ptr = unsafe { ffi::WasmEdge_MemoryInstanceGetPointer(self.inner.0, offset, len) };
        match ptr.is_null() {
            true => Err(WasmEdgeError::Mem(MemError::MutPtr)),
            false => {
                let result = unsafe { ptr.as_mut() };
                match result {
                    Some(ptr) => Ok(ptr),
                    None => Err(WasmEdgeError::Mem(MemError::Ptr2Ref)),
                }
            }
        }
    }

    /// Returns the size, in WebAssembly pages (64 KiB of each page), of this wasm memory.
    pub fn size(&self) -> u32 {
        unsafe { ffi::WasmEdge_MemoryInstanceGetPageSize(self.inner.0) as u32 }
    }

    /// Grows this WebAssembly memory by `count` pages.
    ///
    /// # Arguments
    ///
    /// - `count` specifies the page counts to be extended to the [Memory].
    ///
    /// # Errors
    ///
    /// If fail to grow the page count, then an error is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use wasmedge_sys::{MemType, Memory};
    ///
    /// // create a Memory with a limit range [10, 20]
    /// let ty = MemType::create(10..=20).expect("fail to create a memory type");
    /// let mut mem = Memory::create(&ty).expect("fail to create a Memory");
    /// // check page count
    /// let count = mem.size();
    /// assert_eq!(count, 10);
    ///
    /// // grow 5 pages
    /// mem.grow(10).expect("fail to grow the page count");
    /// assert_eq!(mem.size(), 20);
    /// ```
    ///
    pub fn grow(&mut self, count: u32) -> WasmEdgeResult<()> {
        unsafe { check(ffi::WasmEdge_MemoryInstanceGrowPage(self.inner.0, count)) }
    }
}
impl Drop for Memory {
    fn drop(&mut self) {
        if !self.registered && !self.inner.0.is_null() {
            unsafe { ffi::WasmEdge_MemoryInstanceDelete(self.inner.0) };
        }
    }
}

#[derive(Debug)]
pub(crate) struct InnerMemory(pub(crate) *mut ffi::WasmEdge_MemoryInstanceContext);
unsafe impl Send for InnerMemory {}
unsafe impl Sync for InnerMemory {}

/// Struct of WasmEdge MemType.
///
/// A [MemType] classifies a [Memory] and its size range.
#[derive(Debug)]
pub struct MemType {
    pub(crate) inner: InnerMemType,
    pub(crate) registered: bool,
}
impl MemType {
    /// Create a new [MemType] to be associated with the given limit range for the capacity.
    ///
    /// # Arguments
    ///
    /// - `limit` specifies the linear memory size. The start value of the limit range specifies the min size (also, initial size) of the memory, while the end value specifies the max size allowed to grow. The maximum size is `u32::MAX`.
    ///
    /// # Errors
    ///
    /// If fail to create a [MemType], then an error is returned.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ty = MemType::create(0..=u32::MAX);
    /// ```
    ///
    pub fn create(limit: RangeInclusive<u32>) -> WasmEdgeResult<Self> {
        let ctx = unsafe { ffi::WasmEdge_MemoryTypeCreate(ffi::WasmEdge_Limit::from(limit)) };
        match ctx.is_null() {
            true => Err(WasmEdgeError::MemTypeCreate),
            false => Ok(Self {
                inner: InnerMemType(ctx),
                registered: false,
            }),
        }
    }

    /// Returns the limit range of a [MemType].
    ///
    /// # Example
    ///
    /// ```
    /// use wasmedge_sys::MemType;
    ///
    /// let ty = MemType::create(0..=u32::MAX).expect("fail to create a MemType");
    /// assert_eq!(ty.limit(), 0..=u32::MAX);
    /// ```
    ///
    pub fn limit(&self) -> RangeInclusive<u32> {
        let limit = unsafe { ffi::WasmEdge_MemoryTypeGetLimit(self.inner.0) };
        RangeInclusive::from(limit)
    }
}
impl Drop for MemType {
    fn drop(&mut self) {
        if !self.registered && !self.inner.0.is_null() {
            unsafe { ffi::WasmEdge_MemoryTypeDelete(self.inner.0) }
        }
    }
}

#[derive(Debug)]
pub(crate) struct InnerMemType(pub(crate) *mut ffi::WasmEdge_MemoryTypeContext);
unsafe impl Send for InnerMemType {}
unsafe impl Sync for InnerMemType {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{CoreError, CoreExecutionError, WasmEdgeError};
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    #[test]
    fn test_memory_type() {
        let result = MemType::create(0..=u32::MAX);
        assert!(result.is_ok());
        let ty = result.unwrap();
        assert!(!ty.inner.0.is_null());
        assert!(!ty.registered);

        let limit = ty.limit();
        assert_eq!(limit, 0..=u32::MAX);

        let result = MemType::create(10..=101);
        assert!(result.is_ok());
        let ty = result.unwrap();
        assert!(!ty.inner.0.is_null());
        assert!(!ty.registered);

        let limit = ty.limit();
        assert_eq!(limit, 10..=101);
    }

    #[test]
    fn test_memory_grow() {
        // create a Memory with a limit range [10, 20]
        let result = MemType::create(10..=20);
        assert!(result.is_ok());
        let ty = result.unwrap();
        let result = Memory::create(&ty);
        assert!(result.is_ok());
        let mut mem = result.unwrap();
        assert!(!mem.inner.0.is_null());
        assert!(!mem.registered);

        // get type
        let result = mem.ty();
        assert!(result.is_ok());
        let ty = result.unwrap();
        assert!(!ty.inner.0.is_null());
        assert!(ty.registered);
        // check limit
        assert_eq!(ty.limit(), 10..=20);

        // check page count
        let count = mem.size();
        assert_eq!(count, 10);

        // grow 5 pages
        let result = mem.grow(10);
        assert!(result.is_ok());
        assert_eq!(mem.size(), 20);

        // grow additional  pages, which causes a failure
        let result = mem.grow(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_data() {
        // create a Memory: the min size 1 and the max size 2
        let result = MemType::create(1..=2);
        assert!(result.is_ok());
        let ty = result.unwrap();
        let result = Memory::create(&ty);
        assert!(result.is_ok());
        let mut mem = result.unwrap();
        assert!(!mem.inner.0.is_null());
        assert!(!mem.registered);

        // check page count
        let count = mem.size();
        assert_eq!(count, 1);

        // get data before set data
        let result = mem.get_data(0, 10);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, vec![0; 10]);

        // set data
        let result = mem.set_data(vec![1; 10], 10);
        assert!(result.is_ok());
        // get data after set data
        let result = mem.get_data(10, 10);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, vec![1; 10]);

        // set data and the data length is larger than the data size in the memory
        let result = mem.set_data(vec![1; 10], u32::pow(2, 16) - 9);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WasmEdgeError::Core(CoreError::Execution(CoreExecutionError::MemoryOutOfBounds))
        );

        // grow the memory size
        let result = mem.grow(1);
        assert!(result.is_ok());
        assert_eq!(mem.size(), 2);
        let result = mem.set_data(vec![1; 10], u32::pow(2, 16) - 9);
        assert!(result.is_ok());
    }

    #[test]
    fn test_memory_send() {
        {
            let result = MemType::create(10..=101);
            assert!(result.is_ok());
            let ty = result.unwrap();
            assert!(!ty.inner.0.is_null());
            assert!(!ty.registered);

            let handle = thread::spawn(move || {
                assert!(!ty.inner.0.is_null());
                assert!(!ty.registered);

                let limit = ty.limit();
                assert_eq!(limit, 10..=101);
            });

            handle.join().unwrap()
        }

        {
            // create a Memory with a limit range [10, 20]
            let result = MemType::create(10..=20);
            assert!(result.is_ok());
            let ty = result.unwrap();
            let result = Memory::create(&ty);
            assert!(result.is_ok());
            let mem = result.unwrap();
            assert!(!mem.inner.0.is_null());
            assert!(!mem.registered);

            let handle = thread::spawn(move || {
                // get type
                let result = mem.ty();
                assert!(result.is_ok());
                let ty = result.unwrap();
                assert!(!ty.inner.0.is_null());
                assert!(ty.registered);
                // check limit
                assert_eq!(ty.limit(), 10..=20);

                // check page count
                let count = mem.size();
                assert_eq!(count, 10);
            });

            handle.join().unwrap()
        }
    }

    #[test]
    fn test_memory_sync() {
        // create a Memory with a limit range [10, 20]
        let result = MemType::create(10..=20);
        assert!(result.is_ok());
        let ty = result.unwrap();
        let result = Memory::create(&ty);
        assert!(result.is_ok());
        let mem = result.unwrap();
        assert!(!mem.inner.0.is_null());
        assert!(!mem.registered);
        let memory = Arc::new(Mutex::new(mem));

        let memory_cloned = Arc::clone(&memory);
        let handle = thread::spawn(move || {
            let mem = memory_cloned.lock().unwrap();

            // get type
            let result = mem.ty();
            assert!(result.is_ok());
            let ty = result.unwrap();
            assert!(!ty.inner.0.is_null());
            assert!(ty.registered);
            // check limit
            assert_eq!(ty.limit(), 10..=20);

            // check page count
            let count = mem.size();
            assert_eq!(count, 10);
        });

        handle.join().unwrap()
    }
}
