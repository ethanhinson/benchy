#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleUnit {
    Degree,
    Radian,
    Gradian,
}

impl AngleUnit {
    pub fn to_radians(self, value: f64) -> f64 {
        match self {
            AngleUnit::Degree | AngleUnit::Gradian => value.to_radians(),
            AngleUnit::Radian => value,
        }
    }
}
