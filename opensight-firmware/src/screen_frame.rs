pub const PREFIX: &str = "\x1eOSF1 ";

#[derive(Clone, Copy)]
pub struct Frame {
    pub position_ms: u32,
    pub rows: [[char; 21]; 6],
}

pub fn decode(text: &str) -> Result<Frame, &'static str> {
    let body = text.strip_prefix(PREFIX).ok_or("Missing frame marker")?;
    let (clock, text) = body.split_once('\n').ok_or("Missing frame clock")?;
    let position_ms = clock.parse::<u32>().map_err(|_| "Invalid frame clock")?;
    if position_ms > 86_400_000 {
        return Err("Frame clock too large");
    }
    let mut frame = Frame {
        position_ms,
        rows: [[' '; 21]; 6],
    };
    let mut lines = text.split('\n');
    for row in &mut frame.rows {
        let line = lines.next().ok_or("Expected six frame rows")?;
        for (column, character) in line.chars().enumerate() {
            if column >= 21 || character.is_control() {
                return Err("Invalid frame row");
            }
            row[column] = character;
        }
    }
    if lines.next().is_some() {
        return Err("Too many frame rows");
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn russian_empty_rows_and_zero_clock() {
        let frame = decode("\x1eOSF1 0\nПривет, ёж!\n\n\n\n\n").unwrap();
        assert_eq!(frame.position_ms, 0);
        assert_eq!(frame.rows[0][8], 'ё');
        assert_eq!(frame.rows[5], [' '; 21]);
    }
    #[test]
    fn rejects_malformed_frames_instead_of_drawing_them() {
        assert!(decode("ordinary answer").is_err());
        assert!(decode("\x1eOSF1 1\none").is_err());
        assert!(decode("\x1eOSF1 -1\n\n\n\n\n\n").is_err());
        assert!(decode("\x1eOSF1 1\n1234567890123456789012\n\n\n\n\n").is_err());
        assert!(decode("\x1eOSF1 1\n\n\n\n\n\n\n").is_err());
    }
}
