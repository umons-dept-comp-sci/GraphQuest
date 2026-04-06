use std::{
    io,
    str::SplitWhitespace,
};

/// Reads the next value from an iterator and returns it as a float.
fn next_and_parse(iterator: &mut SplitWhitespace<'_>) -> f32 {
    iterator
        .next()
        .expect("next value present")
        .parse()
        .expect("correct float")
}

fn main() {
    // Captures the stdin buffer
    let mut stdin = io::stdin().lines();
    // While the stdin is open, wait and read a line
    while let Some(Ok(signature_r_ag)) = stdin.next() {
        let mut values_iterator = signature_r_ag.split_whitespace();

        // Gets the signature, arithmetic geometric index and randic index.
        let sig = values_iterator.next().expect("signature present");
        let ag = next_and_parse(&mut values_iterator);
        let r = next_and_parse(&mut values_iterator);

        // Computes the conjecture result and stores it as an integer
        //      * 1 if true, 0 otherwise
        let result: usize = (ag <= (2. * r.powi(2)) - r).into();

        // Writes output to stdout
        println!("{sig} {result}"); // Flushes the output by default
    }
}
