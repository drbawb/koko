use std::ops::{Add, Sub};

#[derive(Copy, Clone,Debug)]
pub struct V2(pub i64, pub i64);

impl Add for V2 {
    type Output = V2;

    fn add(self, rhs: V2) -> V2 {
        V2(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Sub for V2 {
    type Output = V2;

    fn sub(self, rhs: V2) -> V2 {
        V2(self.0 - rhs.0, self.1 - rhs.1)
    }
}
