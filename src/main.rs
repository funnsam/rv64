use clap::*;
#[derive(Parser)]
struct Args {
    prog: String,

    #[arg(long)]
    testing: bool,
}

fn main() {
    let args = Args::parse();
    if args.testing {
        println!("{}", args.prog);
    }

    let mut ram = std::fs::read(&args.prog).unwrap();
    let mut bus = emu::bus::Bus::new(&mut ram);
    let mut io = emu::io::Io::new();
    let mut cpu = emu::cpu::Cpu::new(&mut bus, &mut io);

    loop {
        cpu.step(args.testing);
    }
}
