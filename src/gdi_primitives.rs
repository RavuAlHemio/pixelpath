use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    BeginPath, CloseFigure, CreatePen, EndPath, FillPath, GetStockObject, GET_STOCK_OBJECT_FLAGS,
    HDC, HGDIOBJ, HPEN, LineTo, MoveToEx, PEN_STYLE, SelectObject, StrokePath,
};

macro_rules! simple_gdi_func {
    ($name:ident, $func:ident, $panic_text:expr) => {
        pub(crate) fn $name(hdc: HDC) {
            let result = unsafe { $func(hdc) };
            if !result.as_bool() {
                panic!($panic_text);
            }
        }
    };
}
simple_gdi_func!(begin_path, BeginPath, "failed to begin path");
simple_gdi_func!(close_figure, CloseFigure, "failed to close figure");
simple_gdi_func!(end_path, EndPath, "failed to end path");
simple_gdi_func!(fill_path, FillPath, "failed to fill path");
simple_gdi_func!(stroke_path, StrokePath, "failed to stroke path");

pub(crate) fn select_object<O: Into<HGDIOBJ>>(hdc: HDC, object: O, description: &str) {
    let selected = unsafe { SelectObject(hdc, object.into()) };
    if selected.is_invalid() {
        panic!("failed to select {}", description);
    }
}

pub(crate) fn select_stock_object(hdc: HDC, stock_object: GET_STOCK_OBJECT_FLAGS, description: &str) {
    let stock_object = unsafe { GetStockObject(stock_object) };
    if stock_object.is_invalid() {
        panic!("failed to obtain {}", description);
    }
    select_object(hdc, stock_object, description);
}

pub(crate) fn move_to(hdc: HDC, x: i32, y: i32) {
    let moved = unsafe { MoveToEx(hdc, x, y, None) };
    if !moved.as_bool() {
        panic!("failed to move");
    }
}

pub(crate) fn line_to(hdc: HDC, x: i32, y: i32) {
    let lined = unsafe { LineTo(hdc, x, y) };
    if !lined.as_bool() {
        panic!("failed to add line");
    }
}

pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    let color =
        (r as u32)
        | (g as u32) << 8
        | (b as u32) << 16;
    COLORREF(color)
}

pub(crate) fn create_pen(style: PEN_STYLE, width: i32, color: COLORREF) -> HPEN {
    let pen = unsafe { CreatePen(style, width, color) };
    if pen.is_invalid() {
        panic!("failed to create pen");
    }
    pen
}
