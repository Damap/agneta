//! Estimateurs stochastiques en streaming : Boundary-Hawkes, VPIN, Fēngdān et Divergence.
#![no_std]
#![forbid(unsafe_code)]

pub mod boundary_hawkes;
pub mod divergence;
pub mod exp_lut;
pub mod queue_depletion;
pub mod vpin;
