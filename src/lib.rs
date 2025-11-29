use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    PtrAdd(isize),
    ValAdd(isize, u8),
    ValSub(isize, u8),
    Output,
    Input,
    Jz(usize),
    Jnz(usize),
    Clear(isize),
    MulAdd(isize, u8),
    ScanLeft,
    ScanRight,
}

pub fn parse(code: Vec<u8>) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut loop_stack = Vec::new();
    let mut i = 0;
    let len = code.len();
    let mut current_offset: isize = 0;

    while i < len {
        let b = code[i];

        // Sequence points flush the pointer update
        let is_sequence_point = matches!(b, b'.' | b',' | b'[' | b']');

        // Handle clear loop [-] or [+] specially
        if b == b'['
            && i + 2 < len
            && code[i + 2] == b']'
            && (code[i + 1] == b'-' || code[i + 1] == b'+')
        {
            if current_offset != 0 {
                ops.push(Op::PtrAdd(current_offset));
                current_offset = 0;
            }
            // Clear applies to current pointer, which is implicitly offset 0 after flush
            ops.push(Op::Clear(0));
            i += 3;
            continue;
        }

        if is_sequence_point && current_offset != 0 {
            ops.push(Op::PtrAdd(current_offset));
            current_offset = 0;
        }

        match b {
            b'>' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'>' {
                    count += 1;
                }
                current_offset += count as isize;
                i += count;
            }
            b'<' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'<' {
                    count += 1;
                }
                current_offset -= count as isize;
                i += count;
            }
            b'+' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'+' {
                    count += 1;
                }
                let val = (count % 256) as u8;
                if let Some(Op::ValAdd(off, prev_val)) = ops.last_mut() {
                    if *off == current_offset {
                        *prev_val = prev_val.wrapping_add(val);
                    } else {
                        ops.push(Op::ValAdd(current_offset, val));
                    }
                } else if let Some(Op::ValSub(off, prev_val)) = ops.last_mut() {
                    if *off == current_offset {
                        if *prev_val > val {
                            *prev_val -= val;
                        } else {
                            let rem = val - *prev_val;
                            ops.pop();
                            if rem > 0 {
                                ops.push(Op::ValAdd(current_offset, rem));
                            }
                        }
                    } else {
                        ops.push(Op::ValAdd(current_offset, val));
                    }
                } else {
                    ops.push(Op::ValAdd(current_offset, val));
                }
                i += count;
            }
            b'-' => {
                let mut count = 1;
                while i + count < len && code[i + count] == b'-' {
                    count += 1;
                }
                let val = (count % 256) as u8;
                if let Some(Op::ValSub(off, prev_val)) = ops.last_mut() {
                    if *off == current_offset {
                        *prev_val = prev_val.wrapping_add(val);
                    } else {
                        ops.push(Op::ValSub(current_offset, val));
                    }
                } else if let Some(Op::ValAdd(off, prev_val)) = ops.last_mut() {
                    if *off == current_offset {
                        if *prev_val > val {
                            *prev_val -= val;
                        } else {
                            let rem = val - *prev_val;
                            ops.pop();
                            if rem > 0 {
                                ops.push(Op::ValSub(current_offset, rem));
                            }
                        }
                    } else {
                        ops.push(Op::ValSub(current_offset, val));
                    }
                } else {
                    ops.push(Op::ValSub(current_offset, val));
                }
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
                ops.push(Op::Jz(0));
                loop_stack.push(ops.len() - 1);
                i += 1;
            }
            b']' => {
                let start = loop_stack.pop().expect("Unmatched '['");
                let end = ops.len();
                ops.push(Op::Jnz(start));

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

    if current_offset != 0 {
        ops.push(Op::PtrAdd(current_offset));
    }

    ops
}

pub fn optimize(ops: Vec<Op>) -> Vec<Op> {
    let ops = optimize_loops(ops);
    optimize_dce(ops)
}

fn optimize_loops(ops: Vec<Op>) -> Vec<Op> {
    let mut new_ops = Vec::new();
    let mut loop_stack = Vec::new();
    let mut i = 0;

    while i < ops.len() {
        match ops[i] {
            Op::Jz(target) => {
                let body = &ops[i + 1..target];
                if let Some(scan_op) = check_scan_loop(body) {
                    new_ops.push(scan_op);
                    i = target + 1;
                } else if let Some(mul_ops) = check_move_loop(body) {
                    new_ops.extend(mul_ops);
                    // Move loop implicitly ends with Clear(0)
                    new_ops.push(Op::Clear(0));
                    i = target + 1;
                } else {
                    new_ops.push(Op::Jz(0));
                    loop_stack.push(new_ops.len() - 1);
                    i += 1;
                }
            }
            Op::Jnz(_) => {
                let start = loop_stack.pop().expect("Optimizer: Unmatched ']'");
                let end = new_ops.len();
                new_ops.push(Op::Jnz(start));

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
    let mut known_zero = true;

    while i < ops.len() {
        match ops[i] {
            Op::Jz(target) => {
                if known_zero {
                    i = target + 1;
                } else {
                    new_ops.push(Op::Jz(0));
                    loop_stack.push(new_ops.len() - 1);
                    known_zero = false;
                    i += 1;
                }
            }
            Op::Jnz(_) => {
                let start = loop_stack.pop().expect("Optimizer: Unmatched ']'");
                let end = new_ops.len();
                new_ops.push(Op::Jnz(start));

                if let Op::Jz(t) = &mut new_ops[start] {
                    *t = end;
                }
                known_zero = true;
                i += 1;
            }
            Op::Clear(offset) => {
                if offset == 0 {
                    if !known_zero {
                        new_ops.push(Op::Clear(0));
                        known_zero = true;
                    }
                } else {
                    new_ops.push(Op::Clear(offset));
                }
                i += 1;
            }
            Op::MulAdd(offset, factor) => {
                if !known_zero {
                    new_ops.push(Op::MulAdd(offset, factor));
                    known_zero = false;
                }
                i += 1;
            }
            Op::ScanLeft | Op::ScanRight => {
                if !known_zero {
                    new_ops.push(ops[i]);
                    known_zero = true;
                }
                i += 1;
            }
            Op::PtrAdd(n) => {
                if let Some(Op::PtrAdd(prev)) = new_ops.last_mut() {
                    *prev += n;
                } else {
                    new_ops.push(Op::PtrAdd(n));
                }
                if n != 0 {
                    known_zero = false;
                }
                i += 1;
            }
            Op::ValAdd(offset, n) => {
                if let Some(Op::ValAdd(prev_off, prev_val)) = new_ops.last_mut() {
                    if *prev_off == offset {
                        *prev_val = prev_val.wrapping_add(n);
                        if *prev_val == 0 {
                            new_ops.pop();
                        }
                    } else {
                        new_ops.push(Op::ValAdd(offset, n));
                    }
                } else if let Some(Op::ValSub(prev_off, prev_val)) = new_ops.last_mut() {
                    if *prev_off == offset {
                        if *prev_val > n {
                            *prev_val -= n;
                        } else if *prev_val < n {
                            let rem = n - *prev_val;
                            new_ops.pop();
                            new_ops.push(Op::ValAdd(offset, rem));
                        } else {
                            new_ops.pop();
                        }
                    } else {
                        new_ops.push(Op::ValAdd(offset, n));
                    }
                } else {
                    new_ops.push(Op::ValAdd(offset, n));
                }
                if offset == 0 {
                    known_zero = false;
                }
                i += 1;
            }
            Op::ValSub(offset, n) => {
                if let Some(Op::ValSub(prev_off, prev_val)) = new_ops.last_mut() {
                    if *prev_off == offset {
                        *prev_val = prev_val.wrapping_add(n);
                        if *prev_val == 0 {
                            new_ops.pop();
                        }
                    } else {
                        new_ops.push(Op::ValSub(offset, n));
                    }
                } else if let Some(Op::ValAdd(prev_off, prev_val)) = new_ops.last_mut() {
                    if *prev_off == offset {
                        if *prev_val > n {
                            *prev_val -= n;
                        } else if *prev_val < n {
                            let rem = n - *prev_val;
                            new_ops.pop();
                            new_ops.push(Op::ValSub(offset, rem));
                        } else {
                            new_ops.pop();
                        }
                    } else {
                        new_ops.push(Op::ValSub(offset, n));
                    }
                } else {
                    new_ops.push(Op::ValSub(offset, n));
                }
                if offset == 0 {
                    known_zero = false;
                }
                i += 1;
            }
            Op::Input => {
                new_ops.push(Op::Input);
                known_zero = false;
                i += 1;
            }
            Op::Output => {
                new_ops.push(ops[i]);
                i += 1;
            }
        }
    }

    // Remove PtrAdd(0)
    new_ops.retain(|op| !matches!(op, Op::PtrAdd(0)));

    new_ops
}

fn check_scan_loop(body: &[Op]) -> Option<Op> {
    if body.len() == 1 {
        match body[0] {
            Op::PtrAdd(1) => Some(Op::ScanRight),
            Op::PtrAdd(-1) => Some(Op::ScanLeft),
            _ => None,
        }
    } else {
        None
    }
}

fn check_move_loop(body: &[Op]) -> Option<Vec<Op>> {
    let mut ptr_offset: isize = 0;
    let mut deltas: HashMap<isize, i16> = HashMap::new();

    for op in body {
        match op {
            Op::PtrAdd(n) => ptr_offset += n,
            Op::ValAdd(offset, n) => *deltas.entry(ptr_offset + offset).or_insert(0) += *n as i16,
            Op::ValSub(offset, n) => *deltas.entry(ptr_offset + offset).or_insert(0) -= *n as i16,
            _ => return None,
        }
    }

    if ptr_offset != 0 {
        return None;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_offset() {
        let code = b">+".to_vec();
        let ops = parse(code);
        assert_eq!(ops, vec![Op::ValAdd(1, 1), Op::PtrAdd(1)]);
    }

    #[test]
    fn test_parse_sequence_point() {
        let code = b">.+".to_vec();
        let ops = parse(code);
        assert_eq!(ops, vec![Op::PtrAdd(1), Op::Output, Op::ValAdd(0, 1)]);
    }

    #[test]
    fn test_dce_loop_at_start() {
        let code = b"[->+<].".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::Output]);
    }

    #[test]
    fn test_dce_redundant_clear() {
        let code = b"+[-][-]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::ValAdd(0, 1), Op::Clear(0)]);
    }

    #[test]
    fn test_dce_scan_loop() {
        let code = b"[<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);

        let code = b"+[<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::ValAdd(0, 1), Op::ScanLeft]);
    }

    #[test]
    fn test_dce_move_loop() {
        let code = b"[->+<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);

        let code = b"+[->+<]".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(
            optimized,
            vec![Op::ValAdd(0, 1), Op::MulAdd(1, 1), Op::Clear(0)]
        );
    }

    #[test]
    fn test_merge_ptr_ops() {
        let code = b">>".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::PtrAdd(2)]);

        let code = b">><<".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);

        let code = b">>><".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::PtrAdd(2)]);
    }

    #[test]
    fn test_merge_val_ops() {
        let code = b"++".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![Op::ValAdd(0, 2)]);

        let code = b"++--".to_vec();
        let ops = parse(code);
        let optimized = optimize(ops);
        assert_eq!(optimized, vec![]);
    }
}
