//! # ~~Triple~~ Multi-Buffering Crate
//!
//! This crate provides utilities for implementing triple buffering and generalized
//! multi-buffering patterns. It includes the [`TripleBuffer`] type alias, built on top of
//! the generic [`MultiBuffer`] struct, enabling developers to create and manage buffers
//! designed for high-performance, concurrent, or time-sensitive workloads.
//!
//! The primary purpose of this crate is to facilitate safe, lock-free access to the latest
//! data in one buffer while enabling concurrent modifications in other buffers. Each buffer
//! is associated with a version tag (`BufferTag`), which helps track updates and ensures
//! consumers can identify the most recent data or detect stale reads.
//!
//! ## Main Features
//!
//! - **`TripleBuffer<T>`**: A convenient alias for three buffers (standard triple buffering).
//! - **Generic Multi-buffering**: `MultiBuffer<T, SIZE>` supports any number of buffers
//!   through constant generics.
//! - **Buffer tags for tracking updates**: Each buffer has a version tag to help identify the latest state.
//! - **Lock-free concurrency**: Allows safe access without blocking threads.
//! - **Convenience methods**: Intuitive API for buffer access, mutation, and publishing updated buffers.
//!
//! ## Examples
//!
//! ### TripleBuffer Example
//!
//! The following example demonstrates how to use a `TripleBuffer` in typical scenarios:
//!
//! ```rust
//! use multibuffer::TripleBuffer;
//!
//! fn main() {
//!     // Create a TripleBuffer with an initial value of 0.
//!     let buffer = TripleBuffer::new(|| 0);
//!
//!     // Modify a specific buffer (index 1).
//!     *buffer.get_mut(1) = 42;
//!
//!     // Publish the updated buffer (index 1) with a version tag of 1.
//!     buffer.publish(1, 1);
//!
//!     // Access the latest published buffer and its tag.
//!     let (latest, tag) = buffer.get_latest();
//!     assert_eq!(*latest, 42);
//!     assert_eq!(tag, 1);
//!
//!     // Perform further modifications and publishing.
//!     *buffer.get_mut(2) = 100;
//!     buffer.publish(2, 2);
//!     let (latest, tag) = buffer.get_latest();
//!     assert_eq!(*latest, 100);
//!     assert_eq!(tag, 2);
//! }
//! ```
//!
//! ## Implementation Details
//!
//! - The [`MultiBuffer`] struct maintains an internal array of buffers, version tags, and
//!   a fence pointer to track the latest published buffer.
//! - Buffers are allocated using [`UnsafeCell`] to allow efficient mutable access without requiring
//!   synchronization primitives.
//! - The version tags (`BufferTag`) are updated on publishing and help consumers confirm
//!   they are reading the most up-to-date data.
//! - The `Sync` implementation is designed with precautions to ensure safe multi-threaded usage,
//!   as long as the user adheres to the safety rules outlined in the API documentation.
//!
//! ## Notes
//!
//! - Correctness of the implementation depends on the user following the API's safety guidelines.
//!   Specifically, concurrent misuse of the `get_latest` and `get_mut` methods may result in
//!   undefined behavior.
//! - The crate relies on Rust's constant generics (`const SIZE: usize`) to specify the number of
//!   buffers, which needs to be determined at compile-time.

#![allow(unsafe_code)]

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// A type alias for a triple buffer, which is a specialized form of the [`MultiBuffer`] struct
/// with a fixed size of three buffers. This is commonly used for triple buffering scenarios.
pub type TripleBuffer<T> = MultiBuffer<T, 3>;

/// A type alias representing the index of a buffer.
///
/// This is used to identify a specific buffer in a multi-buffering structure.
pub type BufferIndex = usize;

/// A type alias for the version tag of a buffer.
///
/// Each buffer in the multi-buffering structure is associated with a `BufferTag`
/// (type alias for `u64`), which tracks the version or state of the buffer.
/// This tag helps consumers determine if a buffer contains updated or stale data.
pub type BufferTag = u64;

/// A generic multi-buffering structure designed for lock-free concurrency, supporting a
/// configurable number of buffers, each tagged with a version number for tracking updates.
///
/// # Type Parameters
/// - `T`: The type of data stored in each buffer.
/// - `SIZE`: The number of buffers in the structure, which is fixed at compile time.
///
/// # Structure
/// - Each buffer has an associated tag (`u64`) to track its version or state. This allows
///   consumers to determine if a specific buffer contains newer data.
/// - An internal fence pointer is used to indicate the latest published buffer.
///
/// # Safety
/// - The buffers are wrapped in [`UnsafeCell`] to allow efficient mutable access without
///   synchronization primitives.
/// - The implementation assumes the user correctly follows the API's safety guarantees to
///   avoid undefined behavior, especially when using methods like `get_mut` and `publish`
///   in a concurrent environment.
/// - Correct use of the API ensures that data remains consistent and race conditions are avoided.
///
/// # Usage
/// This structure is particularly useful in scenarios requiring efficient lock-free
/// communication of data across threads, such as real-time processing or streaming.
pub struct MultiBuffer<T, const SIZE: usize> {
    buffers: [UnsafeCell<T>; SIZE],
    tags: [AtomicU64; SIZE],
    fence: AtomicUsize,
}

impl<T, const SIZE: usize> MultiBuffer<T, SIZE> {
    /// The number of buffers in the structure, determined at compile-time.
    pub const SIZE: usize = SIZE;

    /// Returns the number of buffers in the structure.
    ///
    /// This is a constant value defined by the `SIZE` generic parameter.
    pub const fn len(&self) -> usize {
        SIZE
    }
}

// SAFETY: only one thread writes to a buffer at a time, controlled by design
unsafe impl<T, const SIZE: usize> Sync for MultiBuffer<T, SIZE> {}

impl<T, const SIZE: usize> MultiBuffer<T, SIZE> {
    /// Creates a new `MultiBuffer` with the specified size and initializes each buffer
    /// with the provided `factory` function.
    ///
    /// # Parameters
    /// - `factory`: A function or closure that produces an initial value for each buffer.
    ///
    /// # Returns
    /// A new instance of `MultiBuffer` initialized with the values created by the `factory`.
    ///
    /// # Panics
    /// This method will panic if the `factory` function panics during initialization.
    pub fn new(mut factory: impl FnMut() -> T) -> Self {
        let mut buffers = MaybeUninit::<[UnsafeCell<T>; SIZE]>::uninit();
        let ptr = buffers.as_mut_ptr() as *mut UnsafeCell<T>;

        for i in 0..SIZE {
            let value = factory();
            unsafe {
                ptr.add(i).write(UnsafeCell::new(value));
            }
        }

        let buffers = unsafe { buffers.assume_init() };

        MultiBuffer {
            buffers,
            tags: [const { AtomicU64::new(0) }; SIZE],
            fence: AtomicUsize::new(0),
        }
    }

    /// Retrieves a reference to the latest published buffer.
    ///
    /// # Returns
    /// A reference to the data in the buffer that was most recently published,
    /// along with its associated version tag.
    ///
    /// # Safety
    /// - The method assumes that the underlying concurrency rules are correctly followed
    ///   and guarantees a consistent snapshot of the latest buffer.
    /// - The version tag provides a mechanism to track updates and ensure consumers
    ///   can identify if they are working with the most recent data or detect
    ///   stale reads.
    /// - This method uses `UnsafeCell` dereferencing internally, which is safe as long as
    ///   only one thread is modifying the buffer while others are reading it.
    #[inline]
    #[must_use]
    pub fn get_latest(&self) -> (&T, BufferTag) {
        let idx = self.fence.load(Ordering::Acquire) % SIZE;
        let tag = self.tags[idx].load(Ordering::Acquire);
        let value = unsafe { &*self.buffers[idx].get() };
        (value, tag)
    }

    /// Retrieves a mutable reference to a specific buffer by its index.
    ///
    /// # Parameters
    /// - `index`: The index of the buffer to retrieve.
    ///
    /// # Returns
    /// A mutable reference to the specified buffer.
    ///
    /// # Safety
    /// - The caller must ensure that the accessed buffer is not being simultaneously read
    ///   or written from another thread.
    /// - This method uses internal `UnsafeCell` dereferencing, which assumes the correct
    ///   usage of `MultiBuffer`'s API in a concurrent context.
    #[inline]
    #[must_use]
    pub fn get_mut(&self, index: BufferIndex) -> &mut T {
        unsafe { &mut *self.buffers[index % SIZE].get() }
    }

    /// Publishes a specific buffer by updating the internal fence and its version tag.
    ///
    /// # Parameters
    /// - `index`: The index of the buffer to publish.
    /// - `version`: The version tag to assign to the buffer being published.
    ///
    /// # Safety
    /// - The caller must ensure that the buffer being published is fully modified
    ///   and in a valid state before calling this method.
    /// - This method uses memory ordering guarantees (`Release`) to ensure that
    ///   the changes made to the buffer are visible to other threads before the publish
    ///   operation completes.
    /// - It is the caller's responsibility to prevent data races by adhering to proper
    ///   concurrent usage patterns.
    #[inline]
    pub fn publish(&self, index: BufferIndex, version: BufferTag) {
        let index = index % SIZE;
        self.tags[index].store(version, Ordering::Release);
        self.fence.store(index, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multibuffer() {
        // Create a TripleBuffer with initial values
        let buffer = TripleBuffer::new(|| 0);

        // Test the initial size of the buffer
        assert_eq!(buffer.len(), 3);

        // Verify the latest buffer returns the initial value and its tag
        let (latest, tag) = buffer.get_latest();
        assert_eq!(*latest, 0);
        assert_eq!(tag, 0);

        // Modify a specific buffer
        *buffer.get_mut(1) = 42;

        // Publish the updated buffer (index 1) with a version tag
        buffer.publish(1, 1);

        // Verify the latest buffer is now the updated one with the correct tag
        let (latest, tag) = buffer.get_latest();
        assert_eq!(*latest, 42);
        assert_eq!(tag, 1);

        // Modify another specific buffer
        *buffer.get_mut(2) = 100;

        // Publish the updated buffer (index 2) with a new version tag
        buffer.publish(2, 2);

        // Verify the latest buffer is now the second updated one with the correct tag
        let (latest, tag) = buffer.get_latest();
        assert_eq!(*latest, 100);
        assert_eq!(tag, 2);

        // Modify and publish a buffer with wrapping around indices (e.g., index 4 maps to 1)
        *buffer.get_mut(4) = 7;
        buffer.publish(4, 3);

        // The latest buffer should now be the one at index 1 (mod TRIPLE) with the correct tag
        let (latest, tag) = buffer.get_latest();
        assert_eq!(*latest, 7);
        assert_eq!(tag, 3);
    }
}
