const PART_MASK: u32 = 0x00ff_00ff;
const Y_MASK: u32 = 0x00ff_0000;
const U_MASK: u32 = 0x0000_ff00;
const V_MASK: u32 = 0x0000_00ff;

pub(super) fn upscale(
    input: &[u32],
    width: usize,
    height: usize,
    scale: usize,
    output: &mut [u32],
) -> bool {
    if !(2..=4).contains(&scale)
        || input.len() != width * height
        || output.len() != width * height * scale * scale
    {
        return false;
    }

    let output_width = width * scale;
    let nl = output_width;
    let nl1 = nl + nl;
    let nl2 = nl1 + nl;

    for y in 0..height {
        for x in 0..width {
            let b1 = sample(input, width, height, x, y, 0, -2);
            let pb = sample(input, width, height, x, y, 0, -1);
            let pe = sample(input, width, height, x, y, 0, 0);
            let ph = sample(input, width, height, x, y, 0, 1);
            let h5 = sample(input, width, height, x, y, 0, 2);

            let a1 = sample(input, width, height, x, y, -1, -2);
            let pa = sample(input, width, height, x, y, -1, -1);
            let pd = sample(input, width, height, x, y, -1, 0);
            let pg = sample(input, width, height, x, y, -1, 1);
            let g5 = sample(input, width, height, x, y, -1, 2);

            let a0 = sample(input, width, height, x, y, -2, -1);
            let d0 = sample(input, width, height, x, y, -2, 0);
            let g0 = sample(input, width, height, x, y, -2, 1);

            let c1 = sample(input, width, height, x, y, 1, -2);
            let pc = sample(input, width, height, x, y, 1, -1);
            let pf = sample(input, width, height, x, y, 1, 0);
            let pi = sample(input, width, height, x, y, 1, 1);
            let i5 = sample(input, width, height, x, y, 1, 2);

            let c4 = sample(input, width, height, x, y, 2, -1);
            let f4 = sample(input, width, height, x, y, 2, 0);
            let i4 = sample(input, width, height, x, y, 2, 1);

            let output_offset = y * scale * output_width + x * scale;
            let e = &mut output[output_offset..];

            match scale {
                2 => {
                    e[0] = pe;
                    e[1] = pe;
                    e[nl] = pe;
                    e[nl + 1] = pe;

                    filt2(
                        e,
                        pe,
                        pi,
                        ph,
                        pf,
                        pg,
                        pc,
                        pd,
                        pb,
                        f4,
                        i4,
                        h5,
                        i5,
                        0,
                        1,
                        nl,
                        nl + 1,
                    );
                    filt2(
                        e,
                        pe,
                        pc,
                        pf,
                        pb,
                        pi,
                        pa,
                        ph,
                        pd,
                        b1,
                        c1,
                        f4,
                        c4,
                        nl,
                        0,
                        nl + 1,
                        1,
                    );
                    filt2(
                        e,
                        pe,
                        pa,
                        pb,
                        pd,
                        pc,
                        pg,
                        pf,
                        ph,
                        d0,
                        a0,
                        b1,
                        a1,
                        nl + 1,
                        nl,
                        1,
                        0,
                    );
                    filt2(
                        e,
                        pe,
                        pg,
                        pd,
                        ph,
                        pa,
                        pi,
                        pb,
                        pf,
                        h5,
                        g5,
                        d0,
                        g0,
                        1,
                        nl + 1,
                        0,
                        nl,
                    );
                }
                3 => {
                    e[0] = pe;
                    e[1] = pe;
                    e[2] = pe;
                    e[nl] = pe;
                    e[nl + 1] = pe;
                    e[nl + 2] = pe;
                    e[nl1] = pe;
                    e[nl1 + 1] = pe;
                    e[nl1 + 2] = pe;

                    filt3(
                        e,
                        pe,
                        pi,
                        ph,
                        pf,
                        pg,
                        pc,
                        pd,
                        pb,
                        f4,
                        i4,
                        h5,
                        i5,
                        0,
                        1,
                        2,
                        nl,
                        nl + 1,
                        nl + 2,
                        nl1,
                        nl1 + 1,
                        nl1 + 2,
                    );
                    filt3(
                        e,
                        pe,
                        pc,
                        pf,
                        pb,
                        pi,
                        pa,
                        ph,
                        pd,
                        b1,
                        c1,
                        f4,
                        c4,
                        nl1,
                        nl,
                        0,
                        nl1 + 1,
                        nl + 1,
                        1,
                        nl1 + 2,
                        nl + 2,
                        2,
                    );
                    filt3(
                        e,
                        pe,
                        pa,
                        pb,
                        pd,
                        pc,
                        pg,
                        pf,
                        ph,
                        d0,
                        a0,
                        b1,
                        a1,
                        nl1 + 2,
                        nl1 + 1,
                        nl1,
                        nl + 2,
                        nl + 1,
                        nl,
                        2,
                        1,
                        0,
                    );
                    filt3(
                        e,
                        pe,
                        pg,
                        pd,
                        ph,
                        pa,
                        pi,
                        pb,
                        pf,
                        h5,
                        g5,
                        d0,
                        g0,
                        2,
                        nl + 2,
                        nl1 + 2,
                        1,
                        nl + 1,
                        nl1 + 1,
                        0,
                        nl,
                        nl1,
                    );
                }
                4 => {
                    e[0] = pe;
                    e[1] = pe;
                    e[2] = pe;
                    e[3] = pe;
                    e[nl] = pe;
                    e[nl + 1] = pe;
                    e[nl + 2] = pe;
                    e[nl + 3] = pe;
                    e[nl1] = pe;
                    e[nl1 + 1] = pe;
                    e[nl1 + 2] = pe;
                    e[nl1 + 3] = pe;
                    e[nl2] = pe;
                    e[nl2 + 1] = pe;
                    e[nl2 + 2] = pe;
                    e[nl2 + 3] = pe;

                    filt4(
                        e,
                        pe,
                        pi,
                        ph,
                        pf,
                        pg,
                        pc,
                        pd,
                        pb,
                        f4,
                        i4,
                        h5,
                        i5,
                        nl2 + 3,
                        nl2 + 2,
                        nl1 + 3,
                        3,
                        nl + 3,
                        nl1 + 2,
                        nl2 + 1,
                        nl2,
                        nl1 + 1,
                        nl + 2,
                        2,
                        1,
                        nl + 1,
                        nl1,
                        nl,
                        0,
                    );
                    filt4(
                        e,
                        pe,
                        pc,
                        pf,
                        pb,
                        pi,
                        pa,
                        ph,
                        pd,
                        b1,
                        c1,
                        f4,
                        c4,
                        3,
                        nl + 3,
                        2,
                        0,
                        1,
                        nl + 2,
                        nl1 + 3,
                        nl2 + 3,
                        nl1 + 2,
                        nl + 1,
                        nl,
                        nl1,
                        nl1 + 1,
                        nl2 + 2,
                        nl2 + 1,
                        nl2,
                    );
                    filt4(
                        e,
                        pe,
                        pa,
                        pb,
                        pd,
                        pc,
                        pg,
                        pf,
                        ph,
                        d0,
                        a0,
                        b1,
                        a1,
                        0,
                        1,
                        nl,
                        nl2,
                        nl1,
                        nl + 1,
                        2,
                        3,
                        nl + 2,
                        nl1 + 1,
                        nl2 + 1,
                        nl2 + 2,
                        nl1 + 2,
                        nl + 3,
                        nl1 + 3,
                        nl2 + 3,
                    );
                    filt4(
                        e,
                        pe,
                        pg,
                        pd,
                        ph,
                        pa,
                        pi,
                        pb,
                        pf,
                        h5,
                        g5,
                        d0,
                        g0,
                        nl2,
                        nl1,
                        nl2 + 1,
                        nl2 + 3,
                        nl2 + 2,
                        nl1 + 1,
                        nl,
                        0,
                        nl + 1,
                        nl1 + 2,
                        nl1 + 3,
                        nl + 3,
                        nl + 2,
                        1,
                        2,
                        3,
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    true
}

fn sample(
    input: &[u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    dx: isize,
    dy: isize,
) -> u32 {
    let x = x.saturating_add_signed(dx).min(width - 1);
    let y = y.saturating_add_signed(dy).min(height - 1);
    input[y * width + x]
}

fn filt2(
    e: &mut [u32],
    pe: u32,
    pi: u32,
    ph: u32,
    pf: u32,
    pg: u32,
    pc: u32,
    pd: u32,
    pb: u32,
    f4: u32,
    i4: u32,
    h5: u32,
    i5: u32,
    n0: usize,
    n1: usize,
    n2: usize,
    n3: usize,
) {
    if pe != ph && pe != pf {
        let exterior = pixel_diff(pe, pc)
            + pixel_diff(pe, pg)
            + pixel_diff(pi, h5)
            + pixel_diff(pi, f4)
            + (pixel_diff(ph, pf) << 2);
        let interior = pixel_diff(ph, pd)
            + pixel_diff(ph, i5)
            + pixel_diff(pf, i4)
            + pixel_diff(pf, pb)
            + (pixel_diff(pe, pi) << 2);

        if exterior <= interior {
            let px = if pixel_diff(pe, pf) <= pixel_diff(pe, ph) {
                pf
            } else {
                ph
            };

            if exterior < interior
                && ((!equal(pf, pb) && !equal(ph, pd))
                    || (equal(pe, pi) && (!equal(pf, i4) && !equal(ph, i5)))
                    || equal(pe, pg)
                    || equal(pe, pc))
            {
                let ke = pixel_diff(pf, pg);
                let ki = pixel_diff(ph, pc);
                let left = (ke << 1) <= ki && pe != pg && pd != pg;
                let up = ke >= (ki << 1) && pe != pc && pb != pc;

                if left && up {
                    e[n3] = alpha_blend_224(e[n3], px);
                    e[n2] = alpha_blend_64(e[n2], px);
                    e[n1] = e[n2];
                } else if left {
                    e[n3] = alpha_blend_192(e[n3], px);
                    e[n2] = alpha_blend_64(e[n2], px);
                } else if up {
                    e[n3] = alpha_blend_192(e[n3], px);
                    e[n1] = alpha_blend_64(e[n1], px);
                } else {
                    e[n3] = alpha_blend_128(e[n3], px);
                }
            } else {
                e[n3] = alpha_blend_128(e[n3], px);
            }
        }
    }

    let _ = n0;
}

fn filt3(
    e: &mut [u32],
    pe: u32,
    pi: u32,
    ph: u32,
    pf: u32,
    pg: u32,
    pc: u32,
    pd: u32,
    pb: u32,
    f4: u32,
    i4: u32,
    h5: u32,
    i5: u32,
    n0: usize,
    n1: usize,
    n2: usize,
    n3: usize,
    n4: usize,
    n5: usize,
    n6: usize,
    n7: usize,
    n8: usize,
) {
    if pe != ph && pe != pf {
        let exterior = pixel_diff(pe, pc)
            + pixel_diff(pe, pg)
            + pixel_diff(pi, h5)
            + pixel_diff(pi, f4)
            + (pixel_diff(ph, pf) << 2);
        let interior = pixel_diff(ph, pd)
            + pixel_diff(ph, i5)
            + pixel_diff(pf, i4)
            + pixel_diff(pf, pb)
            + (pixel_diff(pe, pi) << 2);

        if exterior <= interior {
            let px = if pixel_diff(pe, pf) <= pixel_diff(pe, ph) {
                pf
            } else {
                ph
            };

            if exterior < interior
                && ((!equal(pf, pb) && !equal(pf, pc))
                    || (!equal(ph, pd) && !equal(ph, pg))
                    || (equal(pe, pi)
                        && ((!equal(pf, f4) && !equal(pf, i4))
                            || (!equal(ph, h5) && !equal(ph, i5))))
                    || equal(pe, pg)
                    || equal(pe, pc))
            {
                let ke = pixel_diff(pf, pg);
                let ki = pixel_diff(ph, pc);
                let left = (ke << 1) <= ki && pe != pg && pd != pg;
                let up = ke >= (ki << 1) && pe != pc && pb != pc;

                if left && up {
                    e[n7] = alpha_blend_192(e[n7], px);
                    e[n6] = alpha_blend_64(e[n6], px);
                    e[n5] = e[n7];
                    e[n2] = e[n6];
                    e[n8] = px;
                } else if left {
                    e[n7] = alpha_blend_192(e[n7], px);
                    e[n5] = alpha_blend_64(e[n5], px);
                    e[n6] = alpha_blend_64(e[n6], px);
                    e[n8] = px;
                } else if up {
                    e[n5] = alpha_blend_192(e[n5], px);
                    e[n7] = alpha_blend_64(e[n7], px);
                    e[n2] = alpha_blend_64(e[n2], px);
                    e[n8] = px;
                } else {
                    e[n8] = alpha_blend_224(e[n8], px);
                    e[n5] = alpha_blend_32(e[n5], px);
                    e[n7] = alpha_blend_32(e[n7], px);
                }
            } else {
                e[n8] = alpha_blend_128(e[n8], px);
            }
        }
    }

    let _ = (n0, n1, n3, n4);
}

fn filt4(
    e: &mut [u32],
    pe: u32,
    pi: u32,
    ph: u32,
    pf: u32,
    pg: u32,
    pc: u32,
    pd: u32,
    pb: u32,
    f4: u32,
    i4: u32,
    h5: u32,
    i5: u32,
    n15: usize,
    n14: usize,
    n11: usize,
    n3: usize,
    n7: usize,
    n10: usize,
    n13: usize,
    n12: usize,
    n9: usize,
    n6: usize,
    n2: usize,
    n1: usize,
    n5: usize,
    n8: usize,
    n4: usize,
    n0: usize,
) {
    if pe != ph && pe != pf {
        let exterior = pixel_diff(pe, pc)
            + pixel_diff(pe, pg)
            + pixel_diff(pi, h5)
            + pixel_diff(pi, f4)
            + (pixel_diff(ph, pf) << 2);
        let interior = pixel_diff(ph, pd)
            + pixel_diff(ph, i5)
            + pixel_diff(pf, i4)
            + pixel_diff(pf, pb)
            + (pixel_diff(pe, pi) << 2);

        if exterior <= interior {
            let px = if pixel_diff(pe, pf) <= pixel_diff(pe, ph) {
                pf
            } else {
                ph
            };

            if exterior < interior
                && ((!equal(pf, pb) && !equal(ph, pd))
                    || (equal(pe, pi) && (!equal(pf, i4) && !equal(ph, i5)))
                    || equal(pe, pg)
                    || equal(pe, pc))
            {
                let ke = pixel_diff(pf, pg);
                let ki = pixel_diff(ph, pc);
                let left = (ke << 1) <= ki && pe != pg && pd != pg;
                let up = ke >= (ki << 1) && pe != pc && pb != pc;

                if left && up {
                    e[n13] = alpha_blend_192(e[n13], px);
                    e[n12] = alpha_blend_64(e[n12], px);
                    e[n15] = px;
                    e[n14] = px;
                    e[n11] = px;
                    e[n10] = e[n12];
                    e[n3] = e[n12];
                    e[n7] = e[n13];
                } else if left {
                    e[n11] = alpha_blend_192(e[n11], px);
                    e[n13] = alpha_blend_192(e[n13], px);
                    e[n10] = alpha_blend_64(e[n10], px);
                    e[n12] = alpha_blend_64(e[n12], px);
                    e[n14] = px;
                    e[n15] = px;
                } else if up {
                    e[n14] = alpha_blend_192(e[n14], px);
                    e[n7] = alpha_blend_192(e[n7], px);
                    e[n10] = alpha_blend_64(e[n10], px);
                    e[n3] = alpha_blend_64(e[n3], px);
                    e[n11] = px;
                    e[n15] = px;
                } else {
                    e[n11] = alpha_blend_128(e[n11], px);
                    e[n14] = alpha_blend_128(e[n14], px);
                    e[n15] = px;
                }
            } else {
                e[n15] = alpha_blend_128(e[n15], px);
            }
        }
    }

    let _ = (n9, n6, n2, n1, n5, n8, n4, n0);
}

fn pixel_diff(x: u32, y: u32) -> u32 {
    let yuv1 = rgb_to_yuv(x);
    let yuv2 = rgb_to_yuv(y);
    let yuv1_y = yuv1 & Y_MASK;
    let yuv1_u = yuv1 & U_MASK;
    let yuv1_v = yuv1 & V_MASK;
    let yuv2_y = yuv2 & Y_MASK;
    let yuv2_u = yuv2 & U_MASK;
    let yuv2_v = yuv2 & V_MASK;

    (x >> 24).abs_diff(y >> 24)
        + (yuv1_y.abs_diff(yuv2_y) >> 16)
        + (yuv1_u.abs_diff(yuv2_u) >> 8)
        + yuv1_v.abs_diff(yuv2_v)
}

fn equal(x: u32, y: u32) -> bool {
    pixel_diff(x, y) < 155
}

fn rgb_to_yuv(pixel: u32) -> u32 {
    let r = ((pixel >> 16) & 0xff) as i32;
    let g = ((pixel >> 8) & 0xff) as i32;
    let b = (pixel & 0xff) as i32;
    let y = (299 * r + 587 * g + 114 * b) / 1000;
    let u = (-169 * r - 331 * g + 500 * b) / 1000 + 128;
    let v = (500 * r - 419 * g - 81 * b) / 1000 + 128;

    ((y as u32) << 16) | ((u as u32) << 8) | v as u32
}

fn alpha_blend_base(a: u32, b: u32, multiplier: u32, shift: u32) -> u32 {
    let low = (a & PART_MASK).wrapping_add(
        ((b & PART_MASK)
            .wrapping_sub(a & PART_MASK)
            .wrapping_mul(multiplier))
            >> shift,
    ) & PART_MASK;
    let a_high = (a >> 8) & PART_MASK;
    let b_high = (b >> 8) & PART_MASK;
    let high = (a_high
        .wrapping_add((b_high.wrapping_sub(a_high).wrapping_mul(multiplier)) >> shift)
        & PART_MASK)
        << 8;

    low | high
}

fn alpha_blend_32(a: u32, b: u32) -> u32 {
    alpha_blend_base(a, b, 1, 3)
}

fn alpha_blend_64(a: u32, b: u32) -> u32 {
    alpha_blend_base(a, b, 1, 2)
}

fn alpha_blend_128(a: u32, b: u32) -> u32 {
    alpha_blend_base(a, b, 1, 1)
}

fn alpha_blend_192(a: u32, b: u32) -> u32 {
    alpha_blend_base(a, b, 3, 2)
}

fn alpha_blend_224(a: u32, b: u32) -> u32 {
    alpha_blend_base(a, b, 7, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_colour_upscales_to_the_same_colour() {
        for scale in 2..=4 {
            let input = vec![0x0012_3456; 4];
            let mut output = vec![0; input.len() * scale * scale];

            assert!(upscale(&input, 2, 2, scale, &mut output));

            assert!(output.iter().all(|pixel| *pixel == 0x0012_3456));
        }
    }

    #[test]
    fn invalid_scale_is_rejected() {
        let input = vec![0; 4];
        let mut output = vec![0; 4];

        assert!(!upscale(&input, 2, 2, 1, &mut output));
    }

    #[test]
    fn rgb_to_yuv_matches_the_legacy_table_formula_for_known_colours() {
        assert_eq!(rgb_to_yuv(0x0000_0000), 0x0000_8080);
        assert_eq!(rgb_to_yuv(0x00ff_ffff), 0x00ff_8080);
        assert_eq!(rgb_to_yuv(0x00ff_0000), 0x004c_55ff);
    }
}
