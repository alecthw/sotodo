use dioxus::desktop::tao::window::Icon as WindowIcon;

pub(crate) const MAIN_CSS: &str = include_str!("../assets/main.css");
pub(crate) const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
pub(crate) const APPICON_ICO_BYTES: &[u8] = include_bytes!("../assets/appicon.ico");

pub(crate) fn app_window_icon() -> Option<WindowIcon> {
    WindowIcon::from_rgba(app_icon_rgba(256), 256, 256).ok()
}

pub(crate) fn app_icon_rgba(size: u32) -> Vec<u8> {
    let scale = size as f32 / 456.0;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            let point = (x as f32 + 0.5, y as f32 + 0.5);
            let mut color: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
            if rounded_rect_contains(
                point,
                16.0 * scale,
                16.0 * scale,
                424.0 * scale,
                424.0 * scale,
                100.0 * scale,
            ) {
                color = [13.0, 148.0, 136.0, 255.0];
            }
            if rounded_rect_contains(
                point,
                94.0 * scale,
                58.0 * scale,
                260.0 * scale,
                336.0 * scale,
                38.0 * scale,
            ) {
                color = [248.0, 250.0, 252.0, 255.0];
            }
            for (start, end, width, rgb) in [
                ((136.0, 150.0), (280.0, 150.0), 24.0, [15.0, 23.0, 42.0]),
                ((136.0, 216.0), (260.0, 216.0), 24.0, [15.0, 23.0, 42.0]),
                ((136.0, 282.0), (220.0, 282.0), 24.0, [15.0, 23.0, 42.0]),
                ((216.0, 302.0), (266.0, 352.0), 40.0, [250.0, 204.0, 21.0]),
                ((266.0, 352.0), (362.0, 220.0), 40.0, [250.0, 204.0, 21.0]),
            ] {
                if distance_to_segment(
                    point,
                    (start.0 * scale, start.1 * scale),
                    (end.0 * scale, end.1 * scale),
                ) <= width * scale / 2.0
                {
                    color = [rgb[0], rgb[1], rgb[2], 255.0];
                }
            }
            pixels.extend(color.into_iter().map(|value| value.round() as u8));
        }
    }

    pixels
}

pub(crate) fn rounded_rect_contains(
    point: (f32, f32),
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> bool {
    let px = point.0.clamp(x + radius, x + width - radius);
    let py = point.1.clamp(y + radius, y + height - radius);
    distance(point, (px, py)) <= radius
}

pub(crate) fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

pub(crate) fn distance_to_segment(point: (f32, f32), start: (f32, f32), end: (f32, f32)) -> f32 {
    let segment = (end.0 - start.0, end.1 - start.1);
    let length_squared = segment.0 * segment.0 + segment.1 * segment.1;
    let t = (((point.0 - start.0) * segment.0 + (point.1 - start.1) * segment.1) / length_squared)
        .clamp(0.0, 1.0);
    distance(point, (start.0 + segment.0 * t, start.1 + segment.1 * t))
}

#[cfg(windows)]
pub(crate) fn best_ico_image(bytes: &[u8]) -> Option<&[u8]> {
    if read_u16(bytes, 0)? != 0 || read_u16(bytes, 2)? != 1 {
        return None;
    }

    let count = read_u16(bytes, 4)? as usize;
    let mut best: Option<(usize, usize, u32)> = None;
    for index in 0..count {
        let entry = 6 + index * 16;
        let width = bytes.get(entry).copied()? as u32;
        let width = if width == 0 { 256 } else { width };
        let size = read_u32(bytes, entry + 8)? as usize;
        let offset = read_u32(bytes, entry + 12)? as usize;
        if offset.checked_add(size)? > bytes.len() {
            continue;
        }
        if best.map_or(true, |(_, _, best_width)| width >= best_width) {
            best = Some((offset, size, width));
        }
    }

    let (offset, size, _) = best?;
    bytes.get(offset..offset + size)
}

#[cfg(windows)]
pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

#[cfg(windows)]
pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
