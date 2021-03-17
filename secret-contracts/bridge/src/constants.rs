// storage keys
pub const CONFIG_KEY: &[u8] = b"cfg";
pub const CONSTANTS_KEY: &[u8] = b"consts";
pub const CONTRACT_STATUS_KEY: &[u8] = b"status";
pub const MINTERS_KEY: &[u8] = b"minters"; // minimum required to swap sXMR for xmr
pub const MIN_SWAP_AMOUNT_KEY: &[u8] = b"min"; // minimum required to swap sXMR for xmr
pub const MONERO_PROOFS_KEY: &[u8] = b"proofs";
pub const SWAP_DETAILS_KEY: &[u8] = b"swaps";

pub const BLOCK_SIZE: usize = 256;
