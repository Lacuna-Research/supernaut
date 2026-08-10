//! The headless havoc engine: connection actors, cap negotiation, storage,
//! event bus, request handling — per NORTH-STAR §4.2 and its naming amendment
//! (Supernaut app, havoc engine). Never depends on anything terminal.

pub mod bus;
pub mod connection;
pub mod core;
pub mod search;
pub mod storage;
