use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::il2cpp::{
    ext::Il2CppStringExt,
    hook::UnityEngine_CoreModule::{TouchScreenKeyboard, TouchScreenKeyboardType},
    symbols::{GCHandle, Thread},
    types::{Il2CppObject, Il2CppString, RangeInt},
};

use crate::core::utils::{char_to_utf16_index, utf16_to_char_index};

static PENDING_KB_TYPE: AtomicI32 = AtomicI32::new(0);
static PENDING_KEYBOARD_TEXT: AtomicPtr<Il2CppString> = AtomicPtr::new(std::ptr::null_mut());
static ACTIVE_KEYBOARD: AtomicPtr<Il2CppObject> = AtomicPtr::new(std::ptr::null_mut());
pub static KEYBOARD_GC_HANDLE: Lazy<Mutex<Option<GCHandle>>> = Lazy::new(|| Mutex::default());
static KEYBOARD_SELECTION: Lazy<Mutex<RangeInt>> = Lazy::new(|| Mutex::new(RangeInt::new(0, 1)));
pub static KEYBOARD_OWNER: Lazy<Mutex<Option<KeyboardOwner>>> = Lazy::new(|| Mutex::new(None));

#[derive(PartialEq)]
pub enum KeyboardOwner {
    JNI(egui::Id),
    Unity(egui::Id),
}

pub(crate) fn is_ime_visible() -> bool {
    let kb_ptr = ACTIVE_KEYBOARD.load(Ordering::Acquire);
    let unity_visible = if !kb_ptr.is_null() {
        TouchScreenKeyboard::get_status(kb_ptr) == TouchScreenKeyboard::Status::Visible
    } else {
        false
    };
    let jni_visible = crate::android::utils::IS_IME_VISIBLE.load(Ordering::Acquire);

    unity_visible || jni_visible
}

pub(crate) fn ime_scroll_padding(ctx: &egui::Context) -> f32 {
    if !is_ime_visible() {
        return 0.0;
    }
    ctx.input(|i| i.viewport_rect().height() * 0.35)
}

pub fn handle_android_keyboard<T: 'static>(res: &egui::Response, val: &mut T) {
    {
        let Ok(mut owner_lock) = KEYBOARD_OWNER.try_lock() else {
            return;
        };
        if let Some(KeyboardOwner::JNI(_)) = *owner_lock {
            return;
        }

        if res.lost_focus() {
            if let Some(KeyboardOwner::Unity(id)) = *owner_lock {
                if id == res.id {
                    let kb_ptr = ACTIVE_KEYBOARD.load(Ordering::Acquire);
                    if !kb_ptr.is_null() {
                        TouchScreenKeyboard::set_active(kb_ptr, false);
                        ACTIVE_KEYBOARD.store(std::ptr::null_mut(), Ordering::Release);
                        *KEYBOARD_GC_HANDLE.lock().unwrap() = None;
                    }
                    *owner_lock = None;
                }
            }
            return;
        }
    }

    if !res.has_focus() {
        return;
    }

    use egui::{
        text::{CCursor, CCursorRange},
        widgets::text_edit::TextEditState,
    };

    let val_any = val as &dyn std::any::Any;
    PENDING_KB_TYPE.store(TouchScreenKeyboardType::KeyboardType::Default as i32, Ordering::Release);

    let text = if let Some(s) = val_any.downcast_ref::<String>() {
        s.clone()
    } else if let Some(f) = val_any.downcast_ref::<f32>() {
        PENDING_KB_TYPE.store(
            TouchScreenKeyboardType::KeyboardType::DecimalPad as i32,
            Ordering::Release,
        );
        if f.fract() == 0.0 {
            format!("{:.1}", f)
        } else {
            f.to_string()
        }
    } else if let Some(i) = val_any.downcast_ref::<i32>() {
        PENDING_KB_TYPE.store(
            TouchScreenKeyboardType::KeyboardType::NumberPad as i32,
            Ordering::Release,
        );
        i.to_string()
    } else {
        String::new()
    };

    if res.gained_focus() {
        {
            let mut owner_lock = KEYBOARD_OWNER.lock().unwrap();
            *owner_lock = Some(KeyboardOwner::Unity(res.id));
        }

        res.scroll_to_me(Some(egui::Align::Center));

        let ptr = text.to_il2cpp_string();
        PENDING_KEYBOARD_TEXT.store(ptr, Ordering::Release);

        let initial_selection = res.ctx.data(|data| {
            data.get_temp::<TextEditState>(res.id)
                .and_then(|state| state.cursor.char_range())
                .map(|range| {
                    let start_char = range.primary.index.min(range.secondary.index);
                    let end_char = range.primary.index.max(range.secondary.index);

                    let start_u16 = char_to_utf16_index(&text, start_char);
                    let end_u16 = char_to_utf16_index(&text, end_char);

                    RangeInt::new(start_u16, end_u16 - start_u16)
                })
                .unwrap_or(RangeInt::new(char_to_utf16_index(&text, text.chars().count()), 0))
        });
        *KEYBOARD_SELECTION.lock().unwrap() = initial_selection;

        Thread::main_thread().schedule(|| {
            let ptr = PENDING_KEYBOARD_TEXT.swap(std::ptr::null_mut(), Ordering::AcqRel);
            // SAFETY: FFI / raw pointer operation required by IL2CPP interop
            let typ: TouchScreenKeyboardType::KeyboardType = unsafe {
                *(&PENDING_KB_TYPE.load(Ordering::Acquire) as *const i32
                    as *const TouchScreenKeyboardType::KeyboardType)
            };

            if !ptr.is_null() {
                let keyboard = TouchScreenKeyboard::Open(ptr, typ, false, false, false);
                TouchScreenKeyboard::set_selection(keyboard, *KEYBOARD_SELECTION.lock().unwrap());
                let handle = GCHandle::new(keyboard, false);
                *KEYBOARD_GC_HANDLE.lock().unwrap() = Some(handle);
                ACTIVE_KEYBOARD.store(keyboard, Ordering::Release);
            }
        });
    }

    let kb_ptr = ACTIVE_KEYBOARD.load(Ordering::Acquire);
    if !kb_ptr.is_null() {
        let status = TouchScreenKeyboard::get_status(kb_ptr);

        if status == TouchScreenKeyboard::Status::Visible {
            let unity_range = TouchScreenKeyboard::get_selection(kb_ptr);

            let kb_txt_ptr = TouchScreenKeyboard::get_text(kb_ptr);
            // SAFETY: FFI / raw pointer operation required by IL2CPP interop
            if let Some(kb_ref) = unsafe { kb_txt_ptr.as_ref() } {
                let kb_txt_str = kb_ref.as_utf16str().to_string();

                let val_any_mut = val as &mut dyn std::any::Any;

                if let Some(s) = val_any_mut.downcast_mut::<String>() {
                    if *s != kb_txt_str {
                        *s = kb_txt_str.clone();
                    }
                } else if let Some(f) = val_any_mut.downcast_mut::<f32>() {
                    if let Ok(parsed) = kb_txt_str.parse::<f32>() {
                        let changed = !egui::emath::almost_equal(*f, parsed, 1e-6);
                        let drafting =
                            kb_txt_str.ends_with('.') || (kb_txt_str.contains('.') && kb_txt_str.ends_with('0'));

                        if changed && !drafting {
                            *f = parsed;
                        }
                    }
                } else if let Some(i) = val_any_mut.downcast_mut::<i32>() {
                    if let Ok(parsed) = kb_txt_str.parse::<i32>() {
                        if *i != parsed {
                            *i = parsed;
                        }
                    }
                }

                let kb_txt_clone = kb_txt_str.clone();
                res.ctx.data_mut(|data| {
                    if let Some(mut state) = data.get_temp::<TextEditState>(res.id) {
                        let start_char = utf16_to_char_index(&kb_txt_clone, unity_range.start as usize);
                        let end_char =
                            utf16_to_char_index(&kb_txt_clone, (unity_range.start + unity_range.length) as usize);

                        let new_range = CCursorRange::two(CCursor::new(start_char), CCursor::new(end_char));

                        if state.cursor.char_range() != Some(new_range) {
                            state.cursor.set_char_range(Some(new_range));
                            data.insert_temp(res.id, state);
                        }
                    }
                });
            }
            res.ctx.request_repaint();
        }

        if status != TouchScreenKeyboard::Status::Visible {
            res.surrender_focus();
            res.ctx.memory_mut(|mem| mem.stop_text_input());
            res.ctx.data_mut(|data| {
                data.remove::<egui::widgets::text_edit::TextEditState>(res.id);
            });

            ACTIVE_KEYBOARD.store(std::ptr::null_mut(), Ordering::Release);
            *KEYBOARD_GC_HANDLE.lock().unwrap() = None;
            res.ctx.request_repaint();
        }
    }
}
