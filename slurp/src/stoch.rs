use crate::db::MetricRow;
use crate::quote::Quote;

/// True ranges
pub fn get_trs(quotes: &[Quote]) -> Vec<f64> {
    if quotes.len() < 2 {
        return vec![];
    }
    let mut trs = Vec::with_capacity(quotes.len() - 1);
    for (idx, q) in quotes[1..].iter().enumerate() {
        let inter_tr = f64::max(
            f64::abs(q.high - quotes[idx].close),
            f64::abs(q.low - quotes[idx].close),
        );
        let tr = f64::max(q.high - q.low, inter_tr);
        trs.push(tr);
    }
    trs
}

// EMA with alpha = 1 / len
pub fn get_rmas(vals: &[f64], period: usize) -> Vec<f64> {
    if vals.len() < period {
        return vec![];
    }
    let mut rmas = Vec::with_capacity(vals.len() - period + 1);
    let mut avg: f64 = vals[0..period].iter().sum::<f64>() / period as f64;
    let alpha = 1f64 / period as f64;
    rmas.push(avg);
    for val in vals[period..].iter() {
        avg += (val - avg) * alpha;
        rmas.push(avg);
    }
    rmas
}

pub fn get_directional_indicators(quotes: &[Quote], period: usize) -> Vec<f64> {
    let mut pos_dms = Vec::with_capacity(quotes.len() - 1);
    let mut neg_dms = Vec::with_capacity(quotes.len() - 1);
    for (idx, quote) in quotes[1..].iter().enumerate() {
        let up = quote.high - quotes[idx].high;
        let down = quotes[idx].low - quote.low;
        pos_dms.push(if up > down && up > 0.0 { up } else { 0.0 });
        neg_dms.push(if down > up && down > 0.0 { down } else { 0.0 });
    }

    let trs = get_trs(quotes);
    let rma_atrs = get_rmas(&trs, period);
    let rma_pos_dms = get_rmas(&pos_dms, period);
    let rma_neg_dms = get_rmas(&neg_dms, period);
    let calc = |(idx, dm): (usize, &f64)| 100.0 * dm / rma_atrs[idx];
    let pos_dis: Vec<f64> = rma_pos_dms.iter().enumerate().map(calc).collect();
    let neg_dis: Vec<f64> = rma_neg_dms.iter().enumerate().map(calc).collect();
    eprintln!("rma_atr: {}", rma_atrs.last().unwrap());
    eprintln!("pdi: {}", pos_dms.last().unwrap());
    eprintln!("ndi: {}", neg_dms.last().unwrap());
    eprintln!("rpdi: {}", rma_pos_dms.last().unwrap());
    eprintln!("rndi: {}", rma_neg_dms.last().unwrap());
    eprintln!("pos_dis: {}", pos_dis.last().unwrap());
    eprintln!("neg_dis: {}", neg_dis.last().unwrap());
    pos_dis.iter().enumerate().map(|(idx, pdi)| { 
        let ndi = neg_dis[idx];
        let sum = pdi + ndi;
        f64::abs(pdi - ndi) / if sum == 0.0 { 1.0 } else { sum }
    }).collect()
}

pub fn get_adx(quotes: &[Quote], dilen: usize, adxlen: usize) -> f64 {
    if quotes.len() < dilen {
        return -1.0;
    }
    let adxs = get_directional_indicators(quotes, dilen);
    let rma_adxs = get_rmas(&adxs, adxlen);
    rma_adxs.last().map(|adx: &f64| adx * 100.0).unwrap_or(-1.0)
}

pub fn get_smas(vals: &[f64], period: usize) -> Vec<f64> {
    if vals.len() < period {
        return vec![];
    }
    let mut smas = Vec::with_capacity(vals.len() - period + 1);
    let mut sum: f64 = vals[0..period].iter().sum();
    let mut drop_idx = 0;
    smas.push(sum / period as f64);
    for val in vals[period..].iter() {
        sum += val;
        sum -= vals[drop_idx];
        drop_idx += 1;
        smas.push(sum / period as f64);
    }
    smas
}

pub fn get_stochastics(metric_rows: &[MetricRow], k_len: usize) -> Vec<f64> {
    let mut stochs = Vec::with_capacity(metric_rows.len() - k_len as usize + 1);
    for (idx, row) in metric_rows[(k_len as usize - 1)..].iter().enumerate() {
        let mut hi = f64::NEG_INFINITY;
        let mut lo = f64::INFINITY;
        for row in metric_rows[idx..(idx + k_len as usize)].iter() {
            if row.quote.high > hi {
                hi = row.quote.high;
            }
            if row.quote.low < lo {
                lo = row.quote.low;
            }
        }
        stochs.push(100.0 * (row.quote.close - lo) as f64 / (hi - lo) as f64);
    }
    stochs
}

pub fn get_slow_stoch(
    k_len: usize,
    k_smooth: usize,
    d_smooth: usize,
    metric_rows: &[MetricRow],
) -> f64 {
    let stochs = get_stochastics(metric_rows, k_len);
    let ks = get_smas(&stochs, k_smooth);
    let ds = get_smas(&stochs, d_smooth);
    return ds[ds.len() - 1];
}
