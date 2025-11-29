use std::env;
use std::io::{self, Read, Write};

#[derive(Debug, Clone, Copy)]
enum Op {
    PtrAdd(usize),
    PtrSub(usize),
    ValAdd(u8),
    ValSub(u8),
    Output,
    Input,
    Jz(usize),  // Jump if zero ( [ )
    Jnz(usize), // Jump if not zero ( ] )
}

fn main() {
    // 1. Load Code
    let source = env::args().nth(1).expect("Usage: ./bf <file>");
    let raw_code = std::fs::read(source).unwrap();

    // 2. Parse & Optimize (RLE)
    let mut ops = Vec::new();
    let mut loop_stack = Vec::new();
    let mut i = 0;
    let len = raw_code.len();

    while i < len {
        match raw_code[i] {
            b'>' => {
                let mut count = 1;
                while i + count < len && raw_code[i + count] == b'>' {
                    count += 1;
                }
                ops.push(Op::PtrAdd(count));
                i += count;
            }
            b'<' => {
                let mut count = 1;
                while i + count < len && raw_code[i + count] == b'<' {
                    count += 1;
                }
                ops.push(Op::PtrSub(count));
                i += count;
            }
            b'+' => {
                let mut count = 1;
                while i + count < len && raw_code[i + count] == b'+' {
                    count += 1;
                }
                // Modulo 256 for wrapping behavior optimization
                ops.push(Op::ValAdd((count % 256) as u8));
                i += count;
            }
            b'-' => {
                let mut count = 1;
                while i + count < len && raw_code[i + count] == b'-' {
                    count += 1;
                }
                ops.push(Op::ValSub((count % 256) as u8));
                i += count;
            }
            b'.' => {
                ops.push(Op::Output);
                i += 1;
            }
            b',' => {
                ops.push(Op::Input);
                i += 1;
            }
            b'[' => {
                ops.push(Op::Jz(0)); // Placeholder target
                loop_stack.push(ops.len() - 1);
                i += 1;
            }
            b']' => {
                let start = loop_stack.pop().expect("Unmatched '['");
                let end = ops.len(); // Index of this Jnz instruction
                ops.push(Op::Jnz(start));

                // Backpatch the opening bracket to jump to here
                match &mut ops[start] {
                    Op::Jz(target) => *target = end,
                    _ => unreachable!(),
                }
                i += 1;
            }
            _ => {
                // Ignore comments and other characters
                i += 1;
            }
        }
    }

    if !loop_stack.is_empty() {
        panic!("Unmatched '['");
    }

    // 3. Execution
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
                // Read 1 byte. Ignore errors (like EOF) to match typical BF behavior/original impl
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
        }
        pc += 1;
    }
}
