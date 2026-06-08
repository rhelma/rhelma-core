#![forbid(unsafe_code)]

//! Shared AI-related event contracts for Rhelma services.
//!
//! This crate exists to prevent duplicated schema structs across apps (e.g. ai-orchestrator,
//! sandbox-runner) and to keep event payloads stable.

pub mod improvements;

pub mod governance;
