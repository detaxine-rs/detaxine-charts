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
