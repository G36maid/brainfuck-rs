use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    PtrAdd(usize),
    PtrSub(usize),
    ValAdd(u8),
    ValSub(u8),
    Output,
    Input,
    Jz(usize),         // Jump if zero ( [ ), stores jump target index
    Jnz(usize),        // Jump if not zero ( ] ), stores jump target index
    Clear,             // Optimization for [-]
    MulAdd(isize, u8), // Optimization for move loops: offset, factor
    ScanLeft,          // Optimization for [<]
    ScanRight,         // Optimization for [>]
}

pub fn parse(code: Vec<u8>) -> Vec<Op> {
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

pub fn optimize(ops: Vec<Op>) -> Vec<Op> {
    let ops = optimize_loops(ops);
    optimize_dce(ops)
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

fn optimize_dce(ops: Vec<Op>) -> Vec<Op> {
    let mut new_ops = Vec::new();
    let mut loop_stack = Vec::new();
    let mut i = 0;
    // Tracks if the current cell is known to be zero.
    // At the start of the program, all memory is zero.
    let mut known_zero = true;

    while i < ops.len() {
        match ops[i] {
            Op::Jz(target) => {
                if known_zero {
                    // Dead Code: Loop at known zero will not execute.
                    // Skip the loop entirely.
                    i = target + 1;
                    // known_zero remains true
                } else {
                    new_ops.push(Op::Jz(0)); // Placeholder
                    loop_stack.push(new_ops.len() - 1);
                    // Inside the loop, the cell is not zero (initially).
                    known_zero = false;
                    i += 1;
                }
            }
            Op::Jnz(_) => {
                let start = loop_stack.pop().expect("Optimizer: Unmatched ']'");
                let end = new_ops.len();
                new_ops.push(Op::Jnz(start));

                // Backpatch Jz
                if let Op::Jz(t) = &mut new_ops[start] {
                    *t = end;
                }

                // A loop exits only when the cell becomes zero.
                known_zero = true;
                i += 1;
            }
            Op::Clear => {
                // Clear is redundant if already zero, but we keep it clean or remove it.
                // Removing it is better DCE.
                if !known_zero {
                    new_ops.push(Op::Clear);
                    known_zero = true;
                }
                i += 1;
            }
            Op::MulAdd(offset, factor) => {
                // MulAdd (move loop) effectively adds (cell * factor) to target.
                // It does NOT clear the source cell (an explicit Clear op follows usually).
                if !known_zero {
                    new_ops.push(Op::MulAdd(offset, factor));
                    known_zero = false;
                }
                i += 1;
            }
            Op::ScanLeft | Op::ScanRight => {
                // Scan loops ([<] or [>]) run while cell != 0.
                // If cell is 0, they don't run.
                if !known_zero {
                    new_ops.push(ops[i]);
                    // Scan stops when it finds a zero.
                    known_zero = true;
                }
                i += 1;
            }
            Op::PtrAdd(n) => {
                if let Some(Op::PtrAdd(prev)) = new_ops.last_mut() {
                    *prev += n;
                } else if let Some(Op::PtrSub(prev)) = new_ops.last_mut() {
                    if *prev > n {
                        *prev -= n;
                    } else if *prev < n {
                        let rem = n - *prev;
                        new_ops.pop();
                        new_ops.push(Op::PtrAdd(rem));
                    } else {
                        new_ops.pop();
                    }
                } else {
                    new_ops.push(Op::PtrAdd(n));
                }
                known_zero = false;
                i += 1;
            }
            Op::PtrSub(n) => {
                if let Some(Op::PtrSub(prev)) = new_ops.last_mut() {
                    *prev += n;
                } else if let Some(Op::PtrAdd(prev)) = new_ops.last_mut() {
                    if *prev > n {
                        *prev -= n;
                    } else if *prev < n {
                        let rem = n - *prev;
                        new_ops.pop();
                        new_ops.push(Op::PtrSub(rem));
                    } else {
                        new_ops.pop();
                    }
                } else {
                    new_ops.push(Op::PtrSub(n));
                }
                known_zero = false;
                i += 1;
            }
            Op::ValAdd(n) => {
                if let Some(Op::ValAdd(prev)) = new_ops.last_mut() {
                    *prev = prev.wrapping_add(n);
                    if *prev == 0 {
                        new_ops.pop();
                    }
                } else if let Some(Op::ValSub(prev)) = new_ops.last_mut() {
                    if *prev > n {
                        *prev -= n;
                    } else if *prev < n {
                        let rem = n - *prev;
                        new_ops.pop();
                        new_ops.push(Op::ValAdd(rem));
                    } else {
                        new_ops.pop();
                    }
                } else {
                    new_ops.push(Op::ValAdd(n));
                }
                known_zero = false;
                i += 1;
            }
            Op::ValSub(n) => {
                if let Some(Op::ValSub(prev)) = new_ops.last_mut() {
                    *prev = prev.wrapping_add(n);
                    if *prev == 0 {
                        new_ops.pop();
                    }
                } else if let Some(Op::ValAdd(prev)) = new_ops.last_mut() {
                    if *prev > n {
                        *prev -= n;
                    } else if *prev < n {
                        let rem = n - *prev;
                        new_ops.pop();
                        new_ops.push(Op::ValSub(rem));
                    } else {
                        new_ops.pop();
                    }
                } else {
                    new_ops.push(Op::ValSub(n));
                }
                known_zero = false;
                i += 1;
            }
            Op::Input => {
                new_ops.push(Op::Input);
                known_zero = false;
                i += 1;
            }
            Op::Output => {
                new_ops.push(ops[i]);
                // Output reads but doesn't modify the cell.
                // known_zero state is preserved.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dce_loop_at_start() {
        // Code: [->+<] .
        // Loop at start is dead code because memory is 0.
        // Should optimize to just Output.
        let code = b"[->+<].".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);

        // Expected: [Output]
        assert_eq!(optimized, vec![Op::Output]);
    }

    #[test]
    fn test_dce_redundant_clear() {
        // Code: +[-][-]
        // 1. + (ValAdd) -> known_zero = false
        // 2. [-] (Clear) -> kept, known_zero = true
        // 3. [-] (Clear) -> dead, removed.

        let code = b"+[-][-]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);

        // Expected: [ValAdd(1), Clear]
        assert_eq!(optimized, vec![Op::ValAdd(1), Op::Clear]);
    }

    #[test]
    fn test_dce_scan_loop() {
        // Code: [<]
        // Dead at start.
        let code = b"[<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);

        // Code: +[<]
        // Not dead.
        let code = b"+[<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        // + -> ValAdd(1)
        // [<] -> ScanLeft
        assert_eq!(optimized, vec![Op::ValAdd(1), Op::ScanLeft]);
    }

    #[test]
    fn test_dce_move_loop() {
        // Code: [->+<]
        // Dead at start.
        let code = b"[->+<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);

        // Code: +[->+<]
        // Not dead.
        let code = b"+[->+<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        // + -> ValAdd(1)
        // [->+<] -> MulAdd(1, 1), Clear
        assert_eq!(optimized, vec![Op::ValAdd(1), Op::MulAdd(1, 1), Op::Clear]);
    }

    #[test]
    fn test_merge_ptr_ops() {
        // >> -> PtrAdd(2)
        let code = b">>".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::PtrAdd(2)]);

        // >><< -> empty (cancels out)
        let code = b">><<".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);

        // >>>< -> PtrAdd(2)
        let code = b">>><".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::PtrAdd(2)]);

        // ><< -> PtrSub(1)
        let code = b"><<".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::PtrSub(1)]);
    }

    #[test]
    fn test_merge_val_ops() {
        // ++ -> ValAdd(2)
        let code = b"++".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::ValAdd(2)]);

        // ++-- -> empty
        let code = b"++--".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);

        // +++- -> ValAdd(2)
        let code = b"+++-".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::ValAdd(2)]);

        // +-- -> ValSub(1)
        let code = b"+--".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::ValSub(1)]);
    }
}
