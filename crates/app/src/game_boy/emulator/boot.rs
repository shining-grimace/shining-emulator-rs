//! Checks performed before the boot-bypass handoff at $0100.
//!
//! References: <https://gbdev.io/pandocs/Power_Up_Sequence.html> and
//! <https://gbdev.io/pandocs/The_Cartridge_Header.html>.
//! The selected hardware model determines the logo check, not the original
//! cartridge flags. SGB's SNES BIOS performs the checks on behalf of its boot ROM.

use super::rom::RomProperties;

pub(super) const LOGO_START: usize = 0x0104;
pub(super) const HEADER_CHECKSUM_INDEX: usize = 0x014d;
pub(super) const OFFICIAL_LOGO: [u8; 48] = [
    0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00, 0x0d,
    0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd, 0xd9, 0x99,
    0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BootHeaderError {
    Truncated,
    LogoMismatch,
    ChecksumMismatch { expected: u8, actual: u8 },
}

impl std::fmt::Display for BootHeaderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("boot header is incomplete"),
            Self::LogoMismatch => formatter.write_str("boot logo check failed"),
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "boot header checksum check failed: expected 0x{expected:02x}, found 0x{actual:02x}"
            ),
        }
    }
}

impl std::error::Error for BootHeaderError {}

pub(super) fn validate_boot_header(
    bytes: &[u8],
    model: &RomProperties,
) -> Result<(), BootHeaderError> {
    let header = bytes
        .get(..=HEADER_CHECKSUM_INDEX)
        .ok_or(BootHeaderError::Truncated)?;
    let logo_len = if model.cgb_flag { 24 } else { 48 };
    if header[LOGO_START..LOGO_START + logo_len] != OFFICIAL_LOGO[..logo_len] {
        return Err(BootHeaderError::LogoMismatch);
    }

    // The global checksum at $014E-$014F is deliberately not checked by boot.
    let expected = header[0x0134..HEADER_CHECKSUM_INDEX]
        .iter()
        .fold(0_u8, |checksum, byte| {
            checksum.wrapping_sub(*byte).wrapping_sub(1)
        });
    let actual = header[HEADER_CHECKSUM_INDEX];
    if expected != actual {
        return Err(BootHeaderError::ChecksumMismatch { expected, actual });
    }
    Ok(())
}
