mod gdi_primitives;


use std::fmt::Write as _;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BLACK_BRUSH, COLOR_WINDOW, EndPaint, FillRect, HBRUSH, HPEN, PAINTSTRUCT, PS_SOLID,
    RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{GetStartupInfoW, STARTUPINFOW};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_BACK, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_P, VK_RETURN, VK_RIGHT, VK_SPACE, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, CW_USEDEFAULT, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    MSG, PostQuitMessage, RegisterClassW, ShowWindow, SW_SHOWDEFAULT, TranslateMessage,
    WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_KEYDOWN, WM_PAINT, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

use crate::gdi_primitives::{
    begin_path, close_figure, create_pen, end_path, fill_path, line_to, move_to, rgb, select_object,
    select_stock_object, stroke_path,
};


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ApplicationState {
    pub cursor: Point,
    pub is_drawing: bool,
    pub paths: Vec<ClosedPath>,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ClosedPath {
    pub points: Vec<Point>,
}
impl ClosedPath {
    pub fn to_svg_elem(&self) -> String {
        let mut path = self.to_svg_path();
        path.insert_str(0, "<path d=\"");
        path.push_str("\" />");
        path
    }

    pub fn to_svg_path(&self) -> String {
        if self.points.len() == 0 {
            return String::with_capacity(0);
        }
        let mut ret = String::new();
        for (i, point) in self.points.iter().enumerate() {
            let prefix = if i == 0 { "M" } else { " L" };
            write!(ret, "{} {} {}", prefix, point.x, point.y).unwrap();
        }
        ret.push_str(" z");
        ret
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Point {
    pub x: i32,
    pub y: i32,
}


const LEFT_OFFSET: i32 = 100;
const TOP_OFFSET: i32 = 100;
const HORIZONTAL_FACTOR: i32 = 100;
const VERTICAL_FACTOR: i32 = 100;
const CROSSHAIR_LENGTH: i32 = 20;
const CROSSHAIR_THICKNESS: i32 = 4;
const RENDER_NUMERATOR: i32 = 1;
const RENDER_DENOMINATOR: i32 = 2;
const NOT_DRAWING_CROSSHAIR_COLOR: COLORREF = rgb(0x00, 0x00, 0xFF);
const DRAWING_CROSSHAIR_COLOR: COLORREF = rgb(0xFF, 0x00, 0x00);

static STATE: Lazy<Mutex<ApplicationState>> = Lazy::new(|| Mutex::new(ApplicationState::default()));
static DRAWING_CROSSHAIR_PEN: Lazy<HPEN> = Lazy::new(|| create_pen(
    PS_SOLID, CROSSHAIR_THICKNESS, DRAWING_CROSSHAIR_COLOR,
));
static NOT_DRAWING_CROSSHAIR_PEN: Lazy<HPEN> = Lazy::new(|| create_pen(
    PS_SOLID, CROSSHAIR_THICKNESS, NOT_DRAWING_CROSSHAIR_COLOR,
));


fn default_window_proc(handle: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(handle, message, wparam, lparam) }
}

fn redraw_window_and_return_zero(handle: HWND) -> LRESULT {
    unsafe { RedrawWindow(handle, None, None, RDW_INVALIDATE | RDW_UPDATENOW) };
    LRESULT(0)
}

unsafe extern "system" fn draw_window_proc(handle: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_CLOSE {
        unsafe { DestroyWindow(handle) }
            .expect("failed to destroy window");
        return LRESULT(0);
    } else if message == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
        return LRESULT(0);
    } else if message == WM_PAINT {
        paint_draw_window(handle);
        return LRESULT(0);
    } else if message == WM_KEYDOWN {
        let key: VIRTUAL_KEY = match wparam.0.try_into() {
            Ok(v) => VIRTUAL_KEY(v),
            Err(_) => {
                return default_window_proc(handle, message, wparam, lparam);
            }
        };

        let mut redraw = true;

        {
            let mut state_guard = STATE.lock().expect("failed to lock state");
            if key == VK_LEFT {
                state_guard.cursor.x -= HORIZONTAL_FACTOR;
                if state_guard.cursor.x < 0 {
                    state_guard.cursor.x = 0;
                }
            } else if key == VK_RIGHT {
                state_guard.cursor.x += HORIZONTAL_FACTOR;
            } else if key == VK_UP {
                state_guard.cursor.y -= VERTICAL_FACTOR;
                if state_guard.cursor.y < 0 {
                    state_guard.cursor.y = 0;
                }
            } else if key == VK_DOWN {
                state_guard.cursor.y += VERTICAL_FACTOR;
            } else if key == VK_SPACE {
                let cursor = state_guard.cursor;
                if !state_guard.is_drawing {
                    // start a new path
                    state_guard.paths.push(ClosedPath::default());
                }
                let last_path = state_guard.paths.last_mut().unwrap();

                // drop a point
                last_path.points.push(cursor);

                // we are certainly drawing now
                state_guard.is_drawing = true;
            } else if key == VK_BACK {
                // forget the last point
                if let Some(last_path) = state_guard.paths.last_mut() {
                    last_path.points.pop();
                }
            } else if key == VK_RETURN {
                // finish this path
                state_guard.is_drawing = false;
            } else if key == VK_ESCAPE {
                // stop drawing and forget the last path
                state_guard.paths.pop();
                state_guard.is_drawing = false;
            } else if key == VK_P {
                for path in &state_guard.paths {
                    println!("{}", path.to_svg_elem());
                }
            } else {
                // unknown key -- don't redraw
                redraw = false;
            }

            println!("{:?}", state_guard.paths);
        }

        if redraw {
            return redraw_window_and_return_zero(handle);
        } else {
            return LRESULT(0);
        }
    }

    default_window_proc(handle, message, wparam, lparam)
}


fn scale(value: i32) -> i32 {
    (value * RENDER_NUMERATOR) / RENDER_DENOMINATOR
}


fn paint_draw_window(handle: HWND) {
    let mut paint_struct = PAINTSTRUCT::default();
    let hdc = unsafe { BeginPaint(handle, &mut paint_struct) };
    if hdc.is_invalid() {
        return;
    }

    // paint background
    let background_brush: isize = (COLOR_WINDOW.0 + 1).try_into().unwrap();
    unsafe { FillRect(hdc, &paint_struct.rcPaint, HBRUSH(background_brush)) };

    {
        let state_guard = STATE.lock().expect("failed to lock state");

        // paint existing paths
        select_stock_object(hdc, BLACK_BRUSH, "black brush");

        for path in &state_guard.paths {
            if path.points.len() == 0 {
                continue;
            }

            begin_path(hdc);
            move_to(
                hdc,
                scale(LEFT_OFFSET + path.points[0].x),
                scale(TOP_OFFSET + path.points[0].y),
            );
            for point in path.points.iter().skip(1) {
                line_to(
                    hdc,
                    scale(LEFT_OFFSET + point.x),
                    scale(TOP_OFFSET + point.y),
                );
            }

            if state_guard.is_drawing {
                // also draw a line to the cursor
                line_to(
                    hdc,
                    scale(LEFT_OFFSET + state_guard.cursor.x),
                    scale(TOP_OFFSET + state_guard.cursor.y),
                );
            }

            close_figure(hdc);
            end_path(hdc);
            fill_path(hdc);
        }

        // paint cursor
        let pen = if state_guard.is_drawing { *DRAWING_CROSSHAIR_PEN } else { *NOT_DRAWING_CROSSHAIR_PEN };
        select_object(hdc, pen, "crosshair pen");

        // vertical line
        begin_path(hdc);
        move_to(
            hdc,
            scale(LEFT_OFFSET + state_guard.cursor.x),
            scale(TOP_OFFSET + state_guard.cursor.y - CROSSHAIR_LENGTH/2),
        );
        line_to(
            hdc,
            scale(LEFT_OFFSET + state_guard.cursor.x),
            scale(TOP_OFFSET + state_guard.cursor.y - CROSSHAIR_LENGTH/2 + CROSSHAIR_LENGTH),
        );
        end_path(hdc);
        stroke_path(hdc);

        // horizontal line
        begin_path(hdc);
        move_to(
            hdc,
            scale(LEFT_OFFSET + state_guard.cursor.x - CROSSHAIR_LENGTH/2),
            scale(TOP_OFFSET + state_guard.cursor.y),
        );
        line_to(
            hdc,
            scale(LEFT_OFFSET + state_guard.cursor.x - CROSSHAIR_LENGTH/2 + CROSSHAIR_LENGTH),
            scale(TOP_OFFSET + state_guard.cursor.y),
        );
        end_path(hdc);
        stroke_path(hdc);
    }

    unsafe { EndPaint(handle, &paint_struct) };
}


fn main() {
    let instance_module_handle = unsafe { GetModuleHandleW(None) }
        .expect("failed to obtain instance handle");
    let instance_handle = HINSTANCE::from(instance_module_handle);

    let mut startup_info = STARTUPINFOW::default();
    unsafe { GetStartupInfoW(&mut startup_info) };

    // register a class for our window
    let window_class_name = w!("PixelPathDrawWindow");
    let mut window_class = WNDCLASSW::default();
    window_class.lpfnWndProc = Some(draw_window_proc);
    window_class.hInstance = instance_handle;
    window_class.lpszClassName = w!("PixelPathDrawWindow");
    let registered = unsafe { RegisterClassW(&window_class) };
    if registered == 0 {
        panic!("failed to register window class: {}", windows::core::Error::from_win32());
    }

    // create the window
    let window_handle = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_class_name,
            w!("PixelPath"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            instance_handle,
            None,
        )
    };
    if window_handle.0 == 0 {
        panic!("failed to create window: {}", windows::core::Error::from_win32());
    }

    unsafe { ShowWindow(window_handle, SW_SHOWDEFAULT) };

    // main loop
    loop {
        let mut message = MSG::default();
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if !result.as_bool() {
            break;
        }
        unsafe { TranslateMessage(&message) };
        unsafe { DispatchMessageW(&message) };
    }
}
