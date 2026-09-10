//! Formal use cases: business logic orchestrated through ports.
//!
//! Each use case is a struct that takes its dependencies (ports) as
//! generics — pure dependency injection without a framework. The daemon
//! and GUI are thin delivery mechanisms that delegate here.

pub mod engine_control;
pub mod profile;
