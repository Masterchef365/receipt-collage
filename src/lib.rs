#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::StripApp;
use serde::{Deserialize, Serialize};

/// Dimensions of the peice
#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Dimensions {
    /// Resolution of the image, in pixels
    pub resolution: [u32; 2],
    /// Real-world width of the peice, in centimeters
    pub width: f32,
}

/// One strip of paper
#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Strip {
    /// Position in normalized coordinates (0 to 1)
    pub position: [f32; 2],
    /// Width, Height in centimeters
    pub size: [f32; 2],
    /// Counter-clockwise rotation with 0 resting on the x axis
    pub rotation: f32,
}

/// Scene data
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Scene {
    pub dims: Dimensions,
    pub strips: Vec<Strip>,
}

impl Dimensions {
    /// Height in centimeters
    pub fn height(&self) -> f32 {
        self.width / self.aspect()
    }

    /// Height in centimeters
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Aspect
    fn aspect(&self) -> f32 {
        let [w, h] = self.resolution.map(|v| v as f32);
        w / h
    }

    /// Pixels per centimeter
    pub fn px_per_cm(&self) -> f32 {
        self.resolution[0] as f32 / self.width()
    }

    /// Centimeters per unit (normal) coordinate
    pub fn cm_per_norm(&self) -> f32 {
        self.width().max(self.height())
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            dims: Dimensions {
                resolution: [1920, 1080],
                width: 100.,
            },
            strips: vec![],
        }
    }
}
