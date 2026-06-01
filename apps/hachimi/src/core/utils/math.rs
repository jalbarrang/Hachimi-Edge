//! Numeric helpers: integer scaling and aspect-ratio fitting.

pub fn scale_to_aspect_ratio(sizes: (i32, i32), aspect_ratio: f32, prefer_larger: bool) -> (i32, i32) {
    let (mut width, mut height) = sizes;
    let orig_aspect_ratio = width as f32 / height as f32;
    // Use original values if possible
    if (aspect_ratio - orig_aspect_ratio).abs() <= 0.001 {
        return sizes;
    } else if (aspect_ratio - 1.0 / orig_aspect_ratio).abs() <= 0.001 {
        return (height, width);
    }

    let scale_by_height = if prefer_larger { height > width } else { width > height };
    if scale_by_height {
        width = (height as f32 * aspect_ratio).round() as i32;
        // height = height;
    } else {
        // width = width;
        height = (width as f32 / aspect_ratio).round() as i32;
    }

    (width, height)
}

pub fn mul_int(base: i32, mult: f32) -> i32 {
    (base as f32 * mult).round() as i32
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn mul_int_basic() {
        assert_eq!(mul_int(10, 1.5), 15);
        assert_eq!(mul_int(10, 0.5), 5);
        assert_eq!(mul_int(3, 0.33), 1);
    }

    #[test]
    fn scale_aspect_ratio_already_correct() {
        let result = scale_to_aspect_ratio((1920, 1080), 1920.0 / 1080.0, false);
        assert_eq!(result, (1920, 1080));
    }

    #[test]
    fn scale_aspect_ratio_inverted() {
        // 1080x1920 with aspect 1920/1080 should swap
        let result = scale_to_aspect_ratio((1080, 1920), 1920.0 / 1080.0, false);
        assert_eq!(result, (1920, 1080));
    }

    #[test]
    fn scale_aspect_ratio_rescale() {
        let (w, h) = scale_to_aspect_ratio((800, 800), 16.0 / 9.0, false);
        let ratio = w as f32 / h as f32;
        assert!((ratio - 16.0 / 9.0).abs() < 0.02);
    }
}
