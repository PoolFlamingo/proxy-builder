//! **La imagen del portapapeles**, leída desde Rust: WebKitGTK no deja que
//! la vista web use `navigator.clipboard.read()` («The request is not allowed
//! by the user agent…»). En Linux se pide al portapapeles de GTK (el de la
//! propia ventana, vale igual en X11 que en Wayland); en Windows y macOS, con
//! arboard.

use image::RgbaImage;
use tauri::AppHandle;

use crate::error::{AppError, AppResult, ErrorCode};

fn fallo(e: impl ToString) -> AppError {
    AppError::new(ErrorCode::Internal).with("detalle", e)
}

/// La imagen que haya en el portapapeles; `None` si no hay ninguna.
#[cfg(target_os = "linux")]
pub async fn imagen(app: &AppHandle) -> AppResult<Option<RgbaImage>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    // GTK solo se toca en el hilo principal. `request_image` no bloquea: GTK
    // llama de vuelta cuando el dueño del portapapeles ha entregado la imagen.
    app.run_on_main_thread(move || {
        gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).request_image(move |_, p| {
            let _ = tx.send(p.and_then(rgba));
        });
    })
    .map_err(fallo)?;
    rx.await.map_err(fallo)
}

/// Copia los píxeles de un `Pixbuf` (RGB o RGBA de 8 bits, filas con relleno).
#[cfg(target_os = "linux")]
fn rgba(p: &gtk::gdk_pixbuf::Pixbuf) -> Option<RgbaImage> {
    let n = usize::try_from(p.n_channels()).ok()?;
    if p.bits_per_sample() != 8 || !(3..=4).contains(&n) {
        return None;
    }
    let (w, h) = (
        u32::try_from(p.width()).ok()?,
        u32::try_from(p.height()).ok()?,
    );
    let paso = usize::try_from(p.rowstride()).ok()?;
    let ancho = w as usize * n;
    let bytes = p.read_pixel_bytes();
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h as usize {
        let fila = bytes.get(y * paso..y * paso + ancho)?;
        for px in fila.chunks_exact(n) {
            out.extend_from_slice(&px[..3]);
            out.push(px.get(3).copied().unwrap_or(255));
        }
    }
    RgbaImage::from_raw(w, h, out)
}

/// La imagen que haya en el portapapeles; `None` si no hay ninguna.
#[cfg(not(target_os = "linux"))]
pub async fn imagen(_app: &AppHandle) -> AppResult<Option<RgbaImage>> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut cb = arboard::Clipboard::new().map_err(fallo)?;
        match cb.get_image() {
            Ok(i) => Ok(RgbaImage::from_raw(
                u32::try_from(i.width).map_err(fallo)?,
                u32::try_from(i.height).map_err(fallo)?,
                i.bytes.into_owned(),
            )),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(e) => Err(fallo(e)),
        }
    })
    .await
    .map_err(fallo)?
}
