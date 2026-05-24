//! Plugin API version and vtable slot count.

/// Current plugin API version passed to `hachimi_init` alongside the vtable pointer.
pub const API_VERSION: i32 = 7;

/// Number of function pointers in `Vtable` (append-only ABI).
pub const VTABLE_SLOT_COUNT: usize = 57;
