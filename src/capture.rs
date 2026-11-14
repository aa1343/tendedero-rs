use std::path::PathBuf;
use chrono::Local;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub struct CaptureResult {
    pub file_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
    pub thumbnail_data: String,
}

pub fn capture_primary_screen(save_dir: &PathBuf) -> Result<CaptureResult, String> {
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        if width <= 0 || height <= 0 {
            return Err("Invalid screen dimensions".to_string());
        }

        let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
        let old_obj = SelectObject(hdc_mem, hbitmap);

        let success = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

        if success.is_err() {
            SelectObject(hdc_mem, old_obj);
            let _ = DeleteObject(hbitmap);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err("BitBlt failed to copy desktop bitmap".to_string());
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // negative for top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }; 1],
        };

        let mut bgra_buffer: Vec<u8> = vec![0; (width * height * 4) as usize];

        let scanlines = GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            height as u32,
            Some(bgra_buffer.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Clean up GDI handles immediately
        SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

        if scanlines == 0 {
            return Err("GetDIBits failed".to_string());
        }

        // Convert BGRA to RGBA in place
        let mut rgba_buffer = bgra_buffer;
        for chunk in rgba_buffer.chunks_exact_mut(4) {
            let b = chunk[0];
            let r = chunk[2];
            chunk[0] = r;
            chunk[2] = b;
        }

        let file_name = format!("截图_{}.png", Local::now().format("%Y%m%d_%H%M%S"));
        let target_path = save_dir.join(&file_name);

        let img = image::RgbaImage::from_raw(width as u32, height as u32, rgba_buffer)
            .ok_or_else(|| "Failed to construct RgbaImage".to_string())?;

        img.save_with_format(&target_path, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to save screenshot image: {}", e))?;

        let size_bytes = std::fs::metadata(&target_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        // Generate downscaled thumbnail for ultra-responsive UI
        let thumb_height = 240u32;
        let thumb_width = (width as u32 * thumb_height) / height as u32;
        let thumb = image::imageops::resize(
            &img,
            thumb_width,
            thumb_height,
            image::imageops::FilterType::Triangle,
        );

        let mut thumb_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut thumb_bytes);
        let _ = thumb.write_to(&mut cursor, image::ImageFormat::Png);

        use base64::Engine;
        let base64_str = base64::engine::general_purpose::STANDARD.encode(&thumb_bytes);
        let thumbnail_data = format!("data:image/png;base64,{}", base64_str);

        Ok(CaptureResult {
            file_path: target_path,
            width: width as u32,
            height: height as u32,
            size_bytes,
            thumbnail_data,
        })
    }
}
