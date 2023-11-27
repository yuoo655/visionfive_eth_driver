use crate::defs::*;
use crate::stmmac::StarfiveHal;

use core::ptr::{read_volatile, write_volatile};

pub fn mdio_write<A: StarfiveHal>(ioaddr: usize, data: u32, value: u32) {
    while unsafe { read_volatile((ioaddr + 0x10) as *mut u32) } & MII_BUSY != 1 {
        A::mdelay(10);
    }

    unsafe {
        write_volatile((ioaddr + 0x14) as *mut u32, data);
        write_volatile((ioaddr + 0x10) as *mut u32, value);
    }

    while unsafe { read_volatile((ioaddr + 0x10) as *mut u32) } & MII_BUSY != 1 {
        A::mdelay(10);
    }
}
