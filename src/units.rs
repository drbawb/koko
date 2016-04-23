use std::ops::{Add, Sub};

pub enum Color {
    RGB(u8, u8, u8),
    RGBA(u8, u8, u8, u8),
}

#[derive(Copy, Clone, Debug)]
pub struct V2f(pub f64, pub f64);

impl V2f {
    pub fn length(&self) ->  f64 {
        f64::sqrt(f64::powf(self.0, 2.0) + f64::powf(self.1, 2.0))
    }

    pub fn norm(&self) -> V2f {
        let length = self.length();

        match length <= 0.0 {
            true  => V2f(0.0, 0.0),
            false => V2f(self.0 / length, self.1 / length),
        }
    }
}

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
