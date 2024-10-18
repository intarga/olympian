use crate::Flag;

// TODO: why the hardcoded values?
// TODO: one flag is returned from this, does it apply to all 25 data points?
// TODO: write docs for this. I need someone to explain the logic behind this check to me first
// though
#[allow(missing_docs)]
pub fn monotonic_increase_check(data: &[Option<f32>; 25]) -> Flag {
    if data.iter().any(Option::is_none) {
        return Flag::DataMissing;
    }

    // take a rolling windows of 2 datapoints over the dataset
    // if the later value is less than the earlier for any of them, there was a decrease
    // apply `!` since we want `true` if there is no decrease
    let no_decrease = !data.windows(2).any(|window| window[1] < window[0]);

    let total_diff = data[24].unwrap() - data[0].unwrap();

    if (no_decrease) && (0.7..=100.).contains(&total_diff) {
        // TODO: check that I got these the right way around. this is 7 in kvalobs, and below is
        // 1, but this check's flags aren't documented in the flag document :(. I'm taking 7 and 1
        // to main fail and pass because a bunch of other checks behave like that, but it's not
        // 100% consistent, so we should double-check
        Flag::Fail
    } else {
        Flag::Pass
    }
}

// TODO: implement monotonic_increase_check_cache?
// should it break a datacache into sets of 25 using rolling windows, or chunks?
