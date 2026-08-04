const INTERNAL_UNITS_PER_DISPLAY_UNIT: f64 = 1200.0;
const DISPLAY_SYMBOL: &str = "$";

pub fn parse_display_amount(input: &str) -> Result<f64, ()> {
    let mut text = input.trim().to_string();
    if text.is_empty() {
        return Err(());
    }

    for symbol in ["$", "€", "£", "¥", "₩"] {
        text = text.replace(symbol, "");
    }
    text = text.replace(' ', "").replace('_', "");

    let (number_text, multiplier) = match text.chars().last().map(|ch| ch.to_ascii_uppercase()) {
        Some('K') => (&text[..text.len() - 1], 1_000.0),
        Some('M') => (&text[..text.len() - 1], 1_000_000.0),
        Some('B') => (&text[..text.len() - 1], 1_000_000_000.0),
        Some('T') => (&text[..text.len() - 1], 1_000_000_000_000.0),
        _ => (text.as_str(), 1.0),
    };

    let normalized = normalize_separators(number_text)?;
    normalized
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value * multiplier)
        .ok_or(())
}

pub fn parse_display_to_internal(input: &str) -> Result<f64, ()> {
    parse_display_amount(input).map(|value| value * INTERNAL_UNITS_PER_DISPLAY_UNIT)
}

pub fn format_internal_amount(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    match trimmed.parse::<f64>() {
        Ok(value) if value.is_finite() => {
            format_display_amount(value / INTERNAL_UNITS_PER_DISPLAY_UNIT)
        }
        _ => raw.to_string(),
    }
}

pub fn format_display_amount(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let negative = value.is_sign_negative();
    let absolute = value.abs();
    let (scaled, suffix) = if absolute >= 1_000_000_000_000.0 {
        (absolute / 1_000_000_000_000.0, "T")
    } else if absolute >= 1_000_000_000.0 {
        (absolute / 1_000_000_000.0, "B")
    } else if absolute >= 1_000_000.0 {
        (absolute / 1_000_000.0, "M")
    } else if absolute >= 1_000.0 {
        (absolute / 1_000.0, "K")
    } else {
        (absolute, "")
    };

    let mut number = if scaled >= 100.0 || scaled.fract().abs() < 0.000_000_1 {
        format!("{scaled:.0}")
    } else if scaled >= 10.0 {
        trim_decimal_zeros(format!("{scaled:.1}"))
    } else {
        trim_decimal_zeros(format!("{scaled:.2}"))
    };

    if negative && number != "0" {
        number.insert(0, '-');
    }

    format!("{DISPLAY_SYMBOL}{number}{suffix}")
}

pub fn format_internal_for_command(value: f64) -> String {
    trim_decimal_zeros(format!("{value:.12}"))
}

fn normalize_separators(raw: &str) -> Result<String, ()> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(());
    }

    let dot_count = raw.matches('.').count();
    let comma_count = raw.matches(',').count();

    if dot_count > 0 && comma_count > 0 {
        let last_dot = raw.rfind('.').unwrap_or(0);
        let last_comma = raw.rfind(',').unwrap_or(0);
        let decimal_separator = if last_dot > last_comma { '.' } else { ',' };
        let mut normalized = String::with_capacity(raw.len());
        for ch in raw.chars() {
            if ch == decimal_separator {
                normalized.push('.');
            } else if ch != '.' && ch != ',' {
                normalized.push(ch);
            }
        }
        return Ok(normalized);
    }

    if comma_count > 0 {
        return normalize_single_separator(raw, ',');
    }

    if dot_count > 1 {
        return normalize_single_separator(raw, '.');
    }

    Ok(raw.to_string())
}

fn normalize_single_separator(raw: &str, separator: char) -> Result<String, ()> {
    let pieces = raw.split(separator).collect::<Vec<_>>();
    if pieces.iter().any(|piece| piece.is_empty()) {
        return Err(());
    }

    if pieces.len() == 2 {
        let decimals = pieces[1].len();
        if decimals == 3 && pieces[0].len() <= 3 {
            return Ok(pieces.concat());
        }
        return Ok(format!("{}.{}", pieces[0], pieces[1]));
    }

    if pieces.iter().skip(1).all(|piece| piece.len() == 3) {
        Ok(pieces.concat())
    } else {
        Err(())
    }
}

fn trim_decimal_zeros(mut value: String) -> String {
    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tfm2_contract_values_match_game_display() {
        assert_eq!(format_internal_amount("480000000"), "$400K");
        assert_eq!(format_internal_amount("100000"), "$83.3");
    }

    #[test]
    fn compact_input_converts_back_to_internal_units() {
        assert_eq!(parse_display_to_internal("$400K").unwrap(), 480_000_000.0);
        assert_eq!(parse_display_to_internal("83.3333333333").unwrap().round(), 100_000.0);
    }

    #[test]
    fn accepts_grouping_and_decimal_separators() {
        assert_eq!(parse_display_amount("1,200,000").unwrap(), 1_200_000.0);
        assert_eq!(parse_display_amount("1 200 000").unwrap(), 1_200_000.0);
        assert_eq!(parse_display_amount("83,3").unwrap(), 83.3);
    }

    #[test]
    fn billion_display_matches_existing_set_all_value() {
        assert_eq!(format_internal_amount("1200000000000"), "$1B");
    }
}
