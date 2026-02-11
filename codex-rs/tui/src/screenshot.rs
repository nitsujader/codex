use std::io;
use std::path::Path;

#[cfg(windows)]
pub(crate) fn capture_primary_monitor_png(path: &Path) -> io::Result<(u32, u32)> {
    use image::DynamicImage;
    use image::ImageFormat;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::BI_RGB;
    use windows::Win32::Graphics::Gdi::BITMAPINFO;
    use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
    use windows::Win32::Graphics::Gdi::BitBlt;
    use windows::Win32::Graphics::Gdi::CAPTUREBLT;
    use windows::Win32::Graphics::Gdi::CreateCompatibleBitmap;
    use windows::Win32::Graphics::Gdi::CreateCompatibleDC;
    use windows::Win32::Graphics::Gdi::DIB_RGB_COLORS;
    use windows::Win32::Graphics::Gdi::DeleteDC;
    use windows::Win32::Graphics::Gdi::DeleteObject;
    use windows::Win32::Graphics::Gdi::GetDC;
    use windows::Win32::Graphics::Gdi::GetDIBits;
    use windows::Win32::Graphics::Gdi::ReleaseDC;
    use windows::Win32::Graphics::Gdi::SRCCOPY;
    use windows::Win32::Graphics::Gdi::SelectObject;
    use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
    use windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN;
    use windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN;

    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if width <= 0 || height <= 0 {
        return Err(io::Error::other("failed to determine screen size"));
    }

    let screen_dc = unsafe { GetDC(HWND(std::ptr::null_mut())) };
    if screen_dc.0.is_null() {
        return Err(io::Error::last_os_error());
    }
    let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if mem_dc.0.is_null() {
        unsafe {
            ReleaseDC(HWND(std::ptr::null_mut()), screen_dc);
        }
        return Err(io::Error::last_os_error());
    }

    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
    if bitmap.0.is_null() {
        unsafe {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND(std::ptr::null_mut()), screen_dc);
        }
        return Err(io::Error::last_os_error());
    }

    let old = unsafe { SelectObject(mem_dc, bitmap) };
    if old.0.is_null() {
        unsafe {
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND(std::ptr::null_mut()), screen_dc);
        }
        return Err(io::Error::last_os_error());
    }

    let blt_result = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width,
            height,
            screen_dc,
            0,
            0,
            SRCCOPY | CAPTUREBLT,
        )
    };
    if let Err(err) = blt_result {
        unsafe {
            let _ = SelectObject(mem_dc, old);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND(std::ptr::null_mut()), screen_dc);
        }
        return Err(io::Error::other(format!("BitBlt failed: {err:?}")));
    }

    let w_u32 = u32::try_from(width).unwrap_or(0);
    let h_u32 = u32::try_from(height).unwrap_or(0);
    let mut buf = vec![0u8; (w_u32 as usize).saturating_mul(h_u32 as usize) * 4];

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>()).unwrap_or(0),
            biWidth: width,
            // Negative height = top-down DIB.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default(); 1],
    };

    let lines = unsafe {
        GetDIBits(
            mem_dc,
            bitmap,
            0,
            h_u32,
            Some(buf.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    unsafe {
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(HWND(std::ptr::null_mut()), screen_dc);
    }

    if lines == 0 {
        return Err(io::Error::last_os_error());
    }

    // Convert BGRA -> RGBA.
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2);
    }

    let Some(rgba) = image::RgbaImage::from_raw(w_u32, h_u32, buf) else {
        return Err(io::Error::other("failed to construct image buffer"));
    };
    let dyn_img = DynamicImage::ImageRgba8(rgba);
    dyn_img
        .save_with_format(path, ImageFormat::Png)
        .map_err(|err| io::Error::other(format!("failed to write png: {err}")))?;

    Ok((w_u32, h_u32))
}

#[cfg(not(windows))]
pub(crate) fn capture_primary_monitor_png(_path: &Path) -> io::Result<(u32, u32)> {
    Err(io::Error::other(
        "screenshot capture is only implemented on Windows",
    ))
}
