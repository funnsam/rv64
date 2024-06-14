mod emu;

fn main() {
    let mut ram = [
        /*
           00100093
           00000113
           3e800193
           002080b3
           40208133
           00011073
           fe314ae3
           0000006f
        */

        0x93, 0x00, 0x10, 0x00,
        0x13, 0x01, 0x00, 0x00,
        0x93, 0x01, 0x80, 0x3e,
        0xb3, 0x80, 0x20, 0x00,
        0x33, 0x81, 0x20, 0x40,
        0x73, 0x10, 0x01, 0x00,
        0xe3, 0x4a, 0x31, 0xfe,
        0x6f, 0x00, 0x00, 0x00,
    ];
    let mut bus = emu::bus::Bus::new(&mut ram);
    let mut io = emu::io::Io::new();
    let mut cpu = emu::cpu::Cpu::new(&mut bus, &mut io);

    loop {
        cpu.step();
    }
}
