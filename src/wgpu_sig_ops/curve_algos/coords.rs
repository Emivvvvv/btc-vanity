#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ProjectiveXYZ<F> {
    pub x: F,
    pub y: F,
    pub z: F,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ETEProjective<F> {
    pub x: F,
    pub y: F,
    pub t: F,
    pub z: F,
}
