use derive_more::Display;

pub use self::log::Log;
pub use block::Block;
pub use receipt::Receipt;
pub use transaction::Transaction;

#[derive(Display, Debug)]
pub enum ParquetFile {
	#[display(fmt = "block")]
	Block,
	#[display(fmt = "transactions")]
	Transactions,
	#[display(fmt = "receipts")]
	Receipts,
	#[display(fmt = "logs")]
	Logs,
}

mod block;
mod log;
mod receipt;
mod transaction;
