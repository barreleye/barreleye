use derive_more::Display;

pub use block::Block;
pub use input::Input;
pub use output::Output;
pub use transaction::Transaction;

#[derive(Display, Debug)]
pub enum ParquetFile {
	#[display("blocks")]
	Blocks,
	#[display("transactions")]
	Transactions,
	#[display("inputs")]
	Inputs,
	#[display("outputs")]
	Outputs,
}

mod block;
mod input;
mod output;
mod transaction;
