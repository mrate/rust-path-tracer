use crate::math::Vector3;

pub const LUMINANCE: Vector3 = Vector3::new(0.2126, 0.7152, 0.0722);

pub const MIN_DIELECTRICS_F0: f32 = 0.04;
pub const MIN_DIELECTRICS_F0_VEC: Vector3 =
    Vector3::new(MIN_DIELECTRICS_F0, MIN_DIELECTRICS_F0, MIN_DIELECTRICS_F0);
