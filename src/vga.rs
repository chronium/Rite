use core::ptr::Unique;
use spin::Mutex;

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! print {
    ($($arg:tt)*) => ({
            use core::fmt::Write;
            let mut writer = $crate::vga::Console.lock();
            writer.write_fmt(format_args!($($arg)*)).unwrap();
    });
}

/// A static VGA buffer writer.
pub static Console: Mutex<Writer> = Mutex::new(Writer {
    col: 0,
    row: 0,
    color: Color::new(HalfColor::White, HalfColor::Black),
    buffer: unsafe { Unique::new(0xB8000 as *mut _) },
});

/// The buffer width.
const BUFFER_WIDTH: usize = 80;

/// The buffer height.
const BUFFER_HEIGHT: usize = 25;

/// The tab width.
const TAB_WIDTH: usize = 4;

/// The `HalfColor` type.
///
/// Represents a 4-bit color.
#[repr(u8)]
#[allow(dead_code)]
pub enum HalfColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// The `Color` type.
#[derive(Copy, Clone)]
pub struct Color(u8);

/// The `Color` implementation.
impl Color {
    /// Constructs a new `Color`.
    pub const fn new(foreground: HalfColor, background: HalfColor) -> Color {
        Color((background as u8) << 4 | (foreground as u8))
    }
}

/// The `Character` type.
///
/// Represents a character in the VGA buffer.
#[repr(C)]
#[derive(Copy, Clone)]
struct Character {
    /// The ASCII character code.
    char_code: u8,
    /// The color byte.
    color: Color,
}

/// The `Buffer` type.
///
/// Represents the contents of the VGA buffer.
struct Buffer {
    /// The characters.
    chars: [[Character; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// The `Writer` type.
pub struct Writer {
    /// The column.
    col: usize,
    /// The row.
    row: usize,
    /// The color.
    color: Color,
    /// The buffer.
    buffer: Unique<Buffer>,
}

/// The `::core::fmt::Write` implementation for `Writer`.
impl ::core::fmt::Write for Writer {
    #[inline(always)]
    fn write_str(&mut self, string: &str) -> ::core::fmt::Result {
        for byte in string.bytes() {
            self.write_byte(byte)
        }
        Ok(())
    }
}

/// The `Writer` implementation.
impl Writer {
    /// Writes a byte.
    #[inline(always)]
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => self.col = 0,
            b'\t' => {
                for _ in 0..(TAB_WIDTH - (self.col % TAB_WIDTH)) {
                    self.write_byte(b' ');
                }
            }
            0x08 => {
                // Backspace
                let blank = Character {
                    char_code: b' ',
                    color: self.color,
                };
                if self.col == 0 && self.row == 0 {
                    return;
                } else if self.col == 0 {
                    self.buffer().chars[self.row][self.col] = blank;
                    self.row -= 1;
                    self.col = BUFFER_WIDTH - 1;
                } else {
                    self.buffer().chars[self.row][self.col] = blank;
                    self.col -= 1;
                }
            }
            _ => {
                if self.col >= BUFFER_WIDTH {
                    self.new_line();
                }
                self.buffer().chars[self.row][self.col] = Character {
                    char_code: byte,
                    color: self.color,
                };
                self.col += 1;
            }
        }
    }

    /// Writes a string.
    #[inline(always)]
    pub fn write_str(&mut self, string: &str) {
        for byte in string.bytes() {
            self.write_byte(byte)
        }
    }

    /// Clears the screen.
    ///
    /// Also properly fills the screen with the current color.
    #[inline(always)]
    pub fn clear_screen(&mut self) {
        self.col = 0;
        self.row = 0;
        let blank = Character {
            char_code: b' ',
            color: self.color,
        };
        let buf = self.buffer();
        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                buf.chars[row][col] = blank;
            }
        }
    }

    /// Sets the cursor to the specified position.
    #[inline(always)]
    pub fn set_cursor(&mut self, x: usize, y: usize) {
        fn clamp(value: usize, min: usize, max: usize) -> usize {
            return if value < min {
                min
            } else {
                if value > max {
                    max
                } else {
                    value
                }
            };
        }
        self.col = clamp(x, 0, BUFFER_WIDTH);
        self.row = clamp(y, 0, BUFFER_HEIGHT);
    }

    /// Sets the foreground and background color.
    #[inline(always)]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Starts a new line.
    #[inline(always)]
    fn new_line(&mut self) {
        self.col = 0;
        if self.row < BUFFER_HEIGHT - 1 {
            self.row += 1;
        } else {
            self.scroll();
        }
    }

    /// Scrolls up by one line and clears the last line.
    #[inline(always)]
    fn scroll(&mut self) {
        let blank = Character {
            char_code: b' ',
            color: self.color,
        };
        for y in 0..(BUFFER_HEIGHT - 1) {
            for x in 0..BUFFER_WIDTH {
                self.buffer().chars[y][x] = self.buffer().chars[y + 1][x];
            }
        }
        self.buffer().chars[BUFFER_HEIGHT - 1] = [blank; BUFFER_WIDTH];
    }

    /// Gets a mutable reference to the buffer.
    #[inline(always)]
    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.get_mut() }
    }
}
