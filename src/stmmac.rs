use crate::defs::*;
use crate::mdio::*;
use crate::rings::*;

use core::marker::PhantomData;
use core::ptr::{read_volatile, write_volatile};

pub trait StarfiveHal {
    fn phys_to_virt(pa: usize) -> usize {
        pa
    }
    fn virt_to_phys(va: usize) -> usize {
        va
    }
    fn dma_alloc_pages(pages: usize) -> (usize, usize);

    fn dma_free_pages(vaddr: usize, pages: usize);

    fn mdelay(m_times: usize);

    fn fence();
}

pub struct StmmacDevice<A: StarfiveHal> {
    rx_ring: RxRing<A>,
    tx_ring: TxRing<A>,
    phantom: PhantomData<A>,
}

impl<A: StarfiveHal> StmmacDevice<A> {
    pub fn new() -> Self {
        let mut rx_ring = RxRing::<A>::new();
        let mut tx_ring = TxRing::<A>::new();

        let (rx_skb_va, rx_skb_pa) = A::dma_alloc_pages(512);

        for i in 0..512 {
            let buff_addr = rx_skb_pa + 0x1000 * i;
            rx_ring.init_rx_desc(i, buff_addr);
            rx_ring.skbuf.push(buff_addr);
        }

        for i in 0..512 {
            tx_ring.init_tx_desc(i, false);
        }
        tx_ring.init_tx_desc(511, true);

        let nic = StmmacDevice::<A> {
            rx_ring: rx_ring,
            tx_ring: tx_ring,
            phantom: PhantomData,
        };

        nic
    }

    pub fn receive(&mut self) -> Option<(*mut u8, u32)> {
        let rx_ring = &mut self.rx_ring;
        let rd_dma = &mut rx_ring.rd;
        let idx = rx_ring.idx;
        let rd = rd_dma.read_volatile(idx).unwrap();

        let rdes0 = rd.rdes0;

        let status = rdes0 & (1 << 31);

        if status >> 31 == 1 {
            // info!("dma own");
            return None;
        }

        let len = (rdes0 & DESC_RXSTS_FRMLENMSK) >> DESC_RXSTS_FRMLENSHFT;

        // get data from skb
        let skb_pa = rx_ring.skbuf[idx] as *mut u8;

        Some((skb_pa, len))
    }

    pub fn transmit(&mut self, skb_pa: usize, len: usize) {
        let tx_ring: &mut TxRing<A> = &mut self.tx_ring;
        let idx: usize = tx_ring.idx;
        tx_ring.set_skb(idx, skb_pa, len);

        let tdes_base = self.tx_ring.td.phy_addr as u32;
        unsafe {
            core::arch::asm!("fence	ow,ow");
        }
        sifive_ccache_flush_range::<A>(tdes_base as usize, tdes_base as usize + 0x1000);
        sifive_ccache_flush_range::<A>(skb_pa as usize, skb_pa as usize + 0x1000);

        let ioaddr = A::phys_to_virt(0x1002_0000);
        unsafe {
            write_volatile((ioaddr + 0x1004) as *mut u32, 0x1);
        }

        // wait until transmit finish
        loop {
            let td = self.tx_ring.td.read_volatile(idx).unwrap();
            if td.tdes0 & (1 << 31) == 0 {
                break;
            }
        }

        self.tx_ring.idx = (idx + 1) % 512;
    }

    pub fn dma_reset(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        unsafe {
            let mut value = read_volatile((ioaddr + DMA_BUS_MODE) as *mut u32);

            value |= DMA_BUS_MODE_SFT_RESET as u32;

            write_volatile((ioaddr + DMA_BUS_MODE) as *mut u32, value);
            A::mdelay(100);

            loop {
                let value = read_volatile((ioaddr + DMA_BUS_MODE) as *mut u32);
                if value != value & DMA_BUS_MODE_SFT_RESET as u32 {
                    break;
                }
            }
        }
    }

    pub fn dma_set_bus_mode(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        unsafe {
            write_volatile((ioaddr + DMA_BUS_MODE) as *mut u32, 0x910880);
        }
    }

    pub fn dma_rxtx_enable(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        unsafe {
            let mut value = read_volatile((ioaddr + DMA_CONTROL) as *mut u32);
            value |= DMA_CONTROL_SR | DMA_CONTROL_ST;
            write_volatile((ioaddr + DMA_CONTROL) as *mut u32, value);
        }
    }

    pub fn set_rxtx_base(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        let tdes_base = self.tx_ring.td.phy_addr as u32;
        let rdes_base = self.rx_ring.rd.phy_addr as u32;
        unsafe {
            write_volatile((ioaddr + DMA_TX_BASE_ADDR) as *mut u32, tdes_base);
            write_volatile((ioaddr + DMA_RCV_BASE_ADDR) as *mut u32, rdes_base);
        }
    }

    pub fn set_mac_addr(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        let macid_lo = 0xddccbbaa;
        let macid_hi = 0x0605;
        unsafe {
            write_volatile((ioaddr + 0x40) as *mut u32, macid_hi);
        }

        unsafe {
            write_volatile((ioaddr + 0x44) as *mut u32, macid_lo);
        }
    }

    pub fn stmmac_mac_link_up(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        unsafe {
            write_volatile((ioaddr + 0x18) as *mut u32, 0xe);
            write_volatile((ioaddr) as *mut u32, 0x61080c);
        }
    }
}

pub fn sifive_ccache_flush_range<A: StarfiveHal>(start: usize, end: usize) {
    let start_pa = start as usize;
    let end_pa = end as usize;
    let mut flush_addr = start_pa;
    let cache_line_size = 0x40;
    let cache_flush = A::phys_to_virt(0x201_0000);
    unsafe { core::arch::asm!("fence") };
    while flush_addr < end_pa as usize {
        unsafe {
            write_volatile((cache_flush + 0x200) as *mut usize, flush_addr);
        }
        flush_addr += cache_line_size;
    }
    A::fence();

    unsafe { core::arch::asm!("fence") };
}
