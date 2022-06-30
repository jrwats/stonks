use crate::db::MetricRow;
use crate::quote::Quote;

pub fn get_trs(quotes: &[Quote]) -> Vec<f64> {
    if quotes.len() < 2 {
        return vec![];
    }
    let mut trs = Vec::with_capacity(quotes.len() - 1);
    for (idx, q) in quotes[1..].iter().enumerate() {
        let inter_tr = f64::max(f64::abs(q.high - quotes[idx].close), f64::abs(q.low - quotes[idx].close));
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
        avg = val * alpha + (1.0 - alpha) * avg;
        rmas.push(avg);
    }
    rmas
}

pub fn get_dirmoves(quotes: &[Quote], rma_atrs: &[f64]) -> Vec<f64> {
    vec![]
}

pub fn get_adxs(quotes: &[Quote], dilen: usize, adxlen: usize) -> Vec<f64> {
    if quotes.len() < dilen {
        return vec![]
    }
    let trs = get_trs(quotes);
    let rma_atrs = get_rmas(&trs, dilen);
    let dir_moves = get_dirmoves(quotes, rma_atrs);
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
