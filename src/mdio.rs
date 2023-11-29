use crate::defs::*;
use crate::stmmac::StarfiveHal;

use core::ptr::{read_volatile, write_volatile};

pub const MII_BUSY: u32 = (1 << 0);
pub fn mdio_write<A: StarfiveHal>(ioaddr: usize, data: u32, value: u32) {

    loop {
        let value = unsafe { read_volatile((ioaddr + 0x10) as *mut u32) };

        if value & MII_BUSY != 1 {
            break;
        }
        A::mdelay(10);
    }



    unsafe{
        write_volatile((ioaddr + 0x14) as *mut u32, data);
        write_volatile((ioaddr + 0x10) as *mut u32, value);
    }

    loop {
        let value = unsafe { read_volatile((ioaddr + 0x10) as *mut u32) };

        if value & MII_BUSY != 1 {
            break;
        }
        A::mdelay(10);
    }
}