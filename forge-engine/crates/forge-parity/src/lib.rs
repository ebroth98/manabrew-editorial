//! `forge-parity` — Cross-engine differential testing for the Forge MTG engine.
//!
//! This crate provides tools to run games through the Rust forge-engine with
//! deterministic agents and compare the resulting game states against a reference
//! implementation (the Java Forge engine).
//!
//! ## Modules
//!
//! - [`protocol`] — Shared JSON types for cross-engine communication
//! - [`snapshot`] — Extracts normalized state snapshots from `GameState`
//! - [`deterministic_agent`] — A fully reproducible `PlayerAgent` implementation
//! - [`comparator`] — Diffs two snapshots to find divergences
//! - [`report`] — Formats parity reports as JSON or text
//! - [`runner`] — Orchestrates game execution and snapshot collection
//! - [`java_bridge`] — Stubbed subprocess bridge for the Java engine

pub mod comparator;
pub mod deterministic_agent;
pub mod java_bridge;
pub mod java_random;
pub mod protocol;
pub mod report;
pub mod runner;
pub mod snapshot;
