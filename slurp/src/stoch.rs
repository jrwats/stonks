use crate::db::MetricRow;
use crate::quote::Quote;

/// True ranges
pub fn get_true_ranges(quotes: &[Quote]) -> Vec<f64> {
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

// Relative Moving Average
// EMA with alpha = 1 / period
pub fn get_rmas(vals: &[f64], period: usize) -> Vec<f64> {
    if vals.len() < period {
        return vec![];
    }
    let mut rmas = Vec::with_capacity(vals.len() - period + 1);
    let mut avg: f64 = vals[0..period].iter().sum::<f64>() / period as f64;
    let alpha = 1f64 / period as f64;
    rmas.push(avg);
    for val in vals[period..].iter() {
        // avg += (val - avg) * alpha;
        avg = alpha * val + (1.0 - alpha) * avg;
        rmas.push(avg);
    }
    rmas
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

pub fn get_adxs(quotes: &[Quote], period: usize) -> Vec<f64> {
    let mut pos_dms = Vec::with_capacity(quotes.len() - 1);
    let mut neg_dms = Vec::with_capacity(quotes.len() - 1);
    for (prev_idx, quote) in quotes[1..].iter().enumerate() {
        let up = quote.high - quotes[prev_idx].high;
        let down = quotes[prev_idx].low - quote.low;
        pos_dms.push(if up > down && up > 0.0 { up } else { 0.0 });
        neg_dms.push(if down > up && down > 0.0 { down } else { 0.0 });
    }

    let true_ranges = get_true_ranges(quotes);
    let atrs = get_rmas(&true_ranges, period);
    let rma_pos_dms = get_rmas(&pos_dms, period);
    let rma_neg_dms = get_rmas(&neg_dms, period);

    let calc = |(idx, rma_dm): (usize, &f64)| rma_dm * 100.0 / atrs[idx];
    let pos_dis: Vec<f64> = rma_pos_dms.iter().enumerate().map(calc).collect();
    let neg_dis: Vec<f64> = rma_neg_dms.iter().enumerate().map(calc).collect();

    // eprintln!("quotes len: {}", quotes.len());
    // eprintln!("rma_atr: {}", atrs.last().unwrap());
    // eprintln!("pdi: {}", pos_dms.last().unwrap());
    // eprintln!("ndi: {}", neg_dms.last().unwrap());
    // eprintln!("rpdi: {}", rma_pos_dms.last().unwrap());
    // eprintln!("rndi: {}", rma_neg_dms.last().unwrap());
    // eprintln!("pos_dis: {}", pos_dis.last().unwrap());
    // eprintln!("neg_dis: {}", neg_dis.last().unwrap());
    // eprintln!("ATR: {}", atrs.last().unwrap());

    pos_dis
        .iter()
        .enumerate()
        .map(|(idx, pdi)| {
            let ndi = neg_dis[idx];
            let sum = pdi + ndi;
            let denom = if sum == 0.0 { 1.0 } else { sum };
            f64::abs(pdi - ndi) / denom
        })
        .collect()
}

pub fn get_adxr(quotes: &[Quote], dilen: usize, adxlen: usize) -> f64 {
    if quotes.len() < dilen {
        return -1.0;
    }
    let adxs = get_adxs(quotes, dilen);
    let rma_adxs = get_rmas(&adxs, adxlen);
    rma_adxs.last().map(|adx: &f64| adx * 100.0).unwrap_or(-1.0)
}

pub fn get_rsis(quotes: &[Quote], period: usize) -> Vec<f64> {
    // up = ta.rma(math.max(ta.change(close), 0), period)
    // down = ta.rma(-math.min(ta.change(close), 0), period)
    // rsi = down == 0 ? 100 : up == 0 ? 0 : 100 - (100 / (1 + up / down))
    let mut raw_ups = Vec::with_capacity(quotes.len() - 1);
    let mut raw_downs = Vec::with_capacity(quotes.len() - 1);
    for (idx, quote) in quotes[1..].iter().enumerate() {
        let change = quote.close - quotes[idx].close;
        raw_ups.push(change.max(0.0));
        raw_downs.push(-change.min(0.0));
    }
    let ups = get_rmas(&raw_ups, period);
    let downs = get_rmas(&raw_downs, period);
    ups.into_iter()
        .zip(downs)
        .map(|(up, down)| {
            if down == 0.0 {
                return 100.0;
            } else if up == 0.0 {
                return 0.0;
            }
            100.0 - (100.0 / (1.0 + up / down))
        })
        .collect()
}

pub fn get_last_rsi(quotes: &[Quote], period: usize) -> f64 {
    *get_rsis(quotes, period).last().unwrap()
}

pub fn get_stochastics(metric_rows: &[MetricRow], k_len: usize) -> Vec<f64> {
    let mut stochs = Vec::with_capacity(metric_rows.len() - k_len + 1);
    for (idx, row) in metric_rows[(k_len - 1)..].iter().enumerate() {
        let mut hi = f64::NEG_INFINITY;
        let mut lo = f64::INFINITY;
        for row in metric_rows[idx..(idx + k_len)].iter() {
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
    let ds = get_smas(&ks, d_smooth);
    return ds[ds.len() - 1];
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_relative_moving_avgs() {
        let vals = [1.0, 2.0, 3.0, 4.0, 5.0];
        let rmas = get_rmas(&vals, 4);
        assert_eq!(rmas, vec![2.5, 3.125]);
    }
}
