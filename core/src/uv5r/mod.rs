//! Baofeng UV-5R / UV-5RA support.
//!
//! Specification: `docs/protocol/uv5r.md`. Section numbers in comments (§x.y)
//! refer to that document.

pub mod channel;
pub mod image;
pub mod protocol;

pub use channel::{Channel, Tone};
pub use image::Image;
pub use protocol::{Connected, Options, Variant};
