use std::collections::HashMap;
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
    Jz(usize),         // Jump if zero ( [ )
    Jnz(usize),        // Jump if not zero ( ] )
    Clear,             // Optimization for [-]
    MulAdd(isize, u8), // Optimization for move loops: offset, factor
    ScanLeft,          // Optimization for [<]
    ScanRight,         // Optimization for [>]
}

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

    // 3. Optimize Loops (MulAdd)
    let ops = optimize_loops(ops);

    // 4. Execution
    execute(ops);
}

fn parse(code: Vec<u8>) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut loop_stack = Vec::new();
    let mut i = 0;
    let len = code.len();

    while i < len {
        let b = code[i];

        // Check for clear loop [-] or [+]
        if b == b'['
            && i + 2 < len
            && code[i + 2] == b']'
            && (code[i + 1] == b'-' || code[i + 1] == b'+')
        {
            ops.push(Op::Clear);
            i += 3;
            continue;
        }

        match b {
            b'>' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'>' {
                    count += 1;
                }
                ops.push(Op::PtrAdd(count));
                i += count;
            }
            b'<' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'<' {
                    count += 1;
                }
                ops.push(Op::PtrSub(count));
                i += count;
            }
            b'+' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'+' {
                    count += 1;
                }
                ops.push(Op::ValAdd((count % 256) as u8));
                i += count;
            }
            b'-' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'-' {
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
                i += 1;
            }
        }
    }

    if !loop_stack.is_empty() {
        panic!("Unmatched '['");
    }
    ops
}

fn optimize_loops(ops: Vec<Op>) -> Vec<Op> {
    let mut new_ops = Vec::new();
    let mut loop_stack = Vec::new(); // Stack stores index in new_ops
    let mut i = 0;

    while i < ops.len() {
        match ops[i] {
            Op::Jz(target) => {
                // Look ahead at the loop body: ops[i+1 .. target]
                // Note: 'target' is the index of Jnz in the *old* ops vector
                let body = &ops[i + 1..target];
                if let Some(scan_op) = check_scan_loop(body) {
                    new_ops.push(scan_op);
                    i = target + 1;
                } else if let Some(mul_ops) = check_move_loop(body) {
                    new_ops.extend(mul_ops);
                    new_ops.push(Op::Clear);
                    i = target + 1; // Skip the entire loop (Jz ... Jnz)
                } else {
                    // Not a move loop, copy Jz
                    new_ops.push(Op::Jz(0)); // Placeholder
                    loop_stack.push(new_ops.len() - 1);
                    i += 1;
                }
            }
            Op::Jnz(_) => {
                let start = loop_stack.pop().expect("Optimizer: Unmatched ']'");
                let end = new_ops.len();
                new_ops.push(Op::Jnz(start));

                // Fix the jump target of the matching Jz
                if let Op::Jz(t) = &mut new_ops[start] {
                    *t = end;
                }
                i += 1;
            }
            other => {
                new_ops.push(other);
                i += 1;
            }
        }
    }
    new_ops
}

fn check_scan_loop(body: &[Op]) -> Option<Op> {
    if body.len() == 1 {
        match body[0] {
            Op::PtrAdd(1) => Some(Op::ScanRight),
            Op::PtrSub(1) => Some(Op::ScanLeft),
            _ => None,
        }
    } else {
        None
    }
}

/// Checks if a loop body is a simple "move loop" pattern (e.g., [->+<]).
/// Returns the list of MulAdd operations if it is.
fn check_move_loop(body: &[Op]) -> Option<Vec<Op>> {
    let mut ptr_offset: isize = 0;
    let mut deltas: HashMap<isize, i16> = HashMap::new();

    for op in body {
        match op {
            Op::PtrAdd(n) => ptr_offset += *n as isize,
            Op::PtrSub(n) => ptr_offset -= *n as isize,
            Op::ValAdd(n) => *deltas.entry(ptr_offset).or_insert(0) += *n as i16,
            Op::ValSub(n) => *deltas.entry(ptr_offset).or_insert(0) -= *n as i16,
            // Any other op means side effects we can't optimize simply
            _ => return None,
        }
    }

    // Net pointer movement must be zero
    if ptr_offset != 0 {
        return None;
    }

    // Must decrement the starting cell by 1 per iteration
    let start_delta = *deltas.get(&0).unwrap_or(&0);
    // -1 (mod 256) check: (delta + 1) should be a multiple of 256
    if (start_delta + 1) % 256 != 0 {
        return None;
    }

    // Generate MulAdd instructions for other cells
    let mut result = Vec::new();
    for (&offset, &delta) in deltas.iter() {
        if offset == 0 {
            continue;
        }
        // delta is the multiplier.
        // e.g. [->++<] adds 2 to offset 1 per iteration. delta=2.
        result.push(Op::MulAdd(offset, delta as u8));
    }

    Some(result)
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
