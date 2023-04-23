
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Extent3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Extent3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn new_square(s: f64) -> Self {
        Extent3::new(s, s, 1f64)
    }

    pub fn abs(&self) -> Self {
        Extent3{ x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }

    pub fn as_abs_integer_tuple(&self) -> (usize, usize, usize) {
        ( self.x.abs() as usize, self.y.abs() as usize, self.z.abs() as usize )
    }
}
