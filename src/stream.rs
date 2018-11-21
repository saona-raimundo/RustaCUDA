//! Streams of work for the device to perform.
//!
//! In CUDA, most work is performed asynchronously. Even tasks such as memory copying can be
//! scheduled by the host and performed when ready. Scheduling this work is done using a Stream.
//!
//! A stream is required for all asynchronous tasks in CUDA, such as kernel launches and
//! asynchronous memory copying. Each task in a stream is performed in the order it was scheduled,
//! and tasks within a stream cannot overlap. Tasks scheduled in multiple streams may interleave or
//! execute concurrently. Sequencing between multiple streams can be achieved using events, which
//! are not currently supported by RustaCUDA. Finally, the host can wait for all work scheduled in
//! a stream to be completed.

use cuda_sys::cuda::{self, CUstream};
use error::{CudaResult, ToResult};
use std::mem;
use std::ptr;

bitflags! {
    /// Bit flags for configuring a CUDA Stream.
    pub struct StreamFlags: u32 {
        /// No flags set.
        const DEFAULT = 0x00;

        /// This stream does not synchronize with the NULL stream.
        ///
        /// Note that the name is chosen to correspond to CUDA documentation, but is nevertheless
        /// misleading. All work within a single stream is ordered and asynchronous regardless
        /// of whether this flag is set. All streams in RustaCUDA may execute work concurrently,
        /// regardless of the flag. However, for legacy reasons, CUDA has a notion of a NULL stream,
        /// which is used as the default when no other stream is provided. Work on other streams
        /// may not be executed concurrently with work on the NULL stream unless this flag is set.
        /// Since RustaCUDA does not provide access to the NULL stream, this flag has no effect in
        /// most circumstances. However, it is recommended to use it anyway, as some other crate
        /// in this binary may be using the NULL stream directly.
        const NON_BLOCKING = 0x01;
    }
}

/// A stream of work for the device to perform.
///
/// See the module-level documentation for more information.
#[derive(Debug)]
pub struct Stream {
    inner: CUstream,
}
impl Stream {
    /// Create a new stream with the given flags and optional priority.
    ///
    /// By convention, `priority` follows a convention where lower numbers represent greater
    /// priorities. That is, work in a stream with a lower priority number may pre-empt work in
    /// a stream with a higher priority number. `Context::get_stream_priority_range` can be used
    /// to get the range of valid priority values; if priority is set outside that range, it will
    /// be automatically clamped to the lowest or highest number in the range.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use rustacuda::*;
    /// # let _ctx = quick_init().unwrap();
    /// use rustacuda::stream::{Stream, StreamFlags};
    ///
    /// // With default priority
    /// let stream = Stream::new(StreamFlags::NON_BLOCKING, None).unwrap();
    ///
    /// // With specific priority
    /// let priority = Stream::new(StreamFlags::NON_BLOCKING, 1i32.into()).unwrap();
    /// ```
    pub fn new(flags: StreamFlags, priority: Option<i32>) -> CudaResult<Stream> {
        unsafe {
            let mut stream = Stream {
                inner: ptr::null_mut(),
            };
            cuda::cuStreamCreateWithPriority(
                &mut stream.inner as *mut CUstream,
                flags.bits(),
                priority.unwrap_or(0),
            ).toResult()?;
            Ok(stream)
        }
    }

    /// Return the flags which were used to create this stream.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use rustacuda::*;
    /// # let _ctx = quick_init().unwrap();
    /// use rustacuda::stream::{Stream, StreamFlags};
    ///
    /// let stream = Stream::new(StreamFlags::NON_BLOCKING, None).unwrap();
    /// assert_eq!(StreamFlags::NON_BLOCKING, stream.get_flags().unwrap());
    /// ```
    pub fn get_flags(&self) -> CudaResult<StreamFlags> {
        unsafe {
            let mut bits = 0u32;
            cuda::cuStreamGetFlags(self.inner, &mut bits as *mut u32).toResult()?;
            Ok(StreamFlags::from_bits_truncate(bits))
        }
    }

    /// Return the priority of this stream.
    ///
    /// If this stream was created without a priority, returns the default priority.
    /// If the stream was created with a priority outside the valid range, returns the clamped
    /// priority.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use rustacuda::*;
    /// # let _ctx = quick_init().unwrap();
    /// use rustacuda::stream::{Stream, StreamFlags};
    ///
    /// let stream = Stream::new(StreamFlags::NON_BLOCKING, 1i32.into()).unwrap();
    /// println!("{}", stream.get_priority().unwrap());
    /// ```
    pub fn get_priority(&self) -> CudaResult<i32> {
        unsafe {
            let mut priority = 0i32;
            cuda::cuStreamGetPriority(self.inner, &mut priority as *mut i32).toResult()?;
            Ok(priority)
        }
    }

    /// Wait until a stream's tasks are completed.
    ///
    /// Waits until the device has completed all operations scheduled for this stream.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use rustacuda::*;
    /// # let _ctx = quick_init().unwrap();
    /// use rustacuda::stream::{Stream, StreamFlags};
    ///
    /// let stream = Stream::new(StreamFlags::NON_BLOCKING, 1i32.into()).unwrap();
    ///
    /// // ... queue up some work on the stream
    ///
    /// // Wait for the work to be completed.
    /// stream.synchronize().unwrap();
    /// ```
    pub fn synchronize(&self) -> CudaResult<()> {
        unsafe { cuda::cuStreamSynchronize(self.inner).toResult() }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        if self.inner.is_null() {
            return;
        }

        unsafe {
            let inner = mem::replace(&mut self.inner, ptr::null_mut());
            // No choice but to panic here.
            cuda::cuStreamDestroy_v2(inner).toResult().unwrap();
        }
    }
}
