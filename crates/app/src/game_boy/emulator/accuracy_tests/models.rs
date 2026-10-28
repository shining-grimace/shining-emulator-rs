//! Mooneye suffixes describe hardware, not cartridge capabilities.
//! <https://github.com/Gekkio/mooneye-test-suite#test-naming>
use super::{GameBoyCore, MemoryBankController, RomProperties};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Model {
    Dmg,
    Sgb,
    Cgb,
}

impl Model {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Dmg => "DMG",
            Self::Sgb => "SGB",
            Self::Cgb => "CGB",
        }
    }
}

pub(super) enum Targets {
    Run(&'static [Model]),
    Skip(&'static str),
}

pub(super) fn targets(path: &str) -> Result<Targets, String> {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("invalid Mooneye ROM path: {path}"))?;
    let suffix = stem.rsplit_once('-').map(|(_, suffix)| suffix);
    if let Some(groups) = suffix.filter(|suffix| {
        !suffix.is_empty()
            && suffix
                .chars()
                .all(|group| matches!(group, 'G' | 'S' | 'C' | 'A'))
    }) {
        return Ok(
            match (
                groups.contains('G'),
                groups.contains('S'),
                groups.contains('C'),
            ) {
                (true, true, true) => Targets::Run(&[Model::Dmg, Model::Sgb, Model::Cgb]),
                (true, true, false) => Targets::Run(&[Model::Dmg, Model::Sgb]),
                (true, false, true) => Targets::Run(&[Model::Dmg, Model::Cgb]),
                (false, true, true) => Targets::Run(&[Model::Sgb, Model::Cgb]),
                (true, false, false) => Targets::Run(&[Model::Dmg]),
                (false, true, false) => Targets::Run(&[Model::Sgb]),
                (false, false, true) => Targets::Run(&[Model::Cgb]),
                (false, false, false) => Targets::Skip("Game Boy Advance is not implemented"),
            },
        );
    }
    // The main acceptance suite targets monochrome hardware; unsuffixed tests
    // are not guaranteed to pass on CGB (see upstream README's final caveat).
    match suffix {
        None => Ok(Targets::Run(&[Model::Dmg, Model::Sgb])),
        Some("dmgABC" | "dmgABCmgb") => Ok(Targets::Run(&[Model::Dmg])),
        Some("sgb") => Ok(Targets::Run(&[Model::Sgb])),
        Some("cgb") => Ok(Targets::Run(&[Model::Cgb])),
        Some("dmg0") => Ok(Targets::Skip("DMG0 revision is not implemented")),
        Some("mgb") => Ok(Targets::Skip("Game Boy Pocket is not implemented")),
        Some("sgb2") => Ok(Targets::Skip("Super Game Boy 2 is not implemented")),
        Some("agb" | "ags") => Ok(Targets::Skip("Game Boy Advance is not implemented")),
        Some(suffix) => Err(format!(
            "unrecognised Mooneye hardware suffix {suffix:?}: {path}"
        )),
    }
}

pub(super) fn load_rom(bytes: &[u8], name: &str, model: Model) -> GameBoyCore {
    let mut core = GameBoyCore::default();
    // Acceptance ROMs use no mapper. Do not reuse this loader for mapper tests.
    let properties = RomProperties {
        valid: true,
        mbc: MemoryBankController::None,
        cgb_flag: model == Model::Cgb,
        sgb_flag: model == Model::Sgb,
        size_bytes: i32::try_from(bytes.len()).expect("Mooneye ROM size fits i32"),
        check_sum: u32::from(*bytes.get(0x014d).expect("Mooneye ROM has a header")),
        ..Default::default()
    };
    assert!(core.reset_for_rom_load(properties, name.to_owned(), name.to_owned(), bytes));
    core
}
