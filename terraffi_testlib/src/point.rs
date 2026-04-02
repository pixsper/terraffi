use terraffi_macro::terraffi_export;

pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn distance(_p: *const Point) -> f32 {
    0.0
}
