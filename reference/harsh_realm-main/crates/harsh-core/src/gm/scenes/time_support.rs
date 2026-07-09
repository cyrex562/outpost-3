//! Time tracking and formatting helpers.
//!
//! Ported from `src/harsh_realm/gm/scenes/time_support.py`. One tick = one hour.

/// Format game ticks into a `Day N, H AM/PM` string.
pub fn format_time(tick: i64) -> String {
    let days = tick.div_euclid(24) + 1;
    let hour = tick.rem_euclid(24);
    let hour_str = if hour == 0 {
        "Midnight".to_string()
    } else if hour == 12 {
        "Noon".to_string()
    } else if hour < 12 {
        format!("{hour} AM")
    } else {
        format!("{} PM", hour - 12)
    };
    format!("Day {days}, {hour_str}")
}

/// Return a thematic description of the current time of day.
pub fn get_time_description(tick: i64) -> &'static str {
    match tick.rem_euclid(24) {
        0..=4 => "Deep Night",
        5..=7 => "Early Morning",
        8..=11 => "Morning",
        12..=16 => "Afternoon",
        17..=19 => "Evening",
        20..=23 => "Late Night",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_cases() {
        assert_eq!(format_time(0), "Day 1, Midnight");
        assert_eq!(format_time(12), "Day 1, Noon");
        assert_eq!(format_time(9), "Day 1, 9 AM");
        assert_eq!(format_time(15), "Day 1, 3 PM");
        // Day rolls over after 24 ticks.
        assert_eq!(format_time(24), "Day 2, Midnight");
        assert_eq!(format_time(25), "Day 2, 1 AM");
    }

    #[test]
    fn time_descriptions() {
        assert_eq!(get_time_description(2), "Deep Night");
        assert_eq!(get_time_description(6), "Early Morning");
        assert_eq!(get_time_description(10), "Morning");
        assert_eq!(get_time_description(14), "Afternoon");
        assert_eq!(get_time_description(18), "Evening");
        assert_eq!(get_time_description(22), "Late Night");
        // Wraps past 24.
        assert_eq!(get_time_description(26), "Deep Night");
    }
}
