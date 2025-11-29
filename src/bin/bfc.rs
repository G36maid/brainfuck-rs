use std::collections::HashMap;
use std::io::Read;

#[derive(Debug, Clone, Copy)]
enum Op {
    PtrAdd(usize),
    PtrSub(usize),
    ValAdd(u8),
    ValSub(u8),
    Output,
    Input,
    Jz(usize), // Jump if zero
    Jnz,       // Jump if not zero
    Clear,
    MulAdd(isize, u8),
    ScanLeft,
    ScanRight,
}

fn main() {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).unwrap();

    // Filter code
    let code: Vec<u8> = raw.bytes().filter(|c| b"><+-.,[]".contains(c)).collect();

    // 1. Parse (RLE + Clear Loop + Jumps)
    let ops = parse(code);

    // 2. Optimize Loops (MulAdd)
    let ops = optimize_loops(ops);

    // 3. Code Generation
    println!("fn main() {{");
    println!("    #[allow(unused_imports)]");
    println!("    use std::io::{{Read, Write}};");
    println!("    let mut tape = [0u8; 30000];");
    println!("    let mut ptr = 0usize;");

    for op in ops {
        match op {
            Op::PtrAdd(n) => println!("    ptr = ptr.wrapping_add({});", n),
            Op::PtrSub(n) => println!("    ptr = ptr.wrapping_sub({});", n),
            Op::ValAdd(n) => println!("    tape[ptr] = tape[ptr].wrapping_add({});", n),
            Op::ValSub(n) => println!("    tape[ptr] = tape[ptr].wrapping_sub({});", n),
            Op::Output => println!("    std::io::stdout().write_all(&[tape[ptr]]).unwrap();"),
            Op::Input => println!(
                "    std::io::stdin().read_exact(std::slice::from_mut(&mut tape[ptr])).ok();"
            ),
            Op::Jz(_) => println!("    while tape[ptr] != 0 {{"),
            Op::Jnz => println!("    }}"),
            Op::Clear => println!("    tape[ptr] = 0;"),
            Op::MulAdd(offset, factor) => {
                // Generate optimized move loop code
                // tape[ptr + offset] += tape[ptr] * factor;
                // Note: using wrapping arithmetic for safety and BF semantics
                println!("    if tape[ptr] != 0 {{");
                println!(
                    "        let target_idx = ptr.wrapping_add({}usize);",
                    offset as usize
                );
                println!(
                    "        tape[target_idx] = tape[target_idx].wrapping_add(tape[ptr].wrapping_mul({}));",
                    factor
                );
                println!("    }}");
            }
            Op::ScanLeft => {
                println!("    if let Some(pos) = tape[..=ptr].iter().rposition(|&x| x == 0) {{");
                println!("        ptr = pos;");
                println!("    }} else {{");
                println!("        ptr = ptr.wrapping_sub(ptr + 1);");
                println!("    }}");
            }
            Op::ScanRight => {
                println!("    if let Some(pos) = tape[ptr..].iter().position(|&x| x == 0) {{");
                println!("        ptr += pos;");
                println!("    }} else {{");
                println!("        ptr = tape.len();");
                println!("    }}");
            }
        }
    }

    println!("}}");
}

fn parse(code: Vec<u8>) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut loop_stack = Vec::new();
    let mut i = 0;
    let len = code.len();

    while i < len {
        let b = code[i];

        // Optimization: Clear Loop [-] or [+]
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
                ops.push(Op::Jz(0)); // Placeholder
                loop_stack.push(ops.len() - 1);
                i += 1;
            }
            b']' => {
                let start = loop_stack.pop().expect("Unmatched '['");
                let end = ops.len();
                ops.push(Op::Jnz);

                // Backpatch
                if let Op::Jz(target) = &mut ops[start] {
                    *target = end;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }

    if !loop_stack.is_empty() {
        panic!("Unmatched '['");
    }
    ops
}

fn optimize_loops(ops: Vec<Op>) -> Vec<Op> {
    let mut new_ops = Vec::new();
    let mut loop_stack = Vec::new();
    let mut i = 0;

    while i < ops.len() {
        match ops[i] {
            Op::Jz(target) => {
                // Check for Move Loop pattern in the loop body
                // Body is between i+1 and target
                if target > i + 1 {
                    // Ensure body is not empty
                    let body = &ops[i + 1..target];
                    if let Some(scan_op) = check_scan_loop(body) {
                        new_ops.push(scan_op);
                        i = target + 1;
                        continue;
                    } else if let Some(mul_ops) = check_move_loop(body) {
                        new_ops.extend(mul_ops);
                        new_ops.push(Op::Clear);
                        i = target + 1; // Skip the loop
                        continue;
                    }
                }

                // Not a move loop, preserve Jz
                new_ops.push(Op::Jz(0));
                loop_stack.push(new_ops.len() - 1);
                i += 1;
            }
            Op::Jnz => {
                let start = loop_stack.pop().expect("Optimizer: Unmatched ']'");
                let end = new_ops.len();
                new_ops.push(Op::Jnz);

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

fn check_move_loop(body: &[Op]) -> Option<Vec<Op>> {
    let mut ptr_offset: isize = 0;
    let mut deltas: HashMap<isize, i16> = HashMap::new();

    for op in body {
        match op {
            Op::PtrAdd(n) => ptr_offset += *n as isize,
            Op::PtrSub(n) => ptr_offset -= *n as isize,
            Op::ValAdd(n) => *deltas.entry(ptr_offset).or_insert(0) += *n as i16,
            Op::ValSub(n) => *deltas.entry(ptr_offset).or_insert(0) -= *n as i16,
            _ => return None, // Complex ops (IO, inner loops) -> abort
        }
    }

    // Net pointer movement must be zero
    if ptr_offset != 0 {
        return None;
    }

    // Must decrement start cell (offset 0) by 1
    let start_delta = *deltas.get(&0).unwrap_or(&0);
    if (start_delta + 1) % 256 != 0 {
        return None;
    }

    let mut result = Vec::new();
    for (&offset, &delta) in deltas.iter() {
        if offset == 0 {
            continue;
        }
        result.push(Op::MulAdd(offset, delta as u8));
    }

    Some(result)
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
