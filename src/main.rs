#![windows_subsystem = "windows"]

use std::io::{BufRead, Read, Seek, Write};

use anyhow;

pub mod win32 {
    pub use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
    pub use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    pub use windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcA;
    pub use windows_sys::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_OWNDC, CS_VREDRAW, WNDCLASSW,
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
                WM_DESTROY, WM_HOTKEY, WM_PAINT, WM_QUIT,
            },
        },
    };
    pub use windows_sys::{
        s, w,
        Win32::{
            System::{
                DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
                Memory::{GlobalLock, GlobalUnlock},
                Ole::CF_TEXT,
            },
            UI::{
                Input::KeyboardAndMouse::{
                    MapVirtualKeyW, SendInput, VkKeyScanA, VkKeyScanW, INPUT, INPUT_0,
                    INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC,
                },
                WindowsAndMessaging::WM_KEYUP,
            },
        },
    };

    pub use windows_sys::Win32::{
        Foundation::RECT,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DestroyWindow, MessageBoxA, PostQuitMessage,
            RegisterClassW, SetWindowPos, ShowWindow, UnregisterClassW, HWND_TOPMOST, MB_OK,
            SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, WS_CAPTION, WS_EX_TOPMOST, WS_GROUP, WS_SIZEBOX,
            WS_SYSMENU,
        },
    };
}

static mut IS_RUNNING: bool = true;

#[inline(always)]
fn is_runnig() -> bool {
    unsafe { IS_RUNNING }
}

#[inline(always)]
fn set_is_running(to: bool) {
    unsafe { IS_RUNNING = to };
}

extern "system" fn wndproc(
    window: win32::HWND,
    message: u32,
    wparam: win32::WPARAM,
    lparam: win32::LPARAM,
) -> win32::LRESULT {
    unsafe {
        match message {
            win32::WM_DESTROY | win32::WM_CLOSE => {
                set_is_running(false);
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


const NAME: &'static str = "CLASSWIN";
static WINDOW_CLASS_NAME: &'static [u16; 9] = wider_string();



impl Window {

    pub fn new(instance: win32::HINSTANCE, width: i32, height: i32) -> anyhow::Result<Self> {
        let wc = win32::WNDCLASSW {
            style: win32::CS_OWNDC | win32::CS_VREDRAW | win32::CS_HREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null_mut(),
            lpszClassName: WINDOW_CLASS_NAME.as_ptr(),
        };
        unsafe {
            if win32::RegisterClassW(&wc) == 0 {
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
        let style = win32::WS_CAPTION | win32::WS_GROUP | win32::WS_SYSMENU | win32::WS_SIZEBOX;
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
                    WINDOW_CLASS_NAME.as_ptr(),
                    win32::s!("Ctrl+Alt+X. Close When Done") as _,
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
            win32::UnregisterClassW(WINDOW_CLASS_NAME.as_ptr(), self.h_instance);
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
                win32::s!("Window Error"),
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
                win32::s!("Unable to register a global hotkey"),
                win32::s!("RegisterHotKey Error"),
                win32::MB_OK | win32::MB_ICONEXCLAMATION,
            );
            return;
        }
    }

    let mut file_line =
        match std::fs::OpenOptions::new().read(true).write(true).open("./skipline.dat") {
            Ok(f) => f,
            Err(e) => {
                let err = e.to_string();
                unsafe {
                    win32::MessageBoxA(
                        0,
                        err.as_str().as_ptr() as _,
                        win32::s!("File Open Error"),
                        win32::MB_OK | win32::MB_ICONEXCLAMATION,
                    )
                };
                return;
            }
        };

    let mut lines_to_skip = {
        let mut file_line_buff = [b'0'; SKILINE_NUMBER_SIZE];
        if let Err(e) = file_line.read_exact(&mut file_line_buff) {
            let err = e.to_string();
            unsafe {
                win32::MessageBoxA(
                    0,
                    err.as_str().as_ptr() as _,
                    win32::s!("File Read Error"),
                    win32::MB_OK | win32::MB_ICONEXCLAMATION,
                )
            };
        }
        // eprintln!("{:?}", std::str::from_utf8(&file_line_buff));
        parse_lines_to_skip(&file_line_buff)
    };
    // dbg!(&lines_to_skip);
    let mut ifile = match std::fs::File::open("./words.txt") {
        Ok(f) => std::io::BufReader::new(f),
        Err(e) => {
            let err = e.to_string();
            unsafe {
                win32::MessageBoxA(
                    0,
                    err.as_str().as_ptr() as _,
                    win32::s!("File Open Error"),
                    win32::MB_OK | win32::MB_ICONEXCLAMATION,
                )
            };
            return;
        }
    };
    let mut buffer = String::with_capacity(128);
    {
        let mut buffer = Vec::with_capacity(128);
        for _ in 0..lines_to_skip {
            buffer.clear();
            let _ = ifile.read_until(b'\n', &mut buffer);
        }
    }

    let mut use_clipboard = false;

    let mut args = std::env::args();
    let _ = args.next();
    if let Some(arg) = args.next() {
        if arg == "clip" {
            use_clipboard = true;
        }
    }

    while is_runnig() {
        poll_event(window.h_window, &mut ifile, &mut buffer, &mut lines_to_skip, use_clipboard);
        unsafe { win32::Sleep(38) };
    }

    unsafe { win32::UnregisterHotKey(window.h_window, 1) };

    let skipline_array = u64_to_array::<8>(lines_to_skip);
    let _ = file_line.seek(std::io::SeekFrom::Start(0));
    let _ = file_line.write_all(&skipline_array);
}

fn u64_to_array<const N: usize>(mut num: u64) -> [u8; N] {
    debug_assert!(N <= 8);
    let mut buf = [0_u8; N];

    for ele in buf.iter_mut().rev() {
        *ele = b'0' + (num % 10) as u8;
        num /= 10;

        if num == 0 {
            break;
        }
    }

    let mut res = [0; N];
    let mut i = 0;
    for ele in buf {
        if ele != 0 {
            res[i] = ele;
            i += 1;
        }
    }

    return res;
}

fn poll_event(
    h_window: isize,
    ifile: &mut std::io::BufReader<std::fs::File>,
    buffer: &mut String,
    linse_to_skip: &mut u64,
    use_clipboard: bool,
) {
    let mut msg = unsafe { std::mem::zeroed() };
    while unsafe { win32::PeekMessageW(&mut msg, h_window, 0, 0, win32::PM_REMOVE) != 0 } {
        if msg.message == win32::WM_QUIT {
            set_is_running(false);
            return;
        }

        if msg.message == win32::WM_HOTKEY {
            if msg.wParam == 1 {
                unsafe { win32::Sleep(400) };
                buffer.clear();
                let size = ifile.read_line(buffer).expect("to read successfully");
                if size == 0 {
                    *linse_to_skip = 0;
                    ifile.rewind().expect("to rewind to the beginig of word.txt");
                } else {
                    let line_slice = &buffer[..size - 1];
                    // dbg!(*linse_to_skip);
                    *linse_to_skip += 1;
                    // dbg!(line_slice);
                    if use_clipboard {
                        set_clipboard_string(line_slice);
                    } else {
                        type_out_characters(line_slice);
                    }
                }
                break;
            }
        }
        unsafe { win32::DispatchMessageW(&msg) };
    }
}

fn lobyte(w: u64) -> u8 {
    (w & 0xff) as u8
}

fn type_out_characters(line_slice: &str) {
    for &chr in line_slice.as_bytes() {
        let vkey = unsafe { win32::VkKeyScanW(chr as u16) };
        if vkey <= -1 {
            continue;
        }

        let wvk = lobyte(vkey as u64) as u16;
        let mut keyboard_input = win32::KEYBDINPUT {
            wVk: wvk,
            wScan: unsafe { win32::MapVirtualKeyW(wvk as u32, win32::MAPVK_VK_TO_VSC) as u16 },
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };
        let mut input = win32::INPUT {
            r#type: win32::INPUT_KEYBOARD,
            Anonymous: win32::INPUT_0 { ki: keyboard_input },
        };

        unsafe {
            win32::SendInput(1, &input, std::mem::size_of::<win32::INPUT>() as i32);
            keyboard_input.dwFlags = win32::KEYEVENTF_KEYUP;
            input.Anonymous = win32::INPUT_0 { ki: keyboard_input };
        }
    }
}

fn set_clipboard_string(line_slice: &str) {
    let bytes = line_slice.as_bytes();
    unsafe {
        if win32::OpenClipboard(0) != 0 {
            win32::EmptyClipboard();

            let size = line_slice.len() + 1;
            let h_mem = windows_sys::Win32::System::Memory::GlobalAlloc(
                windows_sys::Win32::System::Memory::GMEM_MOVEABLE,
                size,
            );
            if h_mem != std::ptr::null_mut() {
                let mem_data = win32::GlobalLock(h_mem) as *mut u8;
                for i in 0..line_slice.len() {
                    let &byte = bytes.get_unchecked(i);
                    *mem_data.offset(i as isize) = byte;
                }
                win32::GlobalUnlock(mem_data as _);

                win32::SetClipboardData(win32::CF_TEXT as u32, h_mem as isize);
            }

            win32::CloseClipboard();
        } else {
            win32::MessageBoxA(
                0,
                win32::s!("Unable to copy data to clipboard"),
                win32::s!("Clipboard Error"),
                win32::MB_OK | win32::MB_ICONEXCLAMATION,
            );
        }
    }
}

fn parse_lines_to_skip(file_line_buff: &[u8]) -> u64 {
    let mut result: u64 = 0;
    for &digit in file_line_buff {
        if digit >= b'0' && digit <= b'9' {
            result = result * 10 + (digit - b'0') as u64;
        }
    }
    return result;
}

const fn wider_string() -> &'static [u16; 9] {
    const INPUT: &[u8] = NAME.as_bytes();
    const OUTPUT_LEN: usize = windows_sys::core::utf16_len(INPUT) + 1;
    const OUTPUT: &[u16; OUTPUT_LEN] = {
        let mut buffer = [0; OUTPUT_LEN];
        let mut input_pos = 0;
        let mut output_pos = 0;
        while let Some((mut code_point, new_pos)) = windows_sys::core::decode_utf8_char(INPUT, input_pos) {
            input_pos = new_pos;
            if code_point <= 0xffff {
                buffer[output_pos] = code_point as u16;
                output_pos += 1;
            } else {
                code_point -= 0x10000;
                buffer[output_pos] = 0xd800 + (code_point >> 10) as u16;
                output_pos += 1;
                buffer[output_pos] = 0xdc00 + (code_point & 0x3ff) as u16;
                output_pos += 1;
            }
        }
        &{ buffer }
    };
    return OUTPUT; 
}
