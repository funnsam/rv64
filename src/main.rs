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
    ram.resize(0x10000, 0);
    let mut bus = emu::bus::Bus::new(&mut ram);
    let mut cpu = emu::cpu::Cpu::new(&mut bus);

    loop {
        cpu.step(args.testing);
    }
}
