use base64::engine::general_purpose::STANDARD as base64_engine;
use base64::engine::Engine;
use itertools::Itertools;
use std::{io, ops::Deref};

pub struct EvcxrContent {
    mime_type: String,
    buf: io::Cursor<Vec<u8>>,
}

impl EvcxrContent {
    fn new<S: AsRef<str>>(mime_type: S) -> Self {
        EvcxrContent {
            mime_type: mime_type.as_ref().to_string(),
            buf: io::Cursor::new(Vec::new()),
        }
    }

    pub fn evcxr_display(&self) {
        let content = base64_engine.encode(self.buf.get_ref());
        if self.mime_type.starts_with("audio/") || self.mime_type == "application/ogg" {
            println!("EVCXR_BEGIN_CONTENT text/html\n<audio controls src=\"data:{};base64,{}\"/>\nEVCXR_END_CONTENT",
                     self.mime_type, content);
        } else {
            println!(
                "EVCXR_BEGIN_CONTENT {}\n{}\nEVCXR_END_CONTENT",
                self.mime_type, content
            );
        };
    }
}

impl EvcxrContent {
    pub fn from_imagebuffer<Px, Container>(img: &image::ImageBuffer<Px, Container>) -> Self
    where
        Px: image::PixelWithColorType,
        [Px::Subpixel]: image::EncodableLayout,
        Container: Deref<Target = [Px::Subpixel]>,
    {
        let mut content = EvcxrContent::new("image/png");
        img.write_to(&mut content, image::ImageOutputFormat::Png)
            .expect("Image compression failed");
        content
    }

    /// This method is not public. That's largely because it takes a reference-to-Vec
    /// rather than a reference-to-slice. This isn't a great API but it is necessary
    /// to avoid having to re-format the data again ready to give to the Vorbis encode
    /// function.
    fn from_i16_interleaved(buf: &Vec<i16>, channels: u32, sample_rate: u64) -> Self {
        const QUALITY: f32 = 3.0;

        let mut encoder = vorbis_encoder::Encoder::new(channels, sample_rate, QUALITY).unwrap();
        let mut output = encoder.encode(buf).expect("Audio compression failed");
        output.append(&mut encoder.flush().expect("Codec refused to flush"));

        Self {
            mime_type: "application/ogg".to_string(),
            buf: io::Cursor::new(output),
        }
    }

    /// TODO: Pass IntoInterator instead of slice (and can be be generic on Iterator::Type)
    /// TODO: Factor out common parts
    pub fn from_f32_mono(buf: &[f32], sample_rate: u64) -> Self {
        let input: Vec<_> = buf.iter().map(|&x| (x * 32767.0) as i16).collect();

        Self::from_i16_interleaved(&input, 1, sample_rate)
    }

    pub fn from_f32_stereo(lbuf: &[f32], rbuf: &[f32], sample_rate: u64) -> Self {
        let lchan = lbuf.iter().map(|&x| (x * 32767.0) as i16);
        let rchan = rbuf.iter().map(|&x| (x * 32767.0) as i16);
        let input: Vec<_> = lchan.interleave(rchan).collect();

        Self::from_i16_interleaved(&input, 2, sample_rate)
    }
}

impl io::Write for EvcxrContent {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf.flush()
    }
}

impl io::Seek for EvcxrContent {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.buf.seek(pos)
    }
}
