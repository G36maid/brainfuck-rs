use std::io::Read;

fn main() {
    let mut code = String::new();
    std::io::stdin().read_to_string(&mut code).unwrap();

    println!("fn main() {{");
    println!("    use std::io::{{Read, Write}};");
    println!("    let mut tape = [0u8; 30000];");
    println!("    let mut ptr = 0usize;");

    for c in code.chars() {
        match c {
            '>' => println!("    ptr = ptr.wrapping_add(1);"),
            '<' => println!("    ptr = ptr.wrapping_sub(1);"),
            '+' => println!("    tape[ptr] = tape[ptr].wrapping_add(1);"),
            '-' => println!("    tape[ptr] = tape[ptr].wrapping_sub(1);"),
            '.' => println!("    std::io::stdout().write_all(&[tape[ptr]]).unwrap();"),
            ',' => println!(
                "    std::io::stdin().read_exact(std::slice::from_mut(&mut tape[ptr])).ok();"
            ),
            '[' => println!("    while tape[ptr] != 0 {{"),
            ']' => println!("    }}"),
            _ => {}
        }
    }
    println!("}}");
}
