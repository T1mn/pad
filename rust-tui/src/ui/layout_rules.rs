pub const COPY_TOAST_MIN_WIDTH: u16 = 18;
pub const COPY_TOAST_MAX_WIDTH: u16 = 32;
pub const COPY_TOAST_HEIGHT: u16 = 4;
pub const COPY_TOAST_RIGHT_MARGIN: u16 = 2;
pub const COPY_TOAST_TOP_MARGIN: u16 = 1;

pub fn clamp_copy_toast_width(content_width: usize) -> u16 {
    (content_width as u16 + 4).clamp(COPY_TOAST_MIN_WIDTH, COPY_TOAST_MAX_WIDTH)
}
