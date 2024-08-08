use derive_more::Display;

pub use self::log::Log;
pub use block::Block;
pub use receipt::Receipt;
pub use transaction::Transaction;

#[derive(Display, Debug)]
pub enum ParquetFile {
	#[display("blocks")]
	Blocks,
	#[display("transactions")]
	Transactions,
	#[display("receipts")]
	Receipts,
	#[display("logs")]
	Logs,
}

mod block;
mod log;
mod receipt;
mod transaction;
