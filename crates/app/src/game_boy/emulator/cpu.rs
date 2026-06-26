#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CpuMode {
    Running,
    Halted,
    Stopped,
}

impl Default for CpuMode {
    fn default() -> Self {
        Self::Running
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CpuRegisters {
    pub(crate) pc: u32,
    pub(crate) sp: u32,
    pub(crate) a: u8,
    pub(crate) f: u8,
    pub(crate) b: u8,
    pub(crate) c: u8,
    pub(crate) d: u8,
    pub(crate) e: u8,
    pub(crate) h: u8,
    pub(crate) l: u8,
    pub(crate) ime: bool,
    pub(crate) ime_enable_delay: u8,
    pub(crate) halt_bug: bool,
}

impl CpuRegisters {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.pc = 0x0100;
        self.sp = 0xfffe;
        self.f = 0xb0;
        self.b = 0x00;
        self.c = 0x13;
        self.d = 0x00;
        self.e = 0xd8;
        self.h = 0x01;
        self.l = 0x4d;
        self.ime = false;
        self.ime_enable_delay = 0;
        self.halt_bug = false;
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CpuTiming {
    pub(crate) clocks_acc: i32,
    pub(crate) clock_frequency_hz: i64,
    pub(crate) system_counter: u16,
    pub(crate) tima_reload_delay: u8,
    pub(crate) tima_reload_active: bool,
}

impl Default for CpuTiming {
    fn default() -> Self {
        Self {
            clocks_acc: 0,
            clock_frequency_hz: 1,
            system_counter: 0,
            tima_reload_delay: 0,
            tima_reload_active: false,
        }
    }
}

impl CpuTiming {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.clocks_acc = 0;
        self.system_counter = 0;
        self.tima_reload_delay = 0;
        self.tima_reload_active = false;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SerialState {
    pub(crate) request: bool,
    pub(crate) is_transferring: bool,
    pub(crate) clock_is_external: bool,
    pub(crate) timer: i32,
}

impl CpuMode {
    pub(crate) fn reset_for_rom_load(&mut self) {
        *self = Self::Running;
    }
}

impl SerialState {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.request = false;
        self.is_transferring = false;
        self.clock_is_external = false;
        self.timer = 0;
    }
}
