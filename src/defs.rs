pub const DMA_BUS_MODE: usize = 0x1000;

pub const DMA_CONTROL: usize = 0x00001018;

pub const DMA_BUS_MODE_SFT_RESET: usize = 0x00000001; /* Software Reset */

pub const DMA_RCV_BASE_ADDR: usize = 0x0000100c; /* Receive List Base */
pub const DMA_TX_BASE_ADDR: usize = 0x00001010; /* Transmit List Base */

/* DMA Control register defines */
pub const DMA_CONTROL_ST: u32 = 0x00002000; /* Start/Stop Transmission */
pub const DMA_CONTROL_SR: u32 = 0x00000002; /* Start/Stop Receive */

pub const MAC_ENABLE_TX: u32 = 1 << 3; /* Transmitter Enable */
pub const MAC_ENABLE_RX: u32 = 1 << 2; /* Receiver Enable */

pub const DMA_XMT_POLL_DEMAND: u32 = 0x00001004; /* Transmit Poll Demand */
pub const DMA_RCV_POLL_DEMAND: u32 = 0x00001008; /* Received Poll Demand */

pub const DESC_RXSTS_FRMLENMSK: u32 = 0x3FFF << 16;
pub const DESC_RXSTS_FRMLENSHFT: u32 = 16;

// mdio
pub const MII_BUSY: u32 = 1 << 0;
pub const MII_WRITE: u32 = 1 << 1;
pub const MII_CLKRANGE_60_100M: u32 = 0;
pub const MII_CLKRANGE_100_150M: u32 = 0x4;
pub const MII_CLKRANGE_20_35M: u32 = 0x8;
pub const MII_CLKRANGE_35_60M: u32 = 0xC;
pub const MII_CLKRANGE_150_250M: u32 = 0x10;
pub const MII_CLKRANGE_250_300M: u32 = 0x14;
pub const MIIADDRSHIFT: u32 = 11;
pub const MIIREGSHIFT: u32 = 6;
pub const MII_REGMSK: u32 = 0x1F << 6;
pub const MII_ADDRMSK: u32 = 0x1F << 11;

pub const SIFIVE_CCACHE_WAY_ENABLE: usize = 0x8;
