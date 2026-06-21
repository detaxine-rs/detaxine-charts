/// Rounds a raw maximum value up to a "nice" ceiling for axis display,
/// so grid lines land on clean numbers instead of awkward values.
///
/// e.g. `122336.0` → `125000.0`, `1_234_567.0` → `1_300_000.0`
pub fn nice_ceiling(raw_max: f64) -> f64 {
    if raw_max <= 0.0 {
        return 0.0;
    }

    let magnitude = 10f64.powf(raw_max.log10().floor());
    let normalized = raw_max / magnitude; // always in range 1.0..10.0

    let nice_normalized = if normalized <= 1.0 {
        1.0
    } else if normalized <= 1.25 {
        1.25
    } else if normalized <= 1.5 {
        1.5
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 2.5 {
        2.5
    } else if normalized <= 3.0 {
        3.0
    } else if normalized <= 4.0 {
        4.0
    } else if normalized <= 5.0 {
        5.0
    } else if normalized <= 7.5 {
        7.5
    } else {
        10.0
    };

    nice_normalized * magnitude
}

/// Formats a number into a shortened, human-readable string for axis labels.
///
/// e.g. `122500.0` → `"122.5K"`, `1_200_000.0` → `"1.2M"`, `850.0` → `"850"`
pub fn format_short_number(value: f64) -> String {
    let abs = value.abs();
    let sign = if value < 0.0 { "-" } else { "" };

    let (scaled, suffix) = if abs >= 1_000_000_000.0 {
        (abs / 1_000_000_000.0, "B")
    } else if abs >= 1_000_000.0 {
        (abs / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        (abs / 1_000.0, "K")
    } else {
        return format!("{}{}", sign, abs.round() as i64);
    };

    // round to 1 decimal, then drop a trailing ".0" for whole numbers
    let rounded = (scaled * 10.0).round() / 10.0;
    if rounded == rounded.trunc() {
        format!("{}{}{}", sign, rounded as i64, suffix)
    } else {
        format!("{}{:.1}{}", sign, rounded, suffix)
    }
}

/// Formats a float with thousands separators (commas).
///
/// Precision defaults to however many decimal places the value already has.
///
/// e.g. `format_with_commas(122336.0, None)` → `"122,336"`
/// e.g. `format_with_commas(1234.5, None)` → `"1,234.5"`
/// e.g. `format_with_commas(1234.567, Some(2))` → `"1,234.57"`
pub fn format_with_commas(f: f64, precision: Option<usize>) -> String {
    let precision = precision.unwrap_or_else(|| {
        let s = f.to_string();
        s.find('.').map(|i| s.len() - i - 1).unwrap_or(0)
    });
    let formatted = format!("{:.prec$}", f, prec = precision);
    let (integer_part, decimal_part) = match formatted.split_once('.') {
        Some((i, d)) => (i, Some(d)),
        None => (formatted.as_str(), None),
    };
    let (sign, digits) = if integer_part.starts_with('-') {
        ("-", &integer_part[1..])
    } else {
        ("", integer_part)
    };
    let with_commas = digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join(",");
    match decimal_part {
        Some(d) if precision > 0 => format!("{sign}{with_commas}.{d}"),
        _ => format!("{sign}{with_commas}"),
    }
}

/// Formats an integer with thousands separators (commas).
///
/// e.g. `format_int_with_commas(122336)` → `"122,336"`
pub fn format_int_with_commas(n: i64) -> String {
    let (sign, digits) = if n < 0 {
        ("-", format!("{}", n.unsigned_abs()))
    } else {
        ("", format!("{}", n))
    };
    let with_commas = digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join(",");
    format!("{sign}{with_commas}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nice_ceiling() {
        assert_eq!(nice_ceiling(122_336.0), 125_000.0);
        assert_eq!(nice_ceiling(1_234_567.0), 1_500_000.0);
        assert_eq!(nice_ceiling(85.0), 100.0);
        assert_eq!(nice_ceiling(0.0), 0.0);
    }

    #[test]
    fn test_format_short_number() {
        assert_eq!(format_short_number(122_500.0), "122.5K");
        assert_eq!(format_short_number(1_200_000.0), "1.2M");
        assert_eq!(format_short_number(100_000.0), "100K");
        assert_eq!(format_short_number(850.0), "850");
        assert_eq!(format_short_number(-1_500.0), "-1.5K");
    }

    #[test]
    fn test_format_with_commas() {
        assert_eq!(format_with_commas(122_336.0, None), "122,336");
        assert_eq!(format_with_commas(1_234.5, None), "1,234.5");
        assert_eq!(format_with_commas(1_234.567, Some(2)), "1,234.57");
        assert_eq!(format_with_commas(-1_500.0, None), "-1,500");
    }

    #[test]
    fn test_format_int_with_commas() {
        assert_eq!(format_int_with_commas(122_336), "122,336");
        assert_eq!(format_int_with_commas(-1_500), "-1,500");
        assert_eq!(format_int_with_commas(850), "850");
    }
}
