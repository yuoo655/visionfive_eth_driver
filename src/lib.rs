#![no_std]
#![allow(dead_code)]

extern crate alloc;
#[macro_use]
extern crate log;

mod defs;
mod mdio;
mod rings;
mod stmmac;

pub use mdio::mdio_write;
pub use stmmac::StarfiveHal;
pub use stmmac::StmmacDevice;
