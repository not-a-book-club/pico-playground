#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::identity_op, clippy::collapsible_else_if)]

extern crate alloc;

mod life;
pub use life::Life;

mod elementry;
pub use elementry::Elementry;

mod bitgrid;
pub use bitgrid::BitGrid;
