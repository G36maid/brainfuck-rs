use std::io::Read;

enum Op {
    PtrAdd(usize),
    PtrSub(usize),
    ValAdd(u8),
    ValSub(u8),
    Output,
    Input,
    LoopStart,
    LoopEnd,
    Clear,
}

fn main() {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).unwrap();

    // Filter code first so RLE and pattern matching works on clean sequence
    let code: Vec<u8> = raw.bytes().filter(|c| b"><+-.,[]".contains(c)).collect();

    let mut ops = Vec::new();
    let mut i = 0;
    let len = code.len();

    while i < len {
        let b = code[i];

        // Optimization: Clear Loop [-] or [+]
        if b == b'[' && i + 2 < len && code[i + 2] == b']' {
            if code[i + 1] == b'-' || code[i + 1] == b'+' {
                ops.push(Op::Clear);
                i += 3;
                continue;
            }
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
                ops.push(Op::LoopStart);
                i += 1;
            }
            b']' => {
                ops.push(Op::LoopEnd);
                i += 1;
            }
            _ => {
                // Should not occur due to filtering
                i += 1;
            }
        }
    }

    // Code Generation
    println!("fn main() {{");
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
            Op::LoopStart => println!("    while tape[ptr] != 0 {{"),
            Op::LoopEnd => println!("    }}"),
            Op::Clear => println!("    tape[ptr] = 0;"),
        }
    }

    println!("}}");
}
