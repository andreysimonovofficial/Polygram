//! Снимок экрана приложения для отладки визуальных проблем.
//! Captures the desktop screen via Windows GDI (BitBlt from GetDC(0))
//! and saves as PNG. Нет необходимости в HWND — работает после отрисовки egui.

use std::path::PathBuf;
use std::time::SystemTime;

use image::RgbaImage;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    GetDeviceCaps, GetDC, GetWindowDC, ReleaseDC, SelectObject, SRCCOPY, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, GetWindowRect};

/// Состояние системы скриншотов.
pub struct ScreenshotState {
    /// Дескриптор окна приложения (HWND), если известен.
    pub hwnd: Option<isize>,
    /// Директория для сохранения скриншотов.
    pub dir: PathBuf,
    /// Флаг запроса скриншота (устанавливается из меню).
    pub requested: bool,
    /// Автозахват на первых N кадрах (для калибровки).
    pub auto_capture_frames: u32,
    /// Результат последнего скриншота.
    pub result: Option<Result<PathBuf, String>>,
}

impl Default for ScreenshotState {
    fn default() -> Self {
        Self {
            hwnd: None,
            dir: PathBuf::from("screenshots"),
            requested: false,
            auto_capture_frames: 0,
            result: None,
        }
    }
}

impl ScreenshotState {
    /// Установить HWND окна.
    pub fn set_hwnd(&mut self, hwnd: Option<isize>) {
        self.hwnd = hwnd;
    }

    /// Запросить скриншот.
    pub fn request(&mut self) {
        self.requested = true;
        self.result = None;
    }

    /// Выполнить захват экрана и сохранить как PNG.
    pub fn capture(&mut self) -> Result<PathBuf, String> {
        let _ = std::fs::write("capture_debug.txt", format!("capture() called, frames={}", self.auto_capture_frames));
        eprintln!("SCREENSHOT: capture() called");
        unsafe {
            let (window_dc, width, height, release_target) = if let Some(hwnd_ptr) = self.hwnd {
                let hwnd = hwnd_ptr as *mut std::ffi::c_void;
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect) == 0 {
                    return Err("GetWindowRect failed".to_owned());
                }
                let width = (rect.right - rect.left).max(1) as u32;
                let height = (rect.bottom - rect.top).max(1) as u32;
                let window_dc = GetWindowDC(hwnd);
                let _ = std::fs::write("capture_debug.txt", format!("window_dc={:p}", window_dc));
                if window_dc.is_null() {
                    return Err("GetWindowDC failed".to_owned());
                }
                (window_dc, width, height, Some(hwnd))
            } else {
                let desktop_hwnd = GetDesktopWindow();
                let _ = std::fs::write("capture_debug.txt", format!("desktop={:p}", desktop_hwnd));
                let window_dc = GetDC(desktop_hwnd);
                let _ = std::fs::write("capture_debug.txt", format!("window_dc={:p}", window_dc));
                if window_dc.is_null() {
                    return Err("GetDC(0) failed".to_owned());
                }
                let width = GetDeviceCaps(window_dc, 8i32) as u32;
                let height = GetDeviceCaps(window_dc, 10i32) as u32;
                (window_dc, width, height, Some(desktop_hwnd))
            };
            let mem_dc = CreateCompatibleDC(window_dc);
            let _ = std::fs::write("capture_debug.txt", format!("mem_dc={:p}", mem_dc));
            if mem_dc.is_null() {
                if let Some(hwnd) = release_target {
                    ReleaseDC(hwnd, window_dc);
                }
                return Err("CreateCompatibleDC failed".to_owned());
            }

            let bitmap = CreateCompatibleBitmap(window_dc, width as i32, height as i32);
            let _ = std::fs::write("capture_debug.txt", format!("bitmap={:p}", bitmap));
            if bitmap.is_null() {
                DeleteDC(mem_dc);
                if let Some(hwnd) = release_target {
                    ReleaseDC(hwnd, window_dc);
                }
                return Err("CreateCompatibleBitmap failed".to_owned());
            }

            let old_bitmap = SelectObject(mem_dc, bitmap);

            let success = BitBlt(
                mem_dc,
                0,
                0,
                width as i32,
                height as i32,
                window_dc,
                0,
                0,
                SRCCOPY,
            );

            let _ = std::fs::write("capture_debug.txt", format!("bitblt_success={}", success));

            if success == 0 {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap);
                DeleteDC(mem_dc);
                if let Some(target) = release_target {
                    ReleaseDC(target, window_dc);
                }
                return Err("BitBlt failed".to_owned());
            }

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0, // BI_RGB
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [unsafe { std::mem::zeroed() }],
            };

            let row_size = ((width * 32 + 31) / 32) * 4;
            let buffer_size = (row_size * height) as usize;
            let mut buffer: Vec<u8> = vec![0u8; buffer_size];

            let scanned = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height,
                buffer.as_mut_ptr() as *mut _,
                &mut bmi as *mut _,
                DIB_RGB_COLORS,
            );

            eprintln!("SCREENSHOT DEBUG: width={} height={} scanned={}", width, height, scanned);
            let _ = std::fs::write("capture_debug.txt", format!("width={} height={} scanned={}", width, height, scanned));

            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(mem_dc);
            if let Some(hwnd) = release_target {
                ReleaseDC(hwnd, window_dc);
            }

            if scanned == 0 {
                return Err("GetDIBits returned 0".to_owned());
            }

            std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;

            let ts = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs();
            let path = self.dir.join(format!("screenshot_{}.png", ts));

            let mut rgba = Vec::with_capacity(buffer_size);
            for chunk in buffer.chunks(4) {
                if chunk.len() == 4 {
                    rgba.push(chunk[2]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[0]); // B
                    rgba.push(chunk[3]); // A
                }
            }

            let img = RgbaImage::from_raw(width, height, rgba)
                .ok_or("Failed to create image from pixels")?;
            img.save(&path)
                .map_err(|e| format!("Failed to save PNG: {}", e))?;

            Ok(path)
        }
    }
}
