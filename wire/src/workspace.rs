pub use crate::generated::rs::devtools::workspace::*;
use bitflags::bitflags;

bitflags! {
	pub struct FileType: u32 {
		const DIR      = 1 << 0;
		const FILE     = 1 << 1;
		const SYMLINK  = 1 << 2;
		const ASSET    = 1 << 3;
		const RESOURCE = 1 << 4;
	}
}
