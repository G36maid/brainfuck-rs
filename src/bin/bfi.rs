use brainfuck_rs::{Op, optimize, parse};
use std::env;
use std::io::{self, Read, Write};

fn main() {
    // 1. Load & Filter Code
    let source = env::args().nth(1).expect("Usage: ./bf <file>");
    let raw = std::fs::read(source).unwrap();
    let code: Vec<u8> = raw
        .into_iter()
        .filter(|c| b"><+-.,[]".contains(c))
        .collect();

    // 2. Parse (RLE + Clear Loop)
    let ops = parse(code);

    // 3. Optimize (Loops + DCE)
    let ops = optimize(ops);

    // 4. Execution
    execute(ops);
}

fn execute(ops: Vec<Op>) {
    let mut pc = 0;
    let mut ptr: usize = 0;
    let mut tape = vec![0u8; 30_000];

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut stdin = io::stdin();

    while pc < ops.len() {
        match ops[pc] {
            Op::PtrAdd(n) => ptr = ptr.wrapping_add(n),
            Op::PtrSub(n) => ptr = ptr.wrapping_sub(n),
            Op::ValAdd(n) => tape[ptr] = tape[ptr].wrapping_add(n),
            Op::ValSub(n) => tape[ptr] = tape[ptr].wrapping_sub(n),
            Op::Output => {
                out.write_all(&[tape[ptr]]).unwrap();
                out.flush().unwrap();
            }
            Op::Input => {
                let _ = stdin.read_exact(std::slice::from_mut(&mut tape[ptr]));
            }
            Op::Jz(target) => {
                if tape[ptr] == 0 {
                    pc = target;
                }
            }
            Op::Jnz(target) => {
                if tape[ptr] != 0 {
                    pc = target;
                }
            }
            Op::Clear => {
                tape[ptr] = 0;
            }
            Op::MulAdd(offset, factor) => {
                if tape[ptr] != 0 {
                    // target_ptr = ptr + offset
                    let target_ptr = ptr.wrapping_add(offset as usize);

                    // Standard Brainfuck tape is often unchecked or cyclic.
                    // Here we respect the 30k buffer size.
                    // Panic if OOB is standard behavior for Vec access.
                    tape[target_ptr] =
                        tape[target_ptr].wrapping_add(tape[ptr].wrapping_mul(factor));
                }
            }
            Op::ScanLeft => {
                if let Some(pos) = tape[..=ptr].iter().rposition(|&x| x == 0) {
                    ptr = pos;
                } else {
                    ptr = ptr.wrapping_sub(ptr + 1);
                }
            }
            Op::ScanRight => {
                if let Some(pos) = tape[ptr..].iter().position(|&x| x == 0) {
                    ptr += pos;
                } else {
                    ptr = tape.len();
                }
            }
        }
        pc += 1;
    }
}
