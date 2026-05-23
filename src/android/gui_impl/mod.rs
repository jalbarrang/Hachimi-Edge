pub mod input_hook;
pub mod keymap;
mod render_hook;

pub fn init() {
    render_hook::init();
    input_hook::init();
}
