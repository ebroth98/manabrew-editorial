//! Agent tools for the parity analyzer.
//!
//! Each module provides tool implementations that the agent loop can invoke
//! to explore code, look up MTG rules, and re-run parity tests.

pub mod code_tools;
pub mod mtg_tools;
pub mod parity_tools;
