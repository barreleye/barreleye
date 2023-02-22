use derive_more::Display;

pub use block::Block;
pub use transaction::Transaction;

#[derive(Display, Debug)]
pub enum ParquetFile {
	#[display(fmt = "block")]
	Block,
	#[display(fmt = "transactions")]
	Transactions,
}

mod block;
mod transaction;
