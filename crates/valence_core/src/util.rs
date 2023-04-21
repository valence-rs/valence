pub use glam::*;

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use rand::random;

    use super::*;

    #[test]
    fn yaw_pitch_round_trip() {
        for _ in 0..=100 {
            let d = (Vec3::new(random(), random(), random()) * 2.0 - 1.0).normalize();

            let (yaw, pitch) = to_yaw_and_pitch(d);
            let d_new = from_yaw_and_pitch(yaw, pitch);

            assert_relative_eq!(d, d_new, epsilon = f32::EPSILON * 100.0);
        }
    }
}

