//! Reconocer una imagen por sus primeros bytes.

/// Formatos que acepta el almacén de imágenes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Jpeg,
    Png,
    Webp,
}

impl Kind {
    pub fn ext(self) -> &'static str {
        match self {
            Kind::Jpeg => "jpg",
            Kind::Png => "png",
            Kind::Webp => "webp",
        }
    }
    pub fn mime(self) -> &'static str {
        match self {
            Kind::Jpeg => "image/jpeg",
            Kind::Png => "image/png",
            Kind::Webp => "image/webp",
        }
    }
    pub fn from_ext(ext: &str) -> Option<Kind> {
        match ext {
            "jpg" | "jpeg" => Some(Kind::Jpeg),
            "png" => Some(Kind::Png),
            "webp" => Some(Kind::Webp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    pub kind: Kind,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProbeError {
    #[error("formato de imagen no admitido")]
    Unsupported,
    #[error("imagen dañada o incompleta")]
    Corrupt,
}

/// Formato y medidas de una imagen a partir de sus bytes.
pub fn probe(bytes: &[u8]) -> Result<Probe, ProbeError> {
    let kind = match imagesize::image_type(bytes) {
        Ok(imagesize::ImageType::Jpeg) => Kind::Jpeg,
        Ok(imagesize::ImageType::Png) => Kind::Png,
        Ok(imagesize::ImageType::Webp) => Kind::Webp,
        Ok(_) => return Err(ProbeError::Unsupported),
        Err(_) => return Err(ProbeError::Unsupported),
    };
    let size = imagesize::blob_size(bytes).map_err(|_| ProbeError::Corrupt)?;
    let (width, height) = (
        u32::try_from(size.width).map_err(|_| ProbeError::Corrupt)?,
        u32::try_from(size.height).map_err(|_| ProbeError::Corrupt)?,
    );
    if width == 0 || height == 0 {
        return Err(ProbeError::Corrupt);
    }
    Ok(Probe {
        kind,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG mínimo válido de 2×3 (cabecera + IHDR bastan para medirlo).
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0]);
        v.extend_from_slice(&[0, 0, 0, 0]);
        v
    }

    #[test]
    fn reconoce_un_png_y_sus_medidas() {
        let p = probe(&png(2, 3)).unwrap();
        assert_eq!(p.kind, Kind::Png);
        assert_eq!((p.width, p.height), (2, 3));
    }

    #[test]
    fn un_texto_no_es_una_imagen() {
        assert_eq!(probe(b"hola, esto es texto"), Err(ProbeError::Unsupported));
    }

    #[test]
    fn extension_y_tipo() {
        assert_eq!(Kind::from_ext("jpeg"), Some(Kind::Jpeg));
        assert_eq!(Kind::Png.mime(), "image/png");
        assert_eq!(Kind::from_ext("gif"), None);
    }
}
