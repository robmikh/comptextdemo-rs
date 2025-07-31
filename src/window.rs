use std::sync::Once;

use windows::{
    UI::Composition::{Compositor, Desktop::DesktopWindowTarget},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::{LibraryLoader::GetModuleHandleW, WinRT::Composition::ICompositorDesktopInterop},
        UI::WindowsAndMessaging::{
            AdjustWindowRectEx, CREATESTRUCTW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
            GWLP_USERDATA, GetWindowLongPtrW, IDC_ARROW, LoadCursorW, PostQuitMessage,
            RegisterClassW, SW_SHOW, SetWindowLongPtrW, ShowWindow, WM_DESTROY, WM_NCCREATE,
            WNDCLASSW, WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
        },
    },
    core::{HSTRING, Interface, PCWSTR, Result, w},
};

static REGISTER_WINDOW_CLASS: Once = Once::new();
const WINDOW_CLASS_NAME: PCWSTR = w!("comptextdemo.Window");

pub struct Window {
    handle: HWND,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Box<Self>> {
        let instance = HINSTANCE(unsafe { GetModuleHandleW(None)? }.0);
        REGISTER_WINDOW_CLASS.call_once(|| {
            let class = WNDCLASSW {
                hCursor: unsafe { LoadCursorW(None, IDC_ARROW).ok().unwrap() },
                hInstance: instance,
                lpszClassName: WINDOW_CLASS_NAME.into(),
                lpfnWndProc: Some(Self::wnd_proc),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });

        let width = width as i32;
        let height = height as i32;
        let window_ex_style = WS_EX_NOREDIRECTIONBITMAP;
        let window_style = WS_OVERLAPPEDWINDOW;

        let (adjusted_width, adjusted_height) = {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            unsafe {
                AdjustWindowRectEx(&mut rect, window_style, false, window_ex_style)?;
            }
            (rect.right - rect.left, rect.bottom - rect.top)
        };

        let mut result = Box::new(Self {
            handle: HWND(std::ptr::null_mut()),
        });

        let window = unsafe {
            CreateWindowExW(
                window_ex_style,
                WINDOW_CLASS_NAME,
                &HSTRING::from(title),
                window_style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                adjusted_width,
                adjusted_height,
                None,
                None,
                Some(instance),
                Some(result.as_mut() as *mut _ as _),
            )?
        };
        let _ = unsafe { ShowWindow(window, SW_SHOW) };

        Ok(result)
    }

    pub fn handle(&self) -> HWND {
        self.handle
    }

    pub fn create_window_target(
        &self,
        compositor: &Compositor,
        is_topmost: bool,
    ) -> Result<DesktopWindowTarget> {
        let compositor_desktop: ICompositorDesktopInterop = compositor.cast()?;
        unsafe { compositor_desktop.CreateDesktopWindowTarget(self.handle(), is_topmost) }
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                return LRESULT(0);
            }
            _ => {}
        }
        unsafe { DefWindowProcW(self.handle, message, wparam, lparam) }
    }

    unsafe extern "system" fn wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            if message == WM_NCCREATE {
                let cs = lparam.0 as *const CREATESTRUCTW;
                let this = (*cs).lpCreateParams as *mut Self;
                (*this).handle = window;

                SetWindowLongPtrW(window, GWLP_USERDATA, this as _);
            } else {
                let this = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Self;

                if let Some(this) = this.as_mut() {
                    return this.message_handler(message, wparam, lparam);
                }
            }
            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}
