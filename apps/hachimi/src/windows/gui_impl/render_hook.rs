#![allow(non_snake_case)]
use std::{
    os::raw::{c_uint, c_void},
    sync::Mutex,
};

use once_cell::sync::OnceCell;
use windows::{
    core::{w, Interface, HRESULT},
    Win32::{
        Foundation::{HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::{D3D11CreateDeviceAndSwapChain, ID3D11Device, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION},
            Dxgi::{
                Common::{DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_SAMPLE_DESC},
                IDXGISwapChain, DXGI_SWAP_CHAIN_DESC, DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, IsIconic, RegisterClassExW,
            UnregisterClassW, WINDOW_EX_STYLE, WNDCLASSEXW, WS_DISABLED,
        },
    },
};

use crate::{
    core::{Error, Gui, Hachimi, Interceptor},
    windows::wnd_hook,
};

use super::d3d11_painter::D3D11Painter;

fn check_hwnd(this: *mut c_void) -> HWND {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let swap_chain = unsafe { std::mem::ManuallyDrop::new(IDXGISwapChain::from_raw(this)) };

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let desc = unsafe {
        match swap_chain.GetDesc() {
            Ok(d) => d,
            Err(_) => return HWND(std::ptr::null_mut()),
        }
    };

    let target = wnd_hook::get_target_hwnd();
    if desc.OutputWindow == target {
        target
    } else {
        HWND(std::ptr::null_mut())
    }
}

static IME_COMPOSITION_POS: Mutex<(f32, f32)> = Mutex::new((0.0, 0.0));

static mut PRESENT_ADDR: usize = 0;
type PresentFn = extern "C" fn(this: *mut c_void, sync_interval: c_uint, flags: c_uint) -> HRESULT;
extern "C" fn IDXGISwapChain_Present(this: *mut c_void, sync_interval: c_uint, flags: c_uint) -> HRESULT {
    // SAFETY: Transmute required for IL2CPP type conversion
    let orig_fn: PresentFn = unsafe { std::mem::transmute(PRESENT_ADDR) };

    let hwnd = check_hwnd(this);
    if hwnd.0.is_null() {
        return orig_fn(this, sync_interval, flags);
    }

    let mut gui = Gui::instance_or_init("windows.menu_open_key")
        .lock()
        .expect("lock poisoned");
    let painter_mutex = match init_painter(this) {
        Ok(v) => v,
        Err(e) => {
            error!("{}", e);
            info!("Unhooking IDXGISwapChain hooks");

            let res = orig_fn(this, sync_interval, flags);
            let interceptor = &Hachimi::instance().interceptor;
            interceptor.unhook(IDXGISwapChain_Present as *const () as usize);
            interceptor.unhook(IDXGISwapChain_ResizeBuffers as *const () as usize);
            return res;
        }
    };
    // Skip if the GUI is empty or the window is minimized
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    if gui.is_empty() || unsafe { IsIconic(hwnd).into() } {
        return orig_fn(this, sync_interval, flags);
    }
    // Check if this is the right swap chain
    let mut painter = painter_mutex.lock().expect("lock poisoned");
    if this != painter.swap_chain().as_raw() {
        return orig_fn(this, sync_interval, flags);
    }

    // Get window size
    let mut rect = RECT::default();
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    if let Err(e) = unsafe { GetClientRect(hwnd, &mut rect) } {
        error!("Failed to get client rect: {}", e);
        return orig_fn(this, sync_interval, flags);
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    gui.set_screen_size(width, height);

    // Run and render the GUI
    let output = gui.run();

    for viewport_output in output.viewport_output.values() {
        for cmd in &viewport_output.commands {
            // Intercept egui telling the OS where the text cursor currently is
            if let egui::ViewportCommand::IMERect(rect) = cmd {
                // Windows IME boxes usually look best positioned at the bottom-left of the cursor.
                let zoom = gui.context.zoom_factor();
                let x = rect.min.x * zoom;
                let y = rect.max.y * zoom;
                let y_unity = height as f32 - y;
                *IME_COMPOSITION_POS.lock().expect("lock poisoned") = (x, y_unity);

                crate::il2cpp::symbols::Thread::main_thread().schedule(|| {
                    let (x, y_unity) = *IME_COMPOSITION_POS.lock().expect("lock poisoned");

                    crate::il2cpp::hook::UnityEngine_InputLegacyModule::Input::set_compositionCursorPos(
                        crate::il2cpp::types::Vector2_t { x, y: y_unity },
                    );
                });
            }
        }
    }

    let (mut renderer_output, _, _) = egui_directx11::split_output(output);

    let layout_pixels_per_point = renderer_output.pixels_per_point;

    let clipped_primitives = gui.context.tessellate(renderer_output.shapes, layout_pixels_per_point);

    renderer_output.shapes = clipped_primitives
        .into_iter()
        .map(|p| egui::epaint::ClippedShape {
            clip_rect: p.clip_rect,
            shape: match p.primitive {
                egui::epaint::Primitive::Mesh(mesh) => egui::Shape::Mesh(mesh.into()),
                egui::epaint::Primitive::Callback(cb) => egui::Shape::Callback(cb),
            },
        })
        .collect();

    renderer_output.pixels_per_point = 1.0;

    if let Err(e) = painter.present(&gui.context, renderer_output) {
        error!("Failed to render GUI: {}", e);
    }

    orig_fn(this, sync_interval, flags)
}

static mut RESIZEBUFFERS_ADDR: usize = 0;
type ResizeBuffersFn = extern "C" fn(
    this: *mut c_void,
    buffer_count: c_uint,
    width: c_uint,
    height: c_uint,
    new_format: DXGI_FORMAT,
    swap_chain_flags: c_uint,
) -> HRESULT;
extern "C" fn IDXGISwapChain_ResizeBuffers(
    this: *mut c_void,
    buffer_count: c_uint,
    width: c_uint,
    height: c_uint,
    new_format: DXGI_FORMAT,
    swap_chain_flags: c_uint,
) -> HRESULT {
    // SAFETY: Transmute required for IL2CPP type conversion
    let orig_fn: ResizeBuffersFn = unsafe { std::mem::transmute(RESIZEBUFFERS_ADDR) };

    // Make sure that a swap chain has the right HWND first before initing the painter,
    // even if we don't use it here.
    if check_hwnd(this).0.is_null() {
        return orig_fn(this, buffer_count, width, height, new_format, swap_chain_flags);
    }

    let painter_mutex = match init_painter(this) {
        Ok(v) => v,
        Err(e) => {
            error!("{}", e);
            info!("Unhooking IDXGISwapChain hooks");

            let interceptor = &Hachimi::instance().interceptor;
            interceptor.unhook(IDXGISwapChain_Present as *const () as usize);
            interceptor.unhook(IDXGISwapChain_ResizeBuffers as *const () as usize);
            return orig_fn(this, buffer_count, width, height, new_format, swap_chain_flags);
        }
    };
    let mut painter = painter_mutex.lock().expect("lock poisoned");
    if this != painter.swap_chain().as_raw() {
        return orig_fn(this, buffer_count, width, height, new_format, swap_chain_flags);
    }

    painter.resize_buffers(|| orig_fn(this, buffer_count, width, height, new_format, swap_chain_flags))
}

static PAINTER: OnceCell<Mutex<D3D11Painter>> = OnceCell::new();
fn init_painter(p_swap_chain: *mut c_void) -> Result<&'static Mutex<D3D11Painter>, Error> {
    // TODO: use get_or_try_init again once it's in stable
    if let Some(painter) = PAINTER.get() {
        return Ok(painter);
    }

    let painter_mutex = {
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        let borrowed_swap_chain = unsafe { std::mem::ManuallyDrop::new(IDXGISwapChain::from_raw(p_swap_chain)) };
        let swap_chain = (*borrowed_swap_chain).clone();
        let painter = D3D11Painter::new(swap_chain)?; // The '?' works here!
        Mutex::new(painter)
    };

    /*
    PAINTER.get_or_try_init(|| {
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        let borrowed_swap_chain = unsafe {
            std::mem::ManuallyDrop::new(IDXGISwapChain::from_raw(p_swap_chain))
        };

        let swap_chain = (&*borrowed_swap_chain).clone();

        let painter = D3D11Painter::new(swap_chain)?;
        Ok(Mutex::new(painter))
    }) */

    Ok(PAINTER.get_or_init(|| painter_mutex))
}

unsafe extern "system" fn dummy_wnd_proc(hwnd: HWND, umsg: c_uint, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) }
}

fn get_swap_chain_vtable() -> Result<*mut usize, Error> {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let hmodule = unsafe { GetModuleHandleW(None).expect("unexpected failure") };

    // Create a fake swap chain to obtain the vtable
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(dummy_wnd_proc),
        lpszClassName: w!("Hachimi"),
        ..Default::default()
    };

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    if unsafe { RegisterClassExW(&wc) } == 0 {
        return Err(Error::RuntimeError("Failed to register dummy window class".to_owned()));
    }

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            wc.lpszClassName,
            w!(""),
            WS_DISABLED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(HINSTANCE(hmodule.0)),
            None,
        )
    }
    .map_err(|e| {
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe {
            let _ = UnregisterClassW(wc.lpszClassName, Some(HINSTANCE(hmodule.0)));
        }
        Error::RuntimeError(format!("Failed to create dummy window: {}", e))
    })?;

    if hwnd.0.is_null() {
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe {
            let _ = UnregisterClassW(wc.lpszClassName, Some(HINSTANCE(hmodule.0)));
        }
        return Err(Error::RuntimeError(
            "Failed to create dummy window (HWND is null)".to_string(),
        ));
    }

    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC {
        BufferCount: 1,
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferDesc: DXGI_MODE_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ..Default::default()
        },
        OutputWindow: hwnd,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            ..Default::default()
        },
        Windowed: true.into(),
        ..Default::default()
    };

    let mut p_swap_chain: Option<IDXGISwapChain> = None;
    let mut p_device: Option<ID3D11Device> = None;
    let mut feature_level = D3D_FEATURE_LEVEL::default();

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&swap_chain_desc),
            Some(&mut p_swap_chain),
            Some(&mut p_device),
            Some(&mut feature_level),
            None,
        )
    }
    .map_err(|e| {
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(wc.lpszClassName, Some(HINSTANCE(hmodule.0)));
        }
        Error::RuntimeError(e.to_string())
    })?;

    let swap_chain_vtable =
        p_swap_chain.map(|swap_chain| Interceptor::get_vtable_from_instance(swap_chain.as_raw() as _));
    std::mem::drop(p_device);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(wc.lpszClassName, Some(HINSTANCE(hmodule.0)));
    }

    Ok(swap_chain_vtable.unwrap_or(0 as _))
}

fn init_internal() -> Result<(), Error> {
    let swap_chain_vtable = get_swap_chain_vtable()?;
    let interceptor = &Hachimi::instance().interceptor;

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        info!("Hooking IDXGISwapChain::Present");
        PRESENT_ADDR = interceptor.hook_vtable(swap_chain_vtable, 8, IDXGISwapChain_Present as *const () as usize)?;

        info!("Hooking IDXGISwapChain::ResizeBuffers");
        RESIZEBUFFERS_ADDR = interceptor.hook_vtable(
            swap_chain_vtable,
            13,
            IDXGISwapChain_ResizeBuffers as *const () as usize,
        )?;
    }

    Ok(())
}

pub fn init() {
    std::thread::spawn(|| {
        init_internal().unwrap_or_else(|e| {
            error!("Init failed: {}", e);
        });
    });
}
