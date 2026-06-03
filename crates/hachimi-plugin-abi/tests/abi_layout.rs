use hachimi_plugin_abi::{Vtable, API_VERSION, VTABLE_SLOT_COUNT};

#[test]
fn vtable_size_is_stable() {
    assert_eq!(
        std::mem::size_of::<Vtable>(),
        VTABLE_SLOT_COUNT * std::mem::size_of::<usize>(),
        "Vtable size changed — plugin ABI break"
    );
}

#[test]
fn vtable_is_copy() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<Vtable>();
}

#[test]
fn api_version_constant() {
    assert_eq!(API_VERSION, 10);
    assert_eq!(VTABLE_SLOT_COUNT, 43);
}
