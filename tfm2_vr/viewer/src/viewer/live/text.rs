//! 글자 한 줄을 그림으로(GDI). 흰 글자 + 선택적 검은 테두리.
use image::{Rgba, RgbaImage};
use std::ffi::c_void;
use windows::{core::w, Win32::{Foundation::{COLORREF, RECT, SIZE}, Graphics::Gdi::*}};

pub fn render(text: &str, px: i32, outline: bool) -> Option<RgbaImage> {
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() { return None; }
        let font = CreateFontW(-px, 0, 0, 0, 700, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, 0, w!("Malgun Gothic"));
        let old_font = SelectObject(dc, HGDIOBJ(font.0));
        let mut wide: Vec<u16> = text.encode_utf16().take(64).collect();
        let mut size = SIZE::default();
        let _ = GetTextExtentPoint32W(dc, &wide, &mut size);
        let pad = 4;
        let (w, h) = (size.cx + pad * 2, size.cy + pad * 2);
        let mut out = None;
        if (1..=2000).contains(&w) && (1..=240).contains(&h) {
            let info = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() }, ..Default::default() };
            let mut bits: *mut c_void = std::ptr::null_mut();
            if let Ok(bitmap) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                let old = SelectObject(dc, HGDIOBJ(bitmap.0));
                SetBkMode(dc, TRANSPARENT);
                SetTextColor(dc, COLORREF(0x00ffffff));
                let mut rect = RECT { left: pad, top: pad, right: w, bottom: h };
                DrawTextW(dc, &mut wide, &mut rect, DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX);
                let _ = GdiFlush();
                let data = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
                let cover = |x: i32, y: i32| if x < 0 || y < 0 || x >= w || y >= h { 0u8 } else { data[((y * w + x) * 4) as usize + 1] };
                let mut im = RgbaImage::new(w as u32, h as u32);
                for y in 0..h { for x in 0..w {
                    let c = cover(x, y);
                    let mut edge = 0u8;
                    if outline { for dy in -2..=2 { for dx in -2i32..=2 { if dx * dx + dy * dy <= 5 { edge = edge.max(cover(x + dx, y + dy)); } } } }
                    let a = c.max(edge);
                    // 테두리는 검정, 글자는 흰색(색은 인스턴스가 곱한다). 프리멀티가 아닌 값으로 둔다.
                    let v = if a == 0 { 0 } else { ((c as u32 * 255) / a as u32) as u8 };
                    im.put_pixel(x as u32, y as u32, Rgba([v, v, v, a]));
                } }
                out = Some(im);
                SelectObject(dc, old);
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
            }
        }
        SelectObject(dc, old_font);
        let _ = DeleteObject(HGDIOBJ(font.0));
        let _ = DeleteDC(dc);
        out
    }
}
