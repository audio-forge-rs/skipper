---
paths: src/**/*.rs, nih-plug/**/*.rs
---

# Audio Thread Rules

**CRITICAL:** The `process()` function runs on the audio thread. With `assert_process_allocs` enabled, ANY memory allocation OR deallocation crashes.

## Forbidden in process()

- `String` creation or `format!()`
- `Vec` allocation or resizing
- `Box::new()` or any heap allocation
- File I/O (`std::fs::*`)
- `std::env::var_os()` - allocates internally
- `nih_log!()` or any logging - allocates strings
- Cloning types that contain `String` or `Vec`
- **Dropping the last `Arc<T>` if T contains heap data** - deallocation!
- Replacing `Option<Arc<T>>` values - drops old value, may deallocate

## Safe in process()

- Pre-allocated buffers
- Stack variables (fixed-size arrays, primitives)
- Atomic operations (`AtomicU32`, `AtomicBool`, etc.)
- Cloning `Arc<T>` (just increments ref count, no allocation)
- Reading through `Arc<RwLock<T>>`
- Copying `Copy` types (integers, floats, tuples of Copy types)

## Subtle Traps

```rust
// BAD: Cloning TrackInfo allocates (contains String fields)
let track_info = context.track_info(); // Returns Arc<TrackInfo>
state.track_info = track_info.clone(); // Safe - Arc clone is cheap

// BAD: But REPLACING an Option<Arc<T>> can DROP the old Arc!
if new_info != state.track_info {
    state.track_info = new_info; // If old Arc had refcount=1, this DEALLOCATES!
}

// GOOD: Update state only from main thread callbacks (initialize, changed)
// Don't poll/update track_info in process() at all
```

**Rule of thumb:** In `process()`, only READ cached data. UPDATE cached data from main-thread callbacks (`initialize()`, CLAP `changed()` callbacks, etc.).

## Shared State Between GUI and Audio Thread

Use `AtomicRefCell` (not `parking_lot::RwLock` which allocates):

```rust
use atomic_refcell::AtomicRefCell;

struct SharedState { /* ... */ }
state: Arc<AtomicRefCell<SharedState>>
```

**CRITICAL:** Both sides must use `try_borrow`/`try_borrow_mut` to avoid panics:

```rust
// BAD: borrow() panics if other thread holds lock
let shared = state.borrow();  // PANIC if audio thread has borrow_mut!

// GOOD: try_borrow gracefully handles contention
let Ok(shared) = state.try_borrow() else {
    return;  // Skip this frame, try again next time
};
```

**In process() (audio thread):**
```rust
if let Ok(mut state) = self.state.try_borrow_mut() {
    state.transport.tempo = transport.tempo;
}
```

**In GUI (main thread):**
```rust
let Ok(shared) = state.try_borrow() else {
    ui.label("Loading...");
    return;
};
```

**Why not RwLock?** `parking_lot::RwLock` allocates thread-local data on first lock acquisition, crashing with `assert_process_allocs`.
