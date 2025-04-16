# ~~Triple~~ Multi-Buffering Crate

This crate provides utilities for implementing triple buffering and generalized
multi-buffering patterns. It includes the `TripleBuffer` type alias, built on top of
the generic `MultiBuffer` struct, enabling developers to create and manage buffers
designed for high-performance, concurrent, or time-sensitive workloads.

The primary purpose of this crate is to facilitate safe, lock-free access to the latest
data in one buffer while enabling concurrent modifications in other buffers. Each buffer
is associated with a version tag (`BufferTag`), which helps track updates and ensures
consumers can identify the most recent data or detect stale reads.

## Main Features

- **`TripleBuffer<T>`**: A convenient alias for three buffers (standard triple buffering).
- **Generic Multi-buffering**: `MultiBuffer<T, SIZE>` supports any number of buffers
  through constant generics.
- **Buffer tags for tracking updates**: Each buffer has a version tag to help identify the latest state.
- **Lock-free concurrency**: Allows safe access without blocking threads.
- **Convenience methods**: Intuitive API for buffer access, mutation, and publishing updated buffers.

## Examples

### TripleBuffer Example

The following example demonstrates how to use a `TripleBuffer` in typical scenarios:

```rust
use multibuffer::TripleBuffer;

fn main() {
    // Create a TripleBuffer with an initial value of 0.
    let buffer = TripleBuffer::new(|| 0);

    // Modify a specific buffer (index 1).
    *buffer.get_mut(1) = 42;

    // Publish the updated buffer (index 1) with a version tag of 1.
    buffer.publish(1, 1);

    // Access the latest published buffer and its tag.
    let (latest, tag) = buffer.get_latest();
    assert_eq!(*latest, 42);
    assert_eq!(tag, 1);

    // Perform further modifications and publishing.
    *buffer.get_mut(2) = 100;
    buffer.publish(2, 2);
    let (latest, tag) = buffer.get_latest();
    assert_eq!(*latest, 100);
    assert_eq!(tag, 2);
}
```

## Implementation Details

- The[`MultiBuffer` struct maintains an internal array of buffers, version tags, and
  a fence pointer to track the latest published buffer.
- Buffers are allocated using `UnsafeCell` to allow efficient mutable access without requiring
  synchronization primitives.
- The version tags (`BufferTag`) are updated on publishing and help consumers confirm
  they are reading the most up-to-date data.
- The `Sync` implementation is designed with precautions to ensure safe multi-threaded usage,
  as long as the user adheres to the safety rules outlined in the API documentation.

## Notes

- Correctness of the implementation depends on the user following the API's safety guidelines.
  Specifically, concurrent misuse of the `get_latest` and `get_mut` methods may result in
  undefined behavior.
- The crate relies on Rust's constant generics (`const SIZE: usize`) to specify the number of
  buffers, which needs to be determined at compile-time.
