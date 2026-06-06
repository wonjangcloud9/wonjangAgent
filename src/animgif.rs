//! 움짤(animated GIF) 만들기 — 여러 이미지를 한 장의 움직이는 GIF로.
//!
//! `image` 크레이트의 gif 인코더만 쓴다(순수 Rust, C 의존 없음). 프레임은 흰 배경에
//! 비율 유지로 맞춰 같은 크기로 만든다(왜곡 없이). 인코딩 결과를 다시 디코딩해 검증 가능.

use anyhow::Result;
use image::{Rgba, RgbaImage};

/// 이미지를 (tw×th) 흰 배경에 비율 유지로 가운데 맞춤(프레임 크기를 통일).
pub fn fit_on_white(img: &RgbaImage, tw: u32, th: u32) -> RgbaImage {
    use image::imageops;
    let tw = tw.max(1);
    let th = th.max(1);
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return RgbaImage::from_pixel(tw, th, Rgba([255, 255, 255, 255]));
    }
    let scale = (tw as f64 / iw as f64).min(th as f64 / ih as f64);
    let nw = ((iw as f64 * scale).round() as u32).clamp(1, tw);
    let nh = ((ih as f64 * scale).round() as u32).clamp(1, th);
    let resized = imageops::resize(img, nw, nh, imageops::FilterType::Triangle);
    let mut canvas = RgbaImage::from_pixel(tw, th, Rgba([255, 255, 255, 255]));
    imageops::overlay(
        &mut canvas,
        &resized,
        ((tw - nw) / 2) as i64,
        ((th - nh) / 2) as i64,
    );
    canvas
}

/// 같은 크기 프레임들을 무한반복 animated GIF 바이트로 인코딩한다. `delay_ms`는 프레임 간격.
pub fn encode(frames: &[RgbaImage], delay_ms: u32) -> Result<Vec<u8>> {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};
    if frames.is_empty() {
        anyhow::bail!("프레임이 없어요.");
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut enc = GifEncoder::new(&mut buf);
        enc.set_repeat(Repeat::Infinite)
            .map_err(|e| anyhow::anyhow!("GIF 반복 설정 실패: {e}"))?;
        for f in frames {
            let frame = Frame::from_parts(f.clone(), 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
            enc.encode_frame(frame)
                .map_err(|e| anyhow::anyhow!("GIF 인코딩 실패: {e}"))?;
        }
    }
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_produces_valid_animated_gif() {
        let a = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
        let b = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 255, 255]));
        let bytes = encode(&[a, b], 100).unwrap();
        assert!(bytes.starts_with(b"GIF89a") || bytes.starts_with(b"GIF87a"));
        // 다시 디코딩해 프레임 2개 확인(왕복 검증).
        use image::AnimationDecoder;
        let dec = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes)).unwrap();
        let frames = dec.into_frames().collect_frames().unwrap();
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn fit_pads_to_target_size_without_distortion() {
        let img = RgbaImage::from_pixel(20, 10, Rgba([0, 0, 0, 255]));
        let f = fit_on_white(&img, 30, 30);
        assert_eq!(f.dimensions(), (30, 30));
        // 0 크기도 안전.
        assert_eq!(
            fit_on_white(&RgbaImage::new(0, 0), 10, 10).dimensions(),
            (10, 10)
        );
    }

    #[test]
    fn empty_frames_error() {
        assert!(encode(&[], 100).is_err());
    }
}
