use crate::db::QuoteRow;

/*
pub fn get_moving_avgs(window: usize, quotes: &[QuoteRow]) -> Vec<(i32, f64)> {
    if quotes.is_empty() || window > quotes.len() {
        return vec![];
    }

    let mut sum: f64 = quotes[0..window].iter().map(|q| q.quote.close).sum();
    let mut avgs = Vec::with_capacity(quotes.len() - window);
    avgs.push((quotes[window - 1].id, sum / (window as f64)));
    let mut drop_idx = 0;
    for quote in quotes[window..].iter() {
        sum -= quotes[drop_idx].quote.close;
        sum += quote.quote.close;
        avgs.push((quote.id, sum / (window as f64)));
        drop_idx += 1;
    }
    avgs
}
*/

pub fn get_exp_moving_avgs(window: usize, quotes: &[QuoteRow]) -> Vec<(i32, f64)> {
    if quotes.is_empty() || window > quotes.len() {
        return vec![];
    }
    let mut avg: f64 = quotes[0].quote.close;
    let mut avgs = Vec::with_capacity(quotes.len() - window);

    // Initialize our EMA with a pseudo-EMA, pretending first entry represents an average, and
    // extending the window until it matches
    let mut init_window = 2f64;
    for quote in quotes[1..window].iter() {
        let init_smoothing = 2.0 / (init_window + 1.0);
        avg = (quote.quote.close - avg) * init_smoothing + avg;
        init_window += 1.0;
    }

    let smoothing: f64 = 2.0 / (window as f64 + 1.0);
    avgs.push((quotes[window - 1].id, avg));
    for quote in quotes[window..].iter() {
        avg = (quote.quote.close - avg) * smoothing + avg;
        avgs.push((quote.id, avg));
    }
    avgs
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::quote::Quote;

    #[test]
    fn test_moving_avgs() {
        let mut id = -1;
        let mocks: Vec<QuoteRow> = [2.0, 3.0, 4.0, 5.5, 6.0, 7.0]
            .map(|f| {
                let mut quote = Quote::default();
                quote.close = f;
                id += 1;
                QuoteRow { id, quote }
            })
            .into_iter()
            .collect();
        let avgs = get_moving_avgs(3, &mocks);
        //  [(2, 3.0), (3, 4.166666666666667), (4, 5.166666666666667), (5, 6.166666666666667)]
        assert_eq!(
            avgs,
            vec![(2, 3.0), (3, 12.5 / 3.0), (4, 15.5 / 3.0), (5, 18.5 / 3.0)]
        );
    }

    #[test]
    fn test_exp_avgs() {
        let mut id = -1;
        let mocks: Vec<QuoteRow> = [2.0, 3.0, 4.0, 5.5, 6.0, 7.0]
            .map(|f| {
                let mut quote = Quote::default();
                quote.close = f;
                id += 1;
                QuoteRow { id, quote }
            })
            .into_iter()
            .collect();
        let avgs = get_exp_moving_avgs(3, &mocks);
        assert_eq!(
            avgs,
            [
                (2, 3.333333333333333),
                (3, 4.416666666666666),
                (4, 5.208333333333333),
                (5, 6.104166666666666)
            ],
        );
    }
}
