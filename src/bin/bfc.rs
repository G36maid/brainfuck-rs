use brainfuck_rs::{Op, optimize, parse};
use std::io::Read;

fn main() {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).unwrap();

    // Filter code
    let code: Vec<u8> = raw.bytes().filter(|c| b"><+-.,[]".contains(c)).collect();

    // 1. Parse (RLE + Clear Loop + Jumps)
    let ops = parse(code);

    // 2. Optimize (Loops + DCE)
    let ops = optimize(ops);

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
            Op::Jnz(_) => println!("    }}"),
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
