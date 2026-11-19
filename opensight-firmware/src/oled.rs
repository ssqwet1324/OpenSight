use crate::text_layout::{COLUMNS, Change, Screen};
#[path = "screen_frame.rs"]
mod screen_frame;
use alloc::string::String;
use display_interface_i2c::I2CInterface;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{
        MonoTextStyle, MonoTextStyleBuilder,
        ascii::{FONT_6X10, FONT_9X18},
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use esp_hal::{Blocking, i2c::master::I2c};
use esp_println::println;
use ssd1306::{I2CDisplayInterface, Ssd1306, mode::BufferedGraphicsMode, prelude::*};

// Latest answer wins. Typing never blocks the WebSocket reader or its pongs.
enum Message {
    Answer(String),
    Frame(screen_frame::Frame),
}
static ANSWERS: Signal<CriticalSectionRawMutex, Message> = Signal::new();
pub const CHARACTER_DELAY_MS: u64 = 45;

pub fn publish(text: &str) {
    if text.starts_with(screen_frame::PREFIX) {
        match screen_frame::decode(text) {
            Ok(frame) => ANSWERS.signal(Message::Frame(frame)),
            Err(error) => println!("OLED frame rejected: {}", error),
        }
    } else {
        ANSWERS.signal(Message::Answer(String::from(text)));
    }
}

pub type Oled = Ssd1306<
    I2CInterface<I2c<'static, Blocking>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

pub fn show_startup(mut bus: I2c<'static, Blocking>) -> Result<Oled, &'static str> {
    let mut found = None;
    for address in [0x3Cu8, 0x3D] {
        // Probe with an actual command, rather than an empty I2C transfer.
        // 0x00 is the command control byte; 0xAE temporarily turns the panel off.
        match bus.write(address, &[0x00, 0xAE]) {
            Ok(()) => {
                println!("OLED found at I2C address 0x{:02X}", address);
                found = Some(address);
                break;
            }
            Err(error) => println!("OLED probe 0x{:02X}: {:?}", address, error),
        }
    }
    let address = found.ok_or("OLED not found. Check GND, 3V3, SDA=GPIO4, SCK=GPIO5.")?;
    let interface = I2CDisplayInterface::new_custom_address(bus, address);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().map_err(|_| "OLED initialization failed")?;
    display.clear_buffer();

    Rectangle::new(Point::zero(), Size::new(128, 64))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut display)
        .map_err(|_| "OLED border draw failed")?;
    Text::with_baseline(
        "OpenSight",
        Point::new(23, 14),
        MonoTextStyle::new(&FONT_9X18, BinaryColor::On),
        Baseline::Top,
    )
    .draw(&mut display)
    .map_err(|_| "OLED title draw failed")?;
    Text::with_baseline(
        "OLED ready",
        Point::new(34, 43),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        Baseline::Top,
    )
    .draw(&mut display)
    .map_err(|_| "OLED status draw failed")?;
    display.flush().map_err(|_| "OLED screen transfer failed")?;
    println!("OLED ready: OpenSight test image sent (128x64).");
    Ok(display)
}

fn readable_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\r' => {}
            '\t' | '\u{a0}' => output.push(' '),
            '—' | '–' | '•' => output.push('-'),
            '«' | '»' | '“' | '”' => output.push('"'),
            '‘' | '’' => output.push('\''),
            '…' => output.push_str("..."),
            '°' => output.push_str(" deg"),
            '×' => output.push('x'),
            '÷' => output.push('/'),
            '\n' | ' '..='~' | 'А'..='я' | 'Ё' | 'ё' | '№' => output.push(c),
            c if c.is_control() => {}
            _ => output.push('?'),
        }
    }
    output
}

fn draw_change(display: &mut Oled, screen: &Screen, change: Change) -> Result<(), &'static str> {
    let style = MonoTextStyle::new(
        &embedded_graphics::mono_font::iso_8859_5::FONT_6X10,
        BinaryColor::On,
    );
    match change {
        Change::None => return Ok(()),
        Change::Glyph {
            column,
            row,
            character,
        } => {
            let mut bytes = [0u8; 4];
            Text::with_baseline(
                character.encode_utf8(&mut bytes),
                Point::new((column * 6) as i32, (row * 10 + 2) as i32),
                style,
                Baseline::Top,
            )
            .draw(display)
            .map_err(|_| "Character draw failed")?;
        }
        Change::Scroll => {
            display.clear_buffer();
            for (row, line) in screen.lines().iter().enumerate() {
                let mut bytes = [0u8; COLUMNS * 4];
                let mut used = 0;
                for &character in line {
                    let count = character.len_utf8();
                    character.encode_utf8(&mut bytes[used..used + count]);
                    used += count;
                }
                let text =
                    core::str::from_utf8(&bytes[..used]).map_err(|_| "Text encoding failed")?;
                Text::with_baseline(
                    text,
                    Point::new(0, (row * 10 + 2) as i32),
                    style,
                    Baseline::Top,
                )
                .draw(display)
                .map_err(|_| "Scroll draw failed")?;
            }
        }
    }
    display.flush().map_err(|_| "OLED transfer failed")
}

#[embassy_executor::task]
pub async fn typewriter(mut display: Oled) {
    println!("OLED ready: AI typewriter and synchronized karaoke frames.");
    let mut message = ANSWERS.wait().await;
    let mut previous_frame: Option<screen_frame::Frame> = None;
    let mut last_logged_ms = 0u32;
    'answers: loop {
        if let Message::Frame(frame) = &message {
            if let Err(error) = draw_frame(&mut display, frame, previous_frame.as_ref()) {
                println!("OLED karaoke: {}", error);
                return;
            }
            if previous_frame.is_none()
                || frame.position_ms.abs_diff(last_logged_ms) >= 2000
                || frame.position_ms < last_logged_ms
            {
                println!("OLED karaoke: rendered at {} ms", frame.position_ms);
                last_logged_ms = frame.position_ms;
            }
            previous_frame = Some(*frame);
            Timer::after(Duration::from_millis(1)).await;
            message = ANSWERS.wait().await;
            continue;
        }
        previous_frame = None;
        let Message::Answer(answer) = &message else {
            unreachable!()
        };
        let answer = readable_text(answer);
        let mut screen = Screen::new();
        display.clear_buffer();
        if display.flush().is_err() {
            println!("OLED: clear failed");
            return;
        }
        println!("OLED: typing {} characters", answer.chars().count());
        let mut characters = answer.chars().peekable();
        let mut word_start = true;

        while let Some(character) = characters.next() {
            if let Some(next) = ANSWERS.try_take() {
                message = next;
                continue 'answers;
            }
            if word_start && !character.is_whitespace() {
                let length = 1 + characters
                    .clone()
                    .take_while(|c| !c.is_whitespace())
                    .count();
                let change = screen.prepare_word(length);
                if change == Change::Scroll {
                    println!("OLED: scroll up");
                }
                if let Err(error) = draw_change(&mut display, &screen, change) {
                    println!("OLED: {}", error);
                    return;
                }
            }
            let change = screen.push(character);
            if change == Change::Scroll {
                println!("OLED: scroll up");
            }
            if let Err(error) = draw_change(&mut display, &screen, change) {
                println!("OLED: {}", error);
                return;
            }
            word_start = character.is_whitespace();
            Timer::after(Duration::from_millis(CHARACTER_DELAY_MS)).await;
        }
        println!("OLED: answer finished");
        message = ANSWERS.wait().await;
    }
}

fn draw_frame(
    display: &mut Oled,
    frame: &screen_frame::Frame,
    previous: Option<&screen_frame::Frame>,
) -> Result<(), &'static str> {
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::iso_8859_5::FONT_6X10)
        .text_color(BinaryColor::On)
        .background_color(BinaryColor::Off)
        .build();
    if previous.is_none() {
        display.clear_buffer();
    }
    for row in 0..6 {
        for column in 0..21 {
            let character = frame.rows[row][column];
            if previous.is_some_and(|old| old.rows[row][column] == character) {
                continue;
            }
            let mut bytes = [0u8; 4];
            Text::with_baseline(
                character.encode_utf8(&mut bytes),
                Point::new((column * 6) as i32, (row * 10 + 2) as i32),
                style,
                Baseline::Top,
            )
            .draw(display)
            .map_err(|_| "Frame draw failed")?;
        }
    }
    display.flush().map_err(|_| "Frame transfer failed")
}
