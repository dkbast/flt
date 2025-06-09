/*!
This should be the only file in this crate which depends on [crossterm]
functionality beyond the data classes.
*/

use std::time::Instant;
use base64::{encode_config, STANDARD};
use crate::event::PlatformEvent;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::style::{Color, Print};
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{ErrorKind, ExecutableCommand, QueueableCommand};
use flutter_sys::Pixel;
use std::collections::{HashMap, VecDeque};
use std::io::{stdout, Stdout, Write};

use std::ops::Add;
use std::sync::mpsc::Sender;
use std::thread;

use std::os::unix::io::RawFd;

extern crate libc;

/// Lines to reserve the terminal for logging.
const LOGGING_WINDOW_HEIGHT: usize = 4;

pub struct TerminalWindow {
    stdout: Stdout,
    lines: Vec<Vec<TerminalCell>>,
    logs: VecDeque<String>,
    // Coordinates of semantics is represented in the "external" height.
    // See [to_external_height].
    semantics: HashMap<(usize, usize), String>,

    // Switches for debugging.
    simple_output: bool,
    alternate_screen: bool,
    showing_help: bool,
    pub(crate) log_events: bool,
}

impl Drop for TerminalWindow {
    fn drop(&mut self) {
        if !self.simple_output {
            self.stdout.execute(DisableMouseCapture).unwrap();
            disable_raw_mode().unwrap();

            // Show cursor.
            self.stdout.execute(Show).unwrap();

            if self.alternate_screen {
                self.stdout.execute(LeaveAlternateScreen).unwrap();
            }
            // Add a newline char so any other subsequent logs appear on the next line.
            self.stdout.execute(Print("\n")).unwrap();
        }
    }
}

impl TerminalWindow {
    pub(crate) fn new(
        simple_output: bool,
        alternate_screen: bool,
        log_events: bool,
        event_sender: Sender<PlatformEvent>,
    ) -> Self {
        let mut stdout = stdout();

        if !simple_output {
            if alternate_screen {
                // This causes the terminal to be output on an alternate buffer.
                stdout.execute(EnterAlternateScreen).unwrap();
            }

            // Hide cursor.
            stdout.execute(Hide).unwrap();

            enable_raw_mode().unwrap();
            stdout.execute(EnableMouseCapture).unwrap();
        }

        thread::spawn(move || {
            let mut should_run = true;
            while should_run {
                let event = read().unwrap();
                let event = normalize_event_height(event);
                should_run = event_sender
                    .send(PlatformEvent::TerminalEvent(event))
                    .is_ok();
            }
        });

        Self {
            stdout,
            lines: vec![],
            logs: VecDeque::new(),
            semantics: HashMap::new(),
            simple_output,
            showing_help: false,
            alternate_screen,
            log_events,
        }
    }

    // get the size of the kitty terminal window in pixels using ioctl
    pub(crate) fn resolution(&self) -> (usize, usize) {

    let mut winsize = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

  unsafe {
        libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize);
    }

   // substract the height of the logging window and convert to pixels
   let y_resolution = (winsize.ws_row / winsize.ws_ypixel);
   let y_without_logging = (winsize.ws_ypixel as usize) - ((LOGGING_WINDOW_HEIGHT as usize) * y_resolution as usize) as usize;




    /*
     *return (winsize.ws_xpixel as usize, y_without_logging as usize);
     */
   return (400, 400);

    }


    pub(crate) fn size(&self) -> (usize, usize) {
        let (width, height) = terminal::size().unwrap();
        let (width, height) = (width as usize, height as usize);

        // Space for the logging window.
        let height = height - LOGGING_WINDOW_HEIGHT;

        (width, to_external_height(height))
    }

    pub(crate) fn update_semantics(&mut self, label_positions: Vec<((usize, usize), String)>) {
        // TODO(jiahaog): This is slow.
        self.semantics = label_positions.into_iter().collect();
    }



pub(crate) fn draw(
    &mut self,
    pixel_grid: Vec<Vec<Pixel>>,
    (x_offset, y_offset): (isize, isize),
) -> Result<(), ErrorKind> {
    if self.simple_output {
        return Ok(());
    }

    if self.showing_help {
        return Ok(());
    }



    // Convert the pixel grid to RGB data
    let mut rgb_data = Vec::new();
    if pixel_grid.len() <= 0 {
        return Ok(());
    }

let pixel_height = pixel_grid.len();
let pixel_width = pixel_grid[0].len(); // Assuming all rows have the same length

let start_instant = Instant::now();

    for y in 0..pixel_height {
        for x in 0..pixel_width {
            let pixel = &pixel_grid[y as usize][x as usize];
            rgb_data.push(pixel.r);
            rgb_data.push(pixel.g);
            rgb_data.push(pixel.b);
        }
    }

    // Base64 encode the RGB data using the updated base64 crate
    let mut encoded_data = encode_config(&rgb_data, STANDARD);

                self.stdout.queue(MoveTo(0, 0))?;

    let chunk_size = 4096;
    while !encoded_data.is_empty() {
        let is_last_chunk = encoded_data.len() <= chunk_size;
        let chunk: String = if is_last_chunk {
            encoded_data.drain(..).collect()
        } else {
            encoded_data.drain(..chunk_size).collect()
        };

        let m = if is_last_chunk { 0 } else { 1 };
            self.stdout.queue(Print(format!(
            "\x1b_Ga=T,f=24,t=d,s={},v={},x={},y={},m={};{}\x1b\\",
            pixel_width,
            pixel_height,
            x_offset,
            y_offset,
            m,
            chunk
        )))?;
        /*
         *write!(
         *    stdout.queue(Print(format!(
         *        "\x1b_Gf=32,s={},v={},m={};{}\x1b\\",
         *        pixel_width,
         *        pixel_height,
         *        m,
         *        chunk
         *    ))),
         *    "\x1b_Ga=T,f=24,t=d,s={},v={},x={},y={},m={};{}\x1b\\",
         *    pixel_width,
         *    pixel_height,
         *    x_offset,
         *    y_offset,
         *    m,
         *    chunk
         *)?;
         */
    }

/*
 *        {
 *            assert!(self.logs.len() <= LOGGING_WINDOW_HEIGHT);
 *
 *            let (_, terminal_height) = terminal::size()?;
 *
 *            for i in 0..LOGGING_WINDOW_HEIGHT {
 *                let y = terminal_height as usize - LOGGING_WINDOW_HEIGHT + i;
 *
 *                self.stdout.queue(MoveTo(0, y as u16))?;
 *                self.stdout
 *                    .queue(Clear(crossterm::terminal::ClearType::CurrentLine))?;
 *                if let Some(line) = self.logs.get(i) {
 *                    self.stdout.queue(Print(line))?;
 *                }
 *            }
 *
 *            let draw_duration = Instant::now().duration_since(start_instant);
 *
 *            let hint_and_fps = format!("{HELP_HINT} [{}]", draw_duration.as_millis());
 *            self.stdout.queue(MoveTo(
 *                (pixel_width - hint_and_fps.len()) as u16,
 *                (terminal_height - 1) as u16,
 *            ))?;
 *            self.stdout.queue(Print(hint_and_fps))?;
 *        }
 */


    self.stdout.flush()?;

    return Ok(());
}



    pub(crate) fn log(&mut self, message: String) {
        if self.simple_output {
            println!("{message}");
        }
        if self.logs.len() == LOGGING_WINDOW_HEIGHT {
            self.logs.pop_front();
        }
        self.logs.push_back(message);
    }

    pub(crate) fn toggle_show_help(&mut self) -> Result<(), ErrorKind> {
        self.showing_help = !self.showing_help;

        self.stdout.execute(Clear(ClearType::All))?;
        self.mark_dirty();

        if self.showing_help {
            self.stdout.queue(MoveTo(0, 0))?;
            self.stdout.queue(Print("Ctrl + r: Reset the viewport."))?;
            self.stdout.queue(MoveTo(0, 2))?;
            self.stdout
                .queue(Print("Ctrl + 5: Increase the pixel ratio."))?;
            self.stdout.queue(MoveTo(0, 4))?;
            self.stdout
                .queue(Print("Ctrl + 4: Decrease the pixel ratio."))?;
            self.stdout.queue(MoveTo(0, 6))?;
            self.stdout
                .queue(Print("Ctrl + Mouse Scroll: Zoom in / out."))?;
            self.stdout.queue(MoveTo(0, 8))?;
            self.stdout
                .queue(Print("Ctrl + Mouse Click and Drag: Pan the viewport. Some terminals might not allow this."))?;
            self.stdout.queue(MoveTo(0, 10))?;
            self.stdout.queue(Print(
                "Ctrl + z: Show semantic labels (very experimental and jank).",
            ))?;
            self.stdout.queue(MoveTo(0, 12))?;
            self.stdout.queue(Print("?: Toggle help."))?;

            self.stdout.queue(MoveTo(0, 14))?;
            self.stdout.queue(Print("Tips: Changing the current terminal emulator's text size will make things look a lot better. "))?;
            self.stdout.queue(MoveTo(0, 15))?;
            self.stdout.queue(Print(
                "But the code is suboptimal and it might lead to more jank.",
            ))?;
            self.stdout.flush()?;
        }
        Ok(())
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.lines.clear();
    }

}

#[derive(PartialEq, Eq, Clone)]
struct TerminalCell {
    top: Color,
    bottom: Color,
    semantics: Option<String>,
}


/// Translates from a "Internal" height to a "External" height.
///
/// "External" height is the height seen by users of this class.
/// "Internal" height is the height actually used when reading / writing to the
/// terminal.
///
/// Translation is needed as the terminal drawing strategy merges two lines of
/// pixels (seen to external users) into one line when written to the terminal.
fn to_external_height<T: Add<Output = T> + Copy>(internal_height: T) -> T {
    internal_height + internal_height
}

fn normalize_event_height(event: Event) -> Event {
    match event {
        Event::Resize(columns, rows) => {
            let rows = rows - LOGGING_WINDOW_HEIGHT as u16;
            Event::Resize(columns, to_external_height(rows))
        }
        Event::Mouse(mut mouse_event) => {
            mouse_event.row = to_external_height(mouse_event.row);
            Event::Mouse(mouse_event)
        }
        x => x,
    }
}

fn to_color(Pixel { r, g, b, a: _ }: &Pixel) -> Color {
    Color::Rgb {
        r: *r,
        g: *g,
        b: *b,
    }
}

const HELP_HINT: &str = "? for help";
