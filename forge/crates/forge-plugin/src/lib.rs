//! forge-plugin — ElizaOS integration boundary for Forge.
//!
//! This crate provides the `SolveIssueAction` that wraps the Forge agent loop
//! behind an ElizaOS-compatible `ActionHandler::handle()` interface.
//!
//! It does NOT depend on the eliza crate directly — instead it defines the
//! contracts that a thin adapter can implement.

pub mod action;
pub use action::{SolveIssueAction, SolveIssueParams, SolveIssueResult};
