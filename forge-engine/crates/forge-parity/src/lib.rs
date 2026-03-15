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

pub mod card_pool;
pub mod choice_space;
pub mod combat_choice_space;
pub mod comparator;
pub mod deck_generator;
pub mod deterministic_agent;
pub mod gui_repro;
pub mod java_bridge;
pub mod java_cache;
pub mod java_random;
pub mod parity_card_map;
pub mod parity_id;
pub mod parity_order;
pub mod protocol;
pub mod report;
pub mod runner;
pub mod snapshot;

#[cfg(feature = "analyze")]
pub mod agent_loop;
#[cfg(feature = "analyze")]
pub mod analyzer;
#[cfg(feature = "analyze")]
pub mod discord;
#[cfg(feature = "analyze")]
pub mod github_issues;
#[cfg(feature = "analyze")]
pub mod llm;
#[cfg(feature = "serve")]
pub mod log_buffer;
#[cfg(feature = "storage")]
pub mod scheduler;
#[cfg(feature = "storage")]
pub mod storage;
#[cfg(feature = "analyze")]
pub mod tools;
#[cfg(feature = "serve")]
pub mod web;
