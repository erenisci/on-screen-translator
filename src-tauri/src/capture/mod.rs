//! Screen capture: topology, the single frame grab, and the coordinate space
//! everything downstream depends on.

pub mod geometry;
pub mod grab;
pub mod monitors;

pub use geometry::Rect;
pub use grab::BgraImage;
pub use monitors::{MonitorInfo, Topology};
