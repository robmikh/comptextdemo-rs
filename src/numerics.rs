use windows::{Foundation::Numerics::Vector2, Graphics::SizeInt32};

pub trait ToVector2 {
    fn to_vector2(&self) -> Vector2;
}

impl ToVector2 for SizeInt32 {
    fn to_vector2(&self) -> Vector2 {
        Vector2 {
            X: self.Width as f32,
            Y: self.Height as f32,
        }
    }
}
