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
    pub rx_ring: RxRing<A>,
    pub tx_ring: TxRing<A>,
    phantom: PhantomData<A>,
}

impl<A: StarfiveHal> StmmacDevice<A> {
    pub fn new() -> Self {
        let mut rx_ring = RxRing::<A>::new();
        let mut tx_ring = TxRing::<A>::new();

        // log::info!("dma_alloc_pages");
        // let (rx_skb_va, rx_skb_pa) = A::dma_alloc_pages(512);

        let skb_start = 0x1801_0000 as usize;
    
        for i in 0..128 {
            let buff_addr = skb_start + 0x1000 * i;
            rx_ring.init_rx_desc(i, buff_addr);
            rx_ring.skbuf.push(A::phys_to_virt(buff_addr));
        }

        // let tskb_start = 0x1802_0000 as usize;
        for i in 0..16 {
            tx_ring.init_tx_desc(i, false);
        }
        tx_ring.init_tx_desc(15, true);

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
            // log::info!("dma own");
            return None;
        }

        let len = (rdes0 & DESC_RXSTS_FRMLENMSK) >> DESC_RXSTS_FRMLENSHFT;

        // get data from skb
        let skb_va = rx_ring.skbuf[idx];
        let skb = skb_va as *mut u8;
        // unsafe {
        //     let packet:&[u8]=  core::slice::from_raw_parts(skb, len as usize);
        //     log::info!("idx {:?} packet {:x?} ",idx, packet);
        // }

        

        Some((skb, len))
    }

    pub fn rx_clean(&mut self){
        let rx_ring = &mut self.rx_ring;
        let rd_dma = &mut rx_ring.rd;
        let idx = rx_ring.idx;

        log::info!("clean idx {:?}", idx);
        let ioaddr = A::phys_to_virt(0x1002_0000);
        let value = unsafe{
            read_volatile((ioaddr + 0x104c) as *mut u32)
        };
        log::info!("Current Host rx descriptor -----{:#x?}", value);
        if idx == 127{
            let skb_start = 0x1801_0000 as usize;
            for i in 0..128 {
                let buff_addr = skb_start + 0x1000 * i;
                rx_ring.init_rx_desc(i, buff_addr);
            }
        }
        rx_ring.idx = (idx + 1) % 128;
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
        let value = unsafe{
            read_volatile((ioaddr + 0x1048) as *mut u32)
        };
        log::info!("Current Host tx descriptor -----{:#x?}", value);

        let value = unsafe{
            read_volatile((ioaddr + 0x1048) as *mut u32)
        };
        log::info!("wait until transmit finish");

        // wait until transmit finish
        loop {
            let td = self.tx_ring.td.read_volatile(idx).unwrap();
            if td.tdes0 & (1 << 31) == 0 {
                break;
            }
        }


        log::info!("transmit finish");

        self.tx_ring.idx = (idx + 1) % 16;
    }


    pub fn tx_clean(&mut self){

        // let tx_ring = &mut self.tx_ring;
        // let idx = tx_ring.idx;
        // log::info!("---------tx clean--------idx{:#x?}", idx);
        // if idx == 15{
        //     for i in 0..16 {
        //         tx_ring.init_tx_desc(i, false);
        //     }
        //     tx_ring.init_tx_desc(15, true);
        // }
        // tx_ring.idx = (idx + 1) % 16;
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
            // write_volatile((ioaddr + 0x1028) as *mut u32, 0xf0);
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
        log::info!("rx base {:#x?} tx base {:#x?}", rdes_base, tdes_base);
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

    pub fn core_init(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        unsafe {
            write_volatile((ioaddr) as *mut u32, 0x618000);
        }
    }


    pub fn stmmac_set_mac(&self,enable: bool) {
        let old_val: u32;
        let mut value: u32;
        let ioaddr = A::phys_to_virt(0x1002_0000);
        old_val = unsafe { read_volatile(ioaddr as *mut u32) };
        value = old_val;
    
        if enable {
            value |= MAC_ENABLE_RX | MAC_ENABLE_TX;
        } else {
            value &= !(MAC_ENABLE_TX | MAC_ENABLE_RX);
        }
    
        if value != old_val {
            unsafe { write_volatile(ioaddr as *mut u32, value) }
        }
    }

    pub fn stmmac_mac_link_up(&self) {
        let ioaddr = A::phys_to_virt(0x1002_0000);
        unsafe {
            // write_volatile((ioaddr + 0x18) as *mut u32, 0xe);
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
