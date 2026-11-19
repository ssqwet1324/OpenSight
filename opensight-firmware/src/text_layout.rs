pub const COLUMNS: usize = 21;
pub const ROWS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    None,
    Glyph {
        column: usize,
        row: usize,
        character: char,
    },
    Scroll,
}

pub struct Screen {
    cells: [[char; COLUMNS]; ROWS],
    column: usize,
    row: usize,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            cells: [[' '; COLUMNS]; ROWS],
            column: 0,
            row: 0,
        }
    }

    pub fn lines(&self) -> &[[char; COLUMNS]; ROWS] {
        &self.cells
    }

    fn newline(&mut self) -> bool {
        self.column = 0;
        if self.row + 1 < ROWS {
            self.row += 1;
            false
        } else {
            self.cells.copy_within(1..ROWS, 0);
            self.cells[ROWS - 1] = [' '; COLUMNS];
            true
        }
    }

    // Keep words together when they fit on an otherwise empty line.
    pub fn prepare_word(&mut self, length: usize) -> Change {
        if self.column > 0 && length <= COLUMNS && self.column + length > COLUMNS {
            if self.newline() {
                Change::Scroll
            } else {
                Change::None
            }
        } else {
            Change::None
        }
    }

    pub fn push(&mut self, character: char) -> Change {
        if character == '\n' {
            return if self.newline() {
                Change::Scroll
            } else {
                Change::None
            };
        }
        if character == ' ' && (self.column == 0 || self.column == COLUMNS) {
            return Change::None;
        }
        let scrolled = self.column == COLUMNS && self.newline();
        self.cells[self.row][self.column] = character;
        let change = Change::Glyph {
            column: self.column,
            row: self.row,
            character,
        };
        self.column += 1;
        if scrolled { Change::Scroll } else { change }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_is_counted_as_characters_not_utf8_bytes() {
        let mut screen = Screen::new();
        for c in "Привет, ёж!".chars() {
            screen.push(c);
        }
        assert_eq!(screen.column, 11);
        assert_eq!(screen.cells[0][8], 'ё');
    }

    #[test]
    fn full_last_line_remains_until_another_character_arrives() {
        let mut screen = Screen::new();
        for row in 0..ROWS {
            for _ in 0..COLUMNS {
                screen.push(char::from(b'0' + row as u8));
            }
        }
        assert_eq!(screen.cells[0][0], '0');
        assert_eq!(screen.cells[5][20], '5');
        assert_eq!(screen.push('Я'), Change::Scroll);
        assert_eq!(screen.cells[0][0], '1');
        assert_eq!(screen.cells[5][0], 'Я');
    }

    #[test]
    fn word_wrap_does_not_split_a_short_word() {
        let mut screen = Screen::new();
        for _ in 0..18 {
            screen.push('а');
        }
        screen.push(' ');
        screen.prepare_word(5);
        for c in "слово".chars() {
            screen.push(c);
        }
        assert_eq!(screen.row, 1);
        assert_eq!(screen.column, 5);
        assert_eq!(screen.cells[1][0], 'с');
    }

    #[test]
    fn newline_after_exact_width_does_not_skip_a_line() {
        let mut screen = Screen::new();
        for _ in 0..COLUMNS {
            screen.push('А');
        }
        screen.push('\n');
        screen.push('Б');
        assert_eq!(screen.row, 1);
        assert_eq!(screen.cells[1][0], 'Б');
    }

    #[test]
    fn very_long_words_wrap_without_going_out_of_bounds() {
        let mut screen = Screen::new();
        screen.prepare_word(1000);
        for _ in 0..1000 {
            screen.push('Я');
        }
        assert!(screen.row < ROWS);
        assert!(screen.column <= COLUMNS);
    }
}
