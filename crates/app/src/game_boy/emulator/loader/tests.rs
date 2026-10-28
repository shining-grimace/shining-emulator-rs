use super::*;
use crate::game_boy::emulator::boot::{BootHeaderError, LOGO_START, OFFICIAL_LOGO};
use crate::game_boy::emulator::bus::{read8, write8};

#[test]
fn minimal_rom_header_loads_as_plain_game_boy_rom() {
    let rom = minimal_rom(0x00, 0x00, 0x00);

    let properties = parse_rom_properties(&rom).expect("minimal ROM should parse");

    assert!(properties.valid);
    assert_eq!(properties.mbc, MemoryBankController::None);
    assert_eq!(properties.size_bytes, i32::try_from(MIN_ROM_BYTES).unwrap());
    assert!(!properties.cgb_flag);
    assert!(!properties.sgb_flag);
}

#[test]
fn unsupported_cartridge_type_is_a_header_error() {
    let rom = minimal_rom(0xff, 0x00, 0x00);

    let error = parse_rom_properties(&rom).expect_err("cartridge type should fail");

    assert!(matches!(error, GameBoyLoadError::InvalidHeader(_)));
}

#[test]
fn large_ram_size_header_codes_are_valid() {
    let ram_128k = minimal_rom(0x1a, 0x00, 0x04);
    let ram_64k = minimal_rom(0x1a, 0x00, 0x05);

    assert!(parse_rom_properties(&ram_128k).is_ok());
    assert!(parse_rom_properties(&ram_64k).is_ok());
}

#[test]
fn emulation_model_setting_can_force_game_boy_mono() {
    let mut properties = RomProperties {
        cgb_flag: true,
        sgb_flag: true,
        ..Default::default()
    };

    apply_emulation_model_setting(&mut properties, EMULATION_MODEL_GAME_BOY_MONO);

    assert!(!properties.cgb_flag);
    assert!(!properties.sgb_flag);
}

#[test]
fn emulation_model_setting_can_force_super_game_boy() {
    let mut properties = RomProperties {
        cgb_flag: true,
        ..Default::default()
    };

    apply_emulation_model_setting(&mut properties, EMULATION_MODEL_SUPER_GAME_BOY);

    assert!(!properties.cgb_flag);
    assert!(properties.sgb_flag);
}

#[test]
fn removed_emulation_model_settings_fall_back_to_best_for_rom() {
    let mut properties = RomProperties {
        cgb_flag: true,
        sgb_flag: true,
        ..Default::default()
    };

    apply_emulation_model_setting(&mut properties, 2);

    assert!(properties.cgb_flag);
    assert!(properties.sgb_flag);
}

#[test]
fn boot_checks_use_the_selected_model_logo_length() {
    let mut rom = minimal_rom(0x00, 0x00, 0x00);
    rom[0x0143] = 0x80;
    rom[HEADER_CHECKSUM_INDEX] = 0x27;
    // CGB accepts a changed bottom half, while DMG and the SGB BIOS reject it.
    rom[LOGO_START + 24] ^= 1;
    assert!(load_for_model(&rom, EMULATION_MODEL_BEST_FOR_ROM).is_ok());
    for model in [
        EMULATION_MODEL_GAME_BOY_MONO,
        EMULATION_MODEL_SUPER_GAME_BOY,
    ] {
        let error = load_for_model(&rom, model).unwrap_err();
        assert!(error.to_string().contains("boot logo check failed"));
    }

    rom[LOGO_START + 23] ^= 1;
    let error = load_for_model(&rom, EMULATION_MODEL_BEST_FOR_ROM).unwrap_err();
    assert!(error.to_string().contains("boot logo check failed"));
}

#[test]
fn failed_boot_checksum_prevents_handoff_and_auto_save_restore() {
    for (cgb_flag, sgb_flag) in [(0, 0), (0x80, 0), (0, 3)] {
        let mut rom = minimal_rom(0x00, 0x00, 0x00);
        rom[0x0143] = cgb_flag;
        rom[SGB_FLAG_INDEX] = sgb_flag;
        // Deliberately wrong for all three headers.
        rom[HEADER_CHECKSUM_INDEX] = 0;
        let loaded = loaded_test_rom(rom);
        for resume_auto_save in [false, true] {
            let mut core = GameBoyCore::default();
            core.cpu_registers.pc = 0x4321;
            core.memory.rom[0x0100] = 0x76;
            let error = load_rom_into_emulator(
                &loaded,
                &mut core,
                &test_storage(EMULATION_MODEL_BEST_FOR_ROM),
                resume_auto_save,
            )
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("boot header checksum check failed")
            );
            assert_eq!(core.cpu_registers.pc, 0x4321);
            assert_eq!(core.memory.rom[0x0100], 0x76);
        }
    }
}

#[test]
fn valid_headers_reach_model_specific_handoff_with_boot_locked() {
    for (cgb_flag, sgb_flag, checksum, a, f, c) in [
        (0, 0, 0xa7, 0x01, 0xb0, 0x13),
        (0x80, 0, 0x27, 0x11, 0x80, 0x00),
        (0, 3, 0xa4, 0x01, 0x00, 0x14),
    ] {
        let mut rom = minimal_rom(0x00, 0x00, 0x00);
        rom[0x0143] = cgb_flag;
        rom[SGB_FLAG_INDEX] = sgb_flag;
        rom[HEADER_CHECKSUM_INDEX] = checksum;
        rom[0x0100] = 0xc3;
        let mut core = load_for_model(&rom, EMULATION_MODEL_BEST_FOR_ROM).unwrap();
        assert_eq!(core.cpu_registers.pc, 0x0100);
        assert_eq!(core.cpu_registers.sp, 0xfffe);
        assert_eq!(
            (
                core.cpu_registers.a,
                core.cpu_registers.f,
                core.cpu_registers.c
            ),
            (a, f, c)
        );
        assert!(!core.cpu_registers.ime);
        assert_eq!(core.cpu_registers.ime_enable_delay, 0);
        assert_eq!(read8(&core, 0xff50), 1);
        write8(&mut core, 0xff50, 0);
        assert_eq!(read8(&core, 0xff50), 1);
        assert_eq!(read8(&core, 0x0100), 0xc3);
    }
}

#[test]
fn valid_zero_checksum_reaches_dmg_handoff_with_carry_flags_clear() {
    let mut rom = minimal_rom(0x00, 0x00, 0x00);
    rom[0x014c] = 0xa7;
    rom[HEADER_CHECKSUM_INDEX] = 0;
    let core = load_for_model(&rom, EMULATION_MODEL_GAME_BOY_MONO).unwrap();
    assert_eq!(core.cpu_registers.f, 0x80);
}

#[test]
fn boot_checksum_covers_header_endpoints_but_not_global_checksum_or_body() {
    let rom = minimal_rom(0x00, 0x00, 0x00);
    let properties = parse_rom_properties(&rom).unwrap();
    for index in [0x0134, 0x014c, 0x014d] {
        let mut changed = rom.clone();
        changed[index] ^= 1;
        assert!(matches!(
            validate_boot_header(&changed, &properties),
            Err(BootHeaderError::ChecksumMismatch { .. })
        ));
    }
    let mut changed = rom;
    changed[0x014e..=0x0150].fill(0xff);
    assert_eq!(validate_boot_header(&changed, &properties), Ok(()));
    assert_eq!(
        validate_boot_header(&changed[..0x014d], &properties),
        Err(BootHeaderError::Truncated)
    );
}

fn load_for_model(bytes: &[u8], model: u8) -> Result<GameBoyCore, GameBoyLoadError> {
    let mut core = GameBoyCore::default();
    load_rom_into_emulator(
        &loaded_test_rom(bytes.to_vec()),
        &mut core,
        &test_storage(model),
        false,
    )?;
    Ok(core)
}

fn loaded_test_rom(bytes: Vec<u8>) -> LoadedRomBytes {
    LoadedRomBytes {
        file_name: "boot-test.gb".to_string(),
        opened_file_name: "boot-test.gb".to_string(),
        rom_id: "boot-test".to_string(),
        bytes,
    }
}

fn test_storage(model: u8) -> LocalStorage {
    let mut storage = LocalStorage {
        paths: crate::storage::paths::StoragePaths::default(),
        data: crate::storage::data::LocalStorageData::default(),
    };
    storage.data.settings.emulation_model = model;
    storage
}

fn minimal_rom(cart_type: u8, rom_size_code: u8, external_ram_code: u8) -> Vec<u8> {
    let mut rom = vec![0; MIN_ROM_BYTES];
    rom[LOGO_START..LOGO_START + OFFICIAL_LOGO.len()].copy_from_slice(&OFFICIAL_LOGO);
    rom[HEADER_TITLE_START..HEADER_TITLE_START + 4].copy_from_slice(b"TEST");
    rom[SGB_FLAG_INDEX] = 0x00;
    rom[CART_TYPE_INDEX] = cart_type;
    rom[ROM_SIZE_INDEX] = rom_size_code;
    rom[RAM_SIZE_INDEX] = external_ram_code;
    rom[HEADER_CHECKSUM_INDEX] = rom[HEADER_TITLE_START..HEADER_CHECKSUM_INDEX]
        .iter()
        .fold(0_u8, |checksum, byte| {
            checksum.wrapping_sub(*byte).wrapping_sub(1)
        });
    rom
}
