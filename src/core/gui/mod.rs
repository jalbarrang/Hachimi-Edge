//! In-game egui overlay: menu, dialogs, notifications, and plugin UI.

#[cfg(target_os = "android")]
mod android_keyboard;
mod frame;
mod instance;
mod menu;
mod notification;
mod overlays;
mod scale;
mod splash;
mod theme_preview;
mod tween;
mod update_progress;
mod window;

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use once_cell::sync::OnceCell;
use std::time::Instant;

use fnv::FnvHashSet;
use once_cell::sync::Lazy;

use crate::core::hachimi;
use crate::core::utils::SendPtr;

pub(crate) use notification::Notification;
pub(crate) use tween::TweenInOutWithDelay;
pub(crate) use window::BoxedWindow;

#[cfg(target_os = "android")]
pub use android_keyboard::{handle_android_keyboard, KeyboardOwner, KEYBOARD_GC_HANDLE, KEYBOARD_OWNER};
pub use notification::NotificationGuard;
pub use theme_preview::enqueue_theme_preview;
pub use window::{PersistentMessageWindow, SimpleOkDialog, SimpleYesNoDialog, Window};

pub struct Gui {
    pub context: egui::Context,
    pub(crate) config: hachimi::Config,
    pub input: egui::RawInput,
    pub(crate) default_style: egui::Style,
    pub gui_scale: f32,

    pub finalized_scale: f32,
    pub start_time: Instant,
    pub prev_main_axis_size: i32,
    pub(crate) last_fps_update: Instant,
    pub(crate) tmp_frame_count: u32,
    pub(crate) fps_text: String,
    pub(crate) last_focused: Option<egui::Id>,
    #[cfg(target_os = "android")]
    pub(crate) ime_cooldown: Option<Instant>,

    pub(crate) show_menu: bool,

    pub(crate) splash_visible: bool,
    pub(crate) splash_tween: TweenInOutWithDelay,
    pub(crate) splash_sub_str: String,

    pub(crate) menu_visible: bool,
    pub(crate) menu_anim_time: Option<Instant>,
    pub(crate) menu_fps_value: i32,

    #[cfg(target_os = "windows")]
    pub(crate) menu_vsync_value: i32,

    pub update_progress_visible: bool,

    pub(crate) notifications: Vec<Notification>,
    pub(crate) next_notification_id: u32,
    pub(crate) windows: Vec<BoxedWindow>,
}

pub(crate) const PIXELS_PER_POINT_RATIO: f32 = 3.0 / 1080.0;

pub(crate) static INSTANCE: OnceCell<Mutex<Gui>> = OnceCell::new();
pub(crate) static IS_CONSUMING_INPUT: AtomicBool = AtomicBool::new(false);
pub(crate) static DISABLED_GAME_UIS: Lazy<Mutex<FnvHashSet<SendPtr>>> = Lazy::new(|| Mutex::new(FnvHashSet::default()));
