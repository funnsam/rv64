use clap::*;
#[derive(Parser)]
struct Args {
    prog: String,

    #[arg(long)]
    testing: bool,
}

fn main() {
    let args = Args::parse();

    let mut ram = std::fs::read(&args.prog).unwrap();
    ram.resize(emu::bus::RAM_SIZE as usize, 0);
    let ram = emu::ram::Ram::new(&mut ram);
    let mut bus = emu::bus::Bus::new(ram);
    let mut cpu = emu::cpu::Cpu::new(&mut bus);

    loop {
        cpu.step(args.testing);
    }
}
