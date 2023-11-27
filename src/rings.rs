use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::ptr::{read_volatile, write_volatile};

use crate::StarfiveHal;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct RxDes {
    pub rdes0: u32,
    pub rdes1: u32,
    pub rdes2: u32,
    pub rdes3: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct TxDes {
    pub tdes0: u32,
    pub tdes1: u32,
    pub tdes2: u32,
    pub tdes3: u32,
}

#[derive(Debug)]
pub struct Dma<T> {
    pub count: usize,
    pub phy_addr: usize,
    pub cpu_addr: *mut T,
}

impl<T> Dma<T> {
    pub fn new(cpu_addr: *mut T, phy_addr: usize, count: usize) -> Self {
        Self {
            count: count,
            phy_addr: phy_addr,
            cpu_addr: cpu_addr,
        }
    }

    pub fn read_volatile(&self, index: usize) -> Option<T> {
        if index >= self.count {
            return None;
        }
        let ptr = self.cpu_addr.wrapping_add(index);
        Some(unsafe { ptr.read() })
    }

    pub fn write_volatile(&self, index: usize, value: &T) -> bool
    where
        T: Copy,
    {
        if index >= self.count {
            return false;
        }
        let ptr = self.cpu_addr.wrapping_add(index);
        unsafe { ptr.write(*value) };
        true
    }
}

pub struct RxRing<A> {
    pub rd: Dma<RxDes>,
    pub idx: usize,
    pub skbuf: Vec<usize>,
    phantom: PhantomData<A>,
}

impl<A: StarfiveHal> RxRing<A> {
    pub fn new() -> Self {
        let count = 512;
        let size = core::mem::size_of::<RxDes>() * count;
        let pages = (size + 0x1000 - 1) / 0x1000;
        let (va, pa) = A::dma_alloc_pages(pages);

        let rd_dma = Dma::new(va as _, pa, count);
        let skbuf = Vec::new();

        Self {
            rd: rd_dma,
            idx: 0,
            skbuf: skbuf,
            phantom: PhantomData,
        }
    }

    pub fn init_rx_desc(&mut self, idx: usize, skb_phys_addr: usize) {
        let mut rd = RxDes {
            rdes0: 0,
            rdes1: 0,
            rdes2: 0,
            rdes3: 0,
        };
        //RDES0_OWN
        rd.rdes0 = 1 << 31 as u32;
        //buff size
        rd.rdes1 |= 0x600;
        rd.rdes2 = skb_phys_addr as u32;
        self.rd.write_volatile(idx, &rd);
    }
}

pub struct TxRing<A> {
    pub td: Dma<TxDes>,
    pub idx: usize,
    pub skbuf: Vec<usize>,
    phantom: PhantomData<A>,
}

impl<A: StarfiveHal> TxRing<A> {
    pub fn new() -> Self {
        let count = 512;
        let size = core::mem::size_of::<RxDes>() * count;
        let pages = (size + 0x1000 - 1) / 0x1000;
        let (va, pa) = A::dma_alloc_pages(pages);

        let td_dma = Dma::new(va as _, pa, count);
        let skbuf = Vec::new();

        Self {
            td: td_dma,
            idx: 0,
            skbuf: skbuf,
            phantom: PhantomData,
        }
    }

    pub fn init_tx_desc(&mut self, idx: usize, end: bool) {
        let mut td: TxDes = TxDes {
            tdes0: 0,
            tdes1: 0,
            tdes2: 0,
            tdes3: 0,
        };

        if end {
            td.tdes0 |= 1 << 21;
        }

        self.td.write_volatile(idx, &td);
    }

    pub fn set_skb(&mut self, idx: usize, skb_addr: usize, len: usize) {
        let mut td = self.td.read_volatile(idx).unwrap();

        td.tdes2 = skb_addr as u32;
        td.tdes1 = len as u32;

        td.tdes0 |= 1 << 29;
        td.tdes0 |= 1 << 28;
        td.tdes0 |= 1 << 31;
        self.td.write_volatile(idx, &td);
    }
}
