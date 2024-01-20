use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    BeginPath, BS_SOLID, CloseFigure, CreateSolidBrush, EndPath, ExtCreatePen, FillPath, HBRUSH,
    HDC, HGDIOBJ, HPEN, LineTo, LOGBRUSH, MoveToEx, PEN_STYLE, PS_ENDCAP_SQUARE, PS_GEOMETRIC,
    PS_SOLID, SelectObject, StrokePath,
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

pub(crate) fn ext_create_pen(style: PEN_STYLE, width: u32, brush: &LOGBRUSH, dashes: Option<&[u32]>) -> HPEN {
    let pen = unsafe { ExtCreatePen(style, width, brush, dashes) };
    if pen.is_invalid() {
        panic!("failed to create pen");
    }
    pen
}

pub(crate) fn make_solid_square_endcap_pen(width: u32, color: COLORREF) -> HPEN {
    let brush = LOGBRUSH {
        lbColor: color,
        lbStyle: BS_SOLID,
        lbHatch: 0,
    };
    ext_create_pen(
        PS_GEOMETRIC | PS_SOLID | PS_ENDCAP_SQUARE,
        width,
        &brush,
        None,
    )
}

pub(crate) fn make_solid_brush(color: COLORREF) -> HBRUSH {
    let brush = unsafe { CreateSolidBrush(color) };
    if brush.is_invalid() {
        panic!("failed to create solid brush");
    }
    brush
}
