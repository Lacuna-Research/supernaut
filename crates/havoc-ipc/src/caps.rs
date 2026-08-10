//! Capability constants for the client/core handshake (NORTH-STAR §4.8).
//!
//! Deliberately empty until stage 4 builds the handshake: the constants land
//! here, alongside the feature they name, so client and core negotiate from
//! one vocabulary and never fall into version-lockstep. That they live in this
//! crate — not in core — is the point (§4.2).
