use sha1::{Digest, Sha1};

pub fn rom_identifier(rom_data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    let header_start = 0x0104;
    let header_end_exclusive = 0x0150.min(rom_data.len());

    if rom_data.len() <= header_start {
        hasher.update(rom_data);
    } else {
        hasher.update(&rom_data[..header_start]);
        hasher.update(&vec![0; header_end_exclusive - header_start]);
        hasher.update(&rom_data[header_end_exclusive..]);
    }

    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_identifier_zeroes_header_bytes() {
        let mut first = vec![1; 0x0200];
        let mut second = first.clone();
        first[0x0104] = 2;
        second[0x0104] = 3;

        assert_eq!(rom_identifier(&first), rom_identifier(&second));
    }

    #[test]
    fn rom_identifier_preserves_bytes_after_header() {
        let mut first = vec![1; 0x0200];
        let mut second = first.clone();
        first[0x0150] = 2;
        second[0x0150] = 3;

        assert_ne!(rom_identifier(&first), rom_identifier(&second));
    }

    #[test]
    fn rom_identifier_preserves_bytes_before_header() {
        let mut first = vec![1; 0x0200];
        let mut second = first.clone();
        first[0x0103] = 2;
        second[0x0103] = 3;

        assert_ne!(rom_identifier(&first), rom_identifier(&second));
    }
}
