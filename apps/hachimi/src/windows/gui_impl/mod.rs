mod d3d11_backup;
mod d3d11_painter;
pub mod input;
pub mod render_hook;

pub fn init() {
    render_hook::init();
}
