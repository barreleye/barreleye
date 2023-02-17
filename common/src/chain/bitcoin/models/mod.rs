use derive_more::Display;

pub use block::Block;

#[derive(Display, Debug)]
pub enum ParquetFile {
	#[display(fmt = "block")]
	Block,
}

mod block;
