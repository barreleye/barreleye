use derive_more::Display;

pub use block::Block;
pub use input::Input;
pub use output::Output;
pub use transaction::Transaction;

#[derive(Display, Debug)]
pub enum ParquetFile {
	#[display(fmt = "blocks")]
	Blocks,
	#[display(fmt = "transactions")]
	Transactions,
	#[display(fmt = "inputs")]
	Inputs,
	#[display(fmt = "outputs")]
	Outputs,
}

mod block;
mod input;
mod output;
mod transaction;
