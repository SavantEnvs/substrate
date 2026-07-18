#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    use sp_arithmetic::*;
    let a = u32::from_le_bytes(data[0..4].try_into().unwrap());

    // Exercise Permill
    let p = Permill::from_parts(a % 1_000_001);
    let _ = p.deconstruct();

    // Exercise Perbill
    let pb = Perbill::from_parts(a % 1_000_000_001);
    let _ = pb.deconstruct();

    // Exercise Percent
    let pc = Percent::from_parts((a % 101) as u8);
    let _ = pc.deconstruct();

    if data.len() >= 8 {
        let b = u32::from_le_bytes(data[4..8].try_into().unwrap());

        // Saturating arithmetic
        let _ = a.saturating_add(b);
        let _ = a.saturating_mul(b);

        // Rational128
        if b != 0 {
            let r = Rational128::from(a as u128, b as u128);
            let _ = r.n();
            let _ = r.d();
        }

        // FixedU128
        let fu = FixedU128::from_inner(a as u128);
        let _ = fu.into_inner();

        // FixedI128
        let fi = FixedI128::from_inner(a as i128);
        let _ = fi.into_inner();
    }
});
