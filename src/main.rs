use std::io::{BufRead, Read, Seek};

// #![windows_subsystem = "windows"]

use anyhow;
use windows_sys::{self as lit, Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW};

pub mod win32 {
    pub use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
    pub use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    pub use windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcA;
    pub use windows_sys::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_OWNDC, CS_VREDRAW, WNDCLASSEXW,
    };

    pub use windows_sys::Win32::{
        Foundation::{GetLastError, POINT},
        System::{
            Diagnostics::Debug::{
                FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
            },
            Threading::Sleep,
        },
        UI::{
            Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL},
            WindowsAndMessaging::{
                DispatchMessageW, PeekMessageW, MB_ICONEXCLAMATION, MSG, PM_REMOVE, WM_CLOSE,
                WM_DESTROY, WM_HOTKEY, WM_QUIT,
            },
        },
    };

    pub use windows_sys::Win32::{
        Foundation::RECT,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DestroyWindow, MessageBoxA, RegisterClassExW,
            SetWindowPos, ShowWindow, UnregisterClassW, HWND_TOPMOST, MB_OK, SWP_NOMOVE,
            SWP_NOSIZE, SW_SHOW, WS_CAPTION, WS_EX_TOPMOST, WS_GROUP, WS_SIZEBOX, WS_SYSMENU,
        },
    };
}

static mut IS_RUNNING: bool = true;

extern "system" fn wndproc(
    window: win32::HWND,
    message: u32,
    wparam: win32::WPARAM,
    lparam: win32::LPARAM,
) -> win32::LRESULT {
    unsafe {
        match message {
            win32::WM_DESTROY | win32::WM_CLOSE => {
                IS_RUNNING = false;
                println!("WM_DESTROY");
                return 0;
            }
            _ => win32::DefWindowProcA(window, message, wparam, lparam),
        }
    }
}

fn format_win32_error(error_code: u32) -> String {
    let mut message_buffer: [u16; 512] = [0; 512];
    unsafe {
        let size = win32::FormatMessageW(
            win32::FORMAT_MESSAGE_FROM_SYSTEM | win32::FORMAT_MESSAGE_IGNORE_INSERTS,
            std::ptr::null_mut(),
            error_code,
            0,
            message_buffer.as_mut_ptr() as _,
            message_buffer.len() as u32,
            std::ptr::null_mut(),
        );
        if size == 0 {
            return format!("Unknown error (code: {})", error_code);
        }
        return String::from_utf16_lossy(&message_buffer[..size as usize]);
    }
}

struct Window {
    h_window: win32::HWND,
    h_instance: win32::HINSTANCE,
}

impl Window {
    const CLASS_NAME: *const u16 = lit::w!("CLASSWINs");

    pub fn new(instance: win32::HINSTANCE, width: i32, height: i32) -> anyhow::Result<Self> {
        let winex = win32::WNDCLASSEXW {
            cbSize: std::mem::size_of::<win32::WNDCLASSEXW>() as u32,
            style: win32::CS_OWNDC | win32::CS_VREDRAW | win32::CS_HREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null_mut(),
            lpszClassName: Window::CLASS_NAME,
            hIconSm: 0,
        };
        unsafe {
            if win32::RegisterClassExW(&winex) == 0 {
                let error_code = win32::GetLastError();
                return Err(anyhow::anyhow!(
                    "Err RegisterClassExW failed {}\0",
                    format_win32_error(error_code)
                ));
            }
        }

        const OFFSET: i32 = 100;
        let mut rect = win32::RECT {
            left: OFFSET,
            top: OFFSET,
            right: OFFSET + width,
            bottom: OFFSET + height,
        };
        let style = WS_OVERLAPPEDWINDOW | WS_VISIBLE;// win32::WS_CAPTION | win32::WS_GROUP | win32::WS_SYSMENU | win32::WS_SIZEBOX;
        unsafe {
            if win32::AdjustWindowRect(&mut rect, style, 0) == 0 {
                let error_code = win32::GetLastError();
                return Err(anyhow::anyhow!(
                    "Err AdjustWindowRect failed {}\0",
                    format_win32_error(error_code)
                ));
            }
        }

        let window = unsafe {
            Self {
                h_window: win32::CreateWindowExW(
                    win32::WS_EX_TOPMOST,
                    Window::CLASS_NAME,
                    lit::w!("Ctrl+Alt+X. Close When Done"),
                    style,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    0,
                    0,
                    instance,
                    std::ptr::null_mut(),
                ),
                h_instance: instance,
            }
        };

        if window.h_window == 0 {
            let error_code = unsafe { win32::GetLastError() };
            return Err(anyhow::anyhow!(
                "Err CreateWindowExW failed: {}\0",
                format_win32_error(error_code)
            ));
        }

        unsafe {
            win32::SetWindowPos(
                window.h_window,
                win32::HWND_TOPMOST,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                win32::SWP_NOMOVE | win32::SWP_NOSIZE,
            );
            let _ = win32::ShowWindow(window.h_window, win32::SW_SHOW);
        };

        return Ok(window);
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            win32::UnregisterClassW(Window::CLASS_NAME, self.h_instance);
            win32::DestroyWindow(self.h_window);
        }
    }
}
const SKILINE_NUMBER_SIZE: usize = 8;

fn main() {
    let instance = unsafe { win32::GetModuleHandleW(std::ptr::null()) };
    debug_assert!(instance != 0);

    let window = match Window::new(instance, 330, 100) {
        Ok(k) => k,
        Err(e) => unsafe {
            let err = e.to_string();
            win32::MessageBoxA(
                0,
                err.as_str().as_ptr(),
                lit::s!("Window Error"),
                win32::MB_OK | win32::MB_ICONEXCLAMATION,
            );
            return;
        },
    };

    unsafe {
        // Register Ctrl+Alt+X as a global hotkey
        if win32::RegisterHotKey(
            window.h_window,
            1,
            win32::MOD_CONTROL | win32::MOD_ALT,
            b'X' as u32,
        ) == 0
        {
            win32::MessageBoxA(
                0,
                lit::s!("Unable to register a global hotkey"),
                lit::s!("RegisterHotKey Error"),
                win32::MB_OK | win32::MB_ICONEXCLAMATION,
            );
            return;
        }
    }

    let mut file_line = match std::fs::File::open("./skipline.dat") {
        Ok(f) => f,
        Err(e) => {
            let err = e.to_string();
            unsafe {
                win32::MessageBoxA(
                    0,
                    err.as_str().as_ptr() as _,
                    lit::s!("File Open Error"),
                    win32::MB_OK | win32::MB_ICONEXCLAMATION,
                )
            };
            return;
        }
    };

    let mut file_line_buff = [b'0'; SKILINE_NUMBER_SIZE];
    if let Err(e) = file_line.read_exact(&mut file_line_buff) {
        let err = e.to_string();
        unsafe {
            win32::MessageBoxA(
                0,
                err.as_str().as_ptr() as _,
                lit::s!("File Read Error"),
                win32::MB_OK | win32::MB_ICONEXCLAMATION,
            )
        };
    }

    let mut linse_to_skip = parse_lines_to_skip(&file_line_buff);

    let mut ifile = match std::fs::File::open("./words.txt") {
        Ok(f) => std::io::BufReader::new(f),
        Err(e) => {
            let err = e.to_string();
            unsafe {
                win32::MessageBoxA(
                    0,
                    err.as_str().as_ptr() as _,
                    lit::s!("File Open Error"),
                    win32::MB_OK | win32::MB_ICONEXCLAMATION,
                )
            };
            return;
        }
    };
    let mut buffer = String::with_capacity(128);
    {
        let mut buffer = [0u8; 128];
        for _ in 0..linse_to_skip {
            ifile.read(&mut buffer).expect("to skip it");
        }
    }

    let mut use_clipboard = false;

    let mut args = std::env::args();
    let _ = dbg!(args.next());
    if let Some(arg) = args.next() {
        if arg == "clip" {
            use_clipboard = true;
        }
    }

    while unsafe { IS_RUNNING } {
        poll_event(window.h_window, &mut ifile, &mut buffer, &mut linse_to_skip, use_clipboard);

        unsafe {
            win32::Sleep(38);
        }
    }

    unsafe {
        win32::UnregisterHotKey(window.h_window, 1);
    }
}

fn poll_event(
    h_window: isize,
    ifile: &mut std::io::BufReader<std::fs::File>,
    mut buffer: &mut String,
    linse_to_skip: &mut u64,
    use_clipboard: bool,
) {
    let mut msg = unsafe {std::mem::zeroed()};
    while unsafe { win32::PeekMessageW(&mut msg, h_window, 0, 0, win32::PM_REMOVE) != 0 } {
        if msg.message == win32::WM_QUIT {
            unsafe {
                IS_RUNNING = false;
            }
            return;
        }

        if msg.message == win32::WM_HOTKEY {
            if msg.wParam == 1 {
                // Sleep(300);
                buffer.clear();
                let size = ifile.read_line(&mut buffer).expect("to read successfully");
                if size == 0 {
                    *linse_to_skip = 0;
                    ifile.rewind().expect("to rewind to the beginig of word.txt");
                } else {
                    let line_slice = &buffer[..size];
                    dbg!(line_slice);
                }
                break;
            }
        }

        unsafe {
            win32::DispatchMessageW(&msg);
        }
    }
}

fn parse_lines_to_skip(file_line_buff: &[u8]) -> u64 {
    let mut result: u64 = 0;
    for &digit in file_line_buff {
        if digit >= b'0' && digit <= b'9' {
            dbg!(digit - b'0');
            result = result * 10 + (digit - b'0') as u64;
        }
    }
    return result;
}
