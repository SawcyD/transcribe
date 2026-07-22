use std::{ffi::c_void, io::Cursor};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use png::{BitDepth, ColorType, Encoder};
use serde::{Deserialize, Serialize};
use windows::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, GetMonitorInfoW, MonitorFromPoint, ReleaseDC, SelectObject, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HGDIOBJ, MONITORINFO,
        MONITOR_DEFAULTTONEAREST, SRCCOPY,
    },
    UI::WindowsAndMessaging::GetCursorPos,
};

use crate::{errors::AppError, insertion};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenContext {
    pub application: Option<String>,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub monitor: ScreenRect,
    pub screenshot_data_url: String,
    pub screenshot_width: u32,
    pub screenshot_height: u32,
}

pub fn capture_context() -> Result<ScreenContext, AppError> {
    let target = insertion::capture_active_target()?;
    let mut cursor = POINT::default();
    let monitor_rect = unsafe {
        GetCursorPos(&mut cursor).map_err(|error| AppError::Windows(error.to_string()))?;
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            return Err(AppError::Windows(
                "the cursor monitor is unavailable".into(),
            ));
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err(AppError::Windows(
                "monitor information is unavailable".into(),
            ));
        }
        info.rcMonitor
    };

    let width = (monitor_rect.right - monitor_rect.left).max(1) as u32;
    let height = (monitor_rect.bottom - monitor_rect.top).max(1) as u32;
    let (pixels, screenshot_width, screenshot_height) =
        capture_monitor(monitor_rect, width, height)?;
    let screenshot_data_url = encode_png_data_url(&pixels, screenshot_width, screenshot_height)?;

    Ok(ScreenContext {
        application: target.application_name,
        process_name: target.process_name,
        window_title: target.window_title,
        cursor_x: cursor.x,
        cursor_y: cursor.y,
        monitor: ScreenRect {
            left: monitor_rect.left,
            top: monitor_rect.top,
            width,
            height,
        },
        screenshot_data_url,
        screenshot_width,
        screenshot_height,
    })
}

fn capture_monitor(rect: RECT, width: u32, height: u32) -> Result<(Vec<u8>, u32, u32), AppError> {
    let (capture_width, capture_height) = scaled_dimensions(width, height, 1600);
    let (raw_pixels, raw_width, raw_height) = unsafe { capture_bgra(rect, width, height)? };
    let rgba = bgra_to_rgba(&raw_pixels);
    let resized = resize_rgba(&rgba, raw_width, raw_height, capture_width, capture_height);
    Ok((resized, capture_width, capture_height))
}

unsafe fn capture_bgra(
    rect: RECT,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, u32, u32), AppError> {
    let desktop = GetDC(None);
    if desktop.is_invalid() {
        return Err(AppError::Windows(
            "desktop capture device is unavailable".into(),
        ));
    }
    let memory = CreateCompatibleDC(Some(desktop));
    if memory.is_invalid() {
        let _ = ReleaseDC(None, desktop);
        return Err(AppError::Windows(
            "capture memory device is unavailable".into(),
        ));
    }
    let bitmap = CreateCompatibleBitmap(desktop, width as i32, height as i32);
    if bitmap.is_invalid() {
        let _ = DeleteDC(memory);
        let _ = ReleaseDC(None, desktop);
        return Err(AppError::Windows("capture bitmap is unavailable".into()));
    }
    let previous = SelectObject(memory, HGDIOBJ(bitmap.0));
    let copy_result = BitBlt(
        memory,
        0,
        0,
        width as i32,
        height as i32,
        Some(desktop),
        rect.left,
        rect.top,
        SRCCOPY | CAPTUREBLT,
    );
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let copied = if copy_result.is_ok() {
        GetDIBits(
            memory,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        )
    } else {
        0
    };
    let _ = SelectObject(memory, previous);
    let _ = DeleteObject(HGDIOBJ(bitmap.0));
    let _ = DeleteDC(memory);
    let _ = ReleaseDC(None, desktop);
    if copied == 0 {
        return Err(AppError::Windows(
            "screen pixels could not be captured".into(),
        ));
    }
    Ok((pixels, width, height))
}

fn encode_png_data_url(pixels: &[u8], width: u32, height: u32) -> Result<String, AppError> {
    let mut encoded = Vec::new();
    let mut encoder = Encoder::new(Cursor::new(&mut encoded), width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| AppError::Windows(format!("screenshot encoding failed: {error}")))?;
    writer
        .write_image_data(pixels)
        .map_err(|error| AppError::Windows(format!("screenshot encoding failed: {error}")))?;
    drop(writer);
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(encoded)
    ))
}

fn bgra_to_rgba(pixels: &[u8]) -> Vec<u8> {
    let mut rgba = pixels.to_vec();
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    rgba
}

fn scaled_dimensions(width: u32, height: u32, max_dimension: u32) -> (u32, u32) {
    let largest = width.max(height);
    if largest <= max_dimension {
        return (width, height);
    }
    let scale = max_dimension as f32 / largest as f32;
    (
        (width as f32 * scale).round() as u32,
        (height as f32 * scale).round() as u32,
    )
}

fn resize_rgba(
    pixels: &[u8],
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
) -> Vec<u8> {
    if width == target_width && height == target_height {
        return pixels.to_vec();
    }
    let mut resized = vec![0u8; target_width as usize * target_height as usize * 4];
    for y in 0..target_height {
        let source_y = (y as u64 * height as u64 / target_height as u64) as u32;
        for x in 0..target_width {
            let source_x = (x as u64 * width as u64 / target_width as u64) as u32;
            let source_index = ((source_y * width + source_x) * 4) as usize;
            let target_index = ((y * target_width + x) * 4) as usize;
            resized[target_index..target_index + 4]
                .copy_from_slice(&pixels[source_index..source_index + 4]);
        }
    }
    resized
}

#[cfg(test)]
mod tests {
    use super::{resize_rgba, scaled_dimensions};

    #[test]
    fn scales_large_monitors_without_changing_aspect_ratio() {
        assert_eq!(scaled_dimensions(3840, 2160, 1600), (1600, 900));
    }

    #[test]
    fn resizes_rgba_pixels() {
        let source = vec![255u8; 4 * 2 * 2];
        assert_eq!(resize_rgba(&source, 2, 2, 1, 1), vec![255u8; 4]);
    }
}
