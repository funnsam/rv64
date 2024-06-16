mod emu;

fn main() {
    let prog = std::env::args().nth(1).unwrap();
    println!("{prog}");
    let mut ram = std::fs::read(&prog).unwrap();
    let mut bus = emu::bus::Bus::new(&mut ram);
    let mut io = emu::io::Io::new();
    let mut cpu = emu::cpu::Cpu::new(&mut bus, &mut io);

    loop {
        cpu.step();
    }
}
