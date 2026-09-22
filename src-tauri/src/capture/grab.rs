//! The single virtual-desktop frame grab, and cropping out of it.
//!
//! ADR-0004: the whole virtual desktop is captured **once** per overlay session
//! and every selection is a crop of that buffer. That is what makes the screen
//! genuinely frozen (the user OCRs what they saw, not what the screen shows
//! 400 ms later) and what makes re-selecting free.
//!
//! GDI `BitBlt` over the virtual screen is used rather than a per-monitor
//! capture crate: it produces one buffer spanning every monitor in a single
//! call, in physical pixels, with a negative origin handled naturally. Its
//! known limitation is exclusive-fullscreen and protected content, which
//! TD-07 already accepts and reports rather than silently returning black.

use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    HBITMAP, HDC, HGDIOBJ, SRCCOPY,
};

use super::geometry::{is_usable_selection, Rect};
use crate::error::CaptureError;

/// Bytes per pixel in the captured buffer (BGRA).
const BPP: usize = 4;

/// A BGRA image, top-down, tightly packed (no row padding).
pub struct BgraImage {
    /// Where this image lives in virtual-desktop space. Origin may be negative.
    pub bounds: Rect,
    pub pixels: Vec<u8>,
}

impl BgraImage {
    pub fn width(&self) -> i32 {
        self.bounds.w
    }

    pub fn height(&self) -> i32 {
        self.bounds.h
    }

    fn expected_len(bounds: &Rect) -> usize {
        (bounds.area() as usize) * BPP
    }

    /// Serialize as a 32-bit top-down BMP.
    ///
    /// BMP, not PNG, on purpose: PNG-encoding a multi-monitor 4K frame costs
    /// 200 ms+, which is most of the 250 ms budget for getting the overlay on
    /// screen (NFR-P1). A BMP is a 54-byte header plus a memcpy. The extra
    /// bytes never leave the machine — this goes straight to the local webview
    /// through the `otr://` handler.
    pub fn to_bmp(&self) -> Vec<u8> {
        const FILE_HEADER: usize = 14;
        const INFO_HEADER: usize = 40;
        let pixel_bytes = self.pixels.len();
        let file_size = FILE_HEADER + INFO_HEADER + pixel_bytes;

        let mut out = Vec::with_capacity(file_size);

        // BITMAPFILEHEADER
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // reserved1
        out.extend_from_slice(&0u16.to_le_bytes()); // reserved2
        out.extend_from_slice(&((FILE_HEADER + INFO_HEADER) as u32).to_le_bytes());

        // BITMAPINFOHEADER. Negative height = top-down, matching our buffer.
        out.extend_from_slice(&(INFO_HEADER as u32).to_le_bytes());
        out.extend_from_slice(&self.width().to_le_bytes());
        out.extend_from_slice(&(-self.height()).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // x pixels per meter
        out.extend_from_slice(&0i32.to_le_bytes()); // y pixels per meter
        out.extend_from_slice(&0u32.to_le_bytes()); // colors used
        out.extend_from_slice(&0u32.to_le_bytes()); // colors important

        out.extend_from_slice(&self.pixels);
        out
    }

    /// Crop a region given in **virtual-desktop** coordinates.
    ///
    /// The three steps here are the whole point of ADR-0004, in order:
    ///
    /// 1. clamp the selection to what was actually captured,
    /// 2. reject a mis-click,
    /// 3. translate virtual coordinates into buffer-local ones.
    ///
    /// Skipping step 3 is the classic bug that works perfectly on a single
    /// monitor and crops the wrong screen the moment one sits to the left.
    pub fn crop(&self, selection: &Rect) -> Result<BgraImage, CaptureError> {
        let clamped = selection.intersect(&self.bounds);
        if clamped.is_empty() {
            return Err(CaptureError::SelectionOutsideFrame);
        }
        if !is_usable_selection(&clamped) {
            return Err(CaptureError::SelectionTooSmall);
        }

        let local = clamped.to_local(self.bounds.x, self.bounds.y);

        let src_stride = self.width() as usize * BPP;
        let dst_stride = local.w as usize * BPP;
        let mut pixels = Vec::with_capacity(dst_stride * local.h as usize);

        for row in 0..local.h as usize {
            let start = (local.y as usize + row) * src_stride + local.x as usize * BPP;
            let end = start + dst_stride;
            let slice = self
                .pixels
                .get(start..end)
                .ok_or(CaptureError::SelectionOutsideFrame)?;
            pixels.extend_from_slice(slice);
        }

        Ok(BgraImage {
            // The crop keeps its position in virtual-desktop space, so OCR
            // bounding boxes can be mapped straight back onto the screen.
            bounds: clamped,
            pixels,
        })
    }
}

/// RAII wrappers so an early return can't leak a GDI handle. Leaking device
/// contexts in a long-running tray app is how a process slowly dies.
struct ScreenDc(HDC);

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(None, self.0);
        }
    }
}

struct MemDc(HDC);

impl Drop for MemDc {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}

struct Bitmap(HBITMAP);

impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0));
        }
    }
}

/// Capture `bounds` (virtual-desktop, physical pixels) into one BGRA buffer.
pub fn capture(bounds: Rect) -> Result<BgraImage, CaptureError> {
    if bounds.is_empty() {
        return Err(CaptureError::EmptyDesktop);
    }

    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            return Err(CaptureError::Win32 {
                call: "GetDC",
                detail: "returned a null device context".into(),
            });
        }
        let screen_dc = ScreenDc(screen_dc);

        let mem_dc = CreateCompatibleDC(Some(screen_dc.0));
        if mem_dc.is_invalid() {
            return Err(CaptureError::Win32 {
                call: "CreateCompatibleDC",
                detail: "returned a null device context".into(),
            });
        }
        let mem_dc = MemDc(mem_dc);

        let bitmap = CreateCompatibleBitmap(screen_dc.0, bounds.w, bounds.h);
        if bitmap.is_invalid() {
            return Err(CaptureError::Win32 {
                call: "CreateCompatibleBitmap",
                detail: format!("{}x{}", bounds.w, bounds.h),
            });
        }
        let bitmap = Bitmap(bitmap);

        let previous = SelectObject(mem_dc.0, HGDIOBJ(bitmap.0 .0));

        // CAPTUREBLT includes layered windows, which is what makes tooltips and
        // some overlays show up in the frame the user actually saw.
        let blit = BitBlt(
            mem_dc.0,
            0,
            0,
            bounds.w,
            bounds.h,
            Some(screen_dc.0),
            bounds.x,
            bounds.y,
            SRCCOPY | CAPTUREBLT,
        );

        if let Err(e) = blit {
            SelectObject(mem_dc.0, previous);
            return Err(CaptureError::Win32 {
                call: "BitBlt",
                detail: e.to_string(),
            });
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bounds.w,
                // Negative height asks GDI for a top-down buffer, so row 0 is
                // the top of the screen and indexing needs no flip.
                biHeight: -bounds.h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let len = BgraImage::expected_len(&bounds);
        let mut pixels = vec![0u8; len];

        let copied = GetDIBits(
            mem_dc.0,
            bitmap.0,
            0,
            bounds.h as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc.0, previous);

        if copied == 0 {
            return Err(CaptureError::Win32 {
                call: "GetDIBits",
                detail: "copied zero scan lines".into(),
            });
        }

        // GDI leaves the alpha byte at zero. A 32-bit BMP with a zero alpha
        // channel renders fully transparent in some decoders, so the frozen
        // frame would come up blank — force it opaque.
        for pixel in pixels.chunks_exact_mut(BPP) {
            pixel[3] = 0xFF;
        }

        Ok(BgraImage { bounds, pixels })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic frame whose every pixel encodes its own virtual-desktop
    /// coordinate, so a crop can be checked for having come from the right place.
    fn synthetic_frame(bounds: Rect) -> BgraImage {
        let mut pixels = vec![0u8; BgraImage::expected_len(&bounds)];
        for row in 0..bounds.h {
            for col in 0..bounds.w {
                let i = ((row * bounds.w + col) as usize) * BPP;
                let vx = bounds.x + col;
                let vy = bounds.y + row;
                pixels[i] = (vx & 0xFF) as u8;
                pixels[i + 1] = ((vx >> 8) & 0xFF) as u8;
                pixels[i + 2] = (vy & 0xFF) as u8;
                pixels[i + 3] = 0xFF;
            }
        }
        BgraImage { bounds, pixels }
    }

    fn decode_virtual_x(px: &[u8]) -> i32 {
        px[0] as i32 | ((px[1] as i32) << 8)
    }

    /// A laptop at 0,0 with an external monitor to its LEFT — the layout that
    /// makes the virtual origin negative.
    fn negative_origin_frame() -> BgraImage {
        synthetic_frame(Rect::new(-300, 0, 500, 200))
    }

    #[test]
    fn crop_reads_from_the_selection_not_the_buffer_origin() {
        let frame = negative_origin_frame();
        // A 40x20 box starting at virtual x = -250, i.e. on the left monitor.
        let crop = frame.crop(&Rect::new(-250, 10, 40, 20)).unwrap();

        assert_eq!(crop.bounds, Rect::new(-250, 10, 40, 20));
        assert_eq!(crop.pixels.len(), 40 * 20 * BPP);

        // The first pixel must carry virtual x = -250, not 0 and not -300.
        // If to_local() were skipped this would read -300.
        assert_eq!(decode_virtual_x(&crop.pixels[0..4]), -250 & 0xFFFF);
    }

    #[test]
    fn every_cropped_pixel_comes_from_the_right_source_coordinate() {
        let frame = negative_origin_frame();
        let selection = Rect::new(-120, 30, 16, 12);
        let crop = frame.crop(&selection).unwrap();

        for row in 0..selection.h {
            for col in 0..selection.w {
                let i = ((row * selection.w + col) as usize) * BPP;
                let expected_x = (selection.x + col) & 0xFFFF;
                assert_eq!(
                    decode_virtual_x(&crop.pixels[i..i + 4]),
                    expected_x,
                    "row {row} col {col}"
                );
                assert_eq!(crop.pixels[i + 2], ((selection.y + row) & 0xFF) as u8);
            }
        }
    }

    #[test]
    fn crop_spanning_the_monitor_boundary_is_contiguous() {
        // Straddles virtual x = 0, where the two monitors meet.
        let frame = negative_origin_frame();
        let crop = frame.crop(&Rect::new(-20, 0, 40, 10)).unwrap();
        assert_eq!(crop.bounds.w, 40);
        for col in 0..40 {
            let i = (col as usize) * BPP;
            assert_eq!(
                decode_virtual_x(&crop.pixels[i..i + 4]),
                (-20 + col) & 0xFFFF
            );
        }
    }

    #[test]
    fn crop_clamps_a_selection_that_overhangs_the_edge() {
        let frame = negative_origin_frame();
        let crop = frame.crop(&Rect::new(-400, -50, 200, 100)).unwrap();
        // Clamped to the frame: starts at the frame origin, keeps the overlap.
        assert_eq!(crop.bounds, Rect::new(-300, 0, 100, 50));
        assert_eq!(crop.pixels.len(), 100 * 50 * BPP);
    }

    #[test]
    fn crop_rejects_a_selection_entirely_off_screen() {
        let frame = negative_origin_frame();
        assert!(matches!(
            frame.crop(&Rect::new(-5000, 0, 100, 100)),
            Err(CaptureError::SelectionOutsideFrame)
        ));
    }

    #[test]
    fn crop_rejects_a_misclick() {
        let frame = negative_origin_frame();
        assert!(matches!(
            frame.crop(&Rect::new(0, 0, 4, 4)),
            Err(CaptureError::SelectionTooSmall)
        ));
    }

    #[test]
    fn crop_of_the_whole_frame_is_the_whole_frame() {
        let frame = negative_origin_frame();
        let crop = frame.crop(&frame.bounds).unwrap();
        assert_eq!(crop.bounds, frame.bounds);
        assert_eq!(crop.pixels, frame.pixels);
    }

    #[test]
    fn bmp_header_describes_a_top_down_32bit_image() {
        let image = synthetic_frame(Rect::new(-10, -10, 4, 3));
        let bmp = image.to_bmp();

        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(
            u32::from_le_bytes(bmp[2..6].try_into().unwrap()) as usize,
            bmp.len()
        );
        assert_eq!(u32::from_le_bytes(bmp[10..14].try_into().unwrap()), 54);
        assert_eq!(i32::from_le_bytes(bmp[18..22].try_into().unwrap()), 4);
        // Negative height = top-down.
        assert_eq!(i32::from_le_bytes(bmp[22..26].try_into().unwrap()), -3);
        assert_eq!(u16::from_le_bytes(bmp[28..30].try_into().unwrap()), 32);
        assert_eq!(bmp.len(), 54 + 4 * 3 * BPP);
    }

    #[test]
    fn capture_refuses_an_empty_desktop() {
        assert!(matches!(
            capture(Rect::new(0, 0, 0, 0)),
            Err(CaptureError::EmptyDesktop)
        ));
    }
}
