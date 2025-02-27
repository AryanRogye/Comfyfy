use std::{
    error::Error,
    io::stdout,
    io::Write,
    time::Duration,
};
use std::sync::{Mutex, MutexGuard};
use crossterm::{
    cursor::{
        MoveTo,
        EnableBlinking
    },
    terminal::{
        self,
        Clear, 
        ClearType,
        enable_raw_mode,
        disable_raw_mode
    },
    style::{
        Print,
        SetForegroundColor,
        ResetColor,
        Color
    },
    event::{
        poll,
        read,
        Event,
        KeyCode,
        KeyEventKind,
        KeyModifiers
    },
    ExecutableCommand
};
use tokio::time::{interval, sleep};
use once_cell::sync::Lazy;


/** 

Basic TUI Setup will be middle current song playing
a pause skip and back thats it

This is meant to be ran in a tmux pane or a seperate terminal window

**/

use super::SpotifyClientAuth;

#[derive(PartialEq)]
pub enum TuiState
{
    CommandMode,
    NormalMode
}

#[derive(PartialEq)]
pub enum Control
{
    Pause,
    SkipBack,
    SkipForward
}


pub struct Tui
{
    pub auth : SpotifyClientAuth,
    pub state : TuiState,
    running : bool,
    control : Control
}

static LAST_TRACK : Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static LAST_WIDTH : Lazy<Mutex<u16>> = Lazy::new(|| Mutex::new(8));

impl Tui 
{
    pub fn new(auth: SpotifyClientAuth) -> Self
    {
        Tui
        {
            auth,
            state : TuiState::NormalMode,
            running : false,
            control : Control::Pause
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>>
    {
        stdout().execute(Clear(ClearType::All))?;
        stdout().execute(MoveTo(0,0))?;
        stdout().execute(EnableBlinking)?;
        // hide the cursor

        enable_raw_mode()?;
        self.running = true;

        let mut song_update_interval = interval(Duration::from_secs(5)); // Update every 5 seconds

        // main loop
        loop
        {

            tokio::select!
            {
                _ = self.handle_input() => {

                },
                _ = song_update_interval.tick() => {
                    self.render_current_playing().await?;
                }
            }

            if !self.running
            {
                break;
            }
            
            self.render_skip_back(4, 1, 0).await?;
            self.render_pause(4, 1, 6).await?;
            self.render_skip_forward(4, 1, 15).await?;

            self.render_state()?;
        }

        // clear the terminal
        stdout().execute(Clear(ClearType::All))?;
        // disable raw mode
        disable_raw_mode()?;
        Ok(())
    }

    fn render_box_around_text(&self, start_index : u16, padding : u16, display_str : String, start_x : u16) -> Result<(), Box<dyn Error>>
    {
        stdout().execute(MoveTo((start_x + 1) + padding, start_index))?; // center of the screen
        stdout().execute(Print(&display_str))?;

        // add a | to the left of the pause
        stdout().execute(MoveTo(start_x, start_index))?;
        stdout().execute(Print("│"))?;
        // add a | to the right of the pause
        stdout().execute(MoveTo(start_x + (display_str.len() as u16 + 1) + (padding * 2) , start_index))?;
        stdout().execute(Print("│"))?;
        
        // add a top left corner
        stdout().execute(MoveTo(start_x, start_index - 1))?;
        stdout().execute(Print("┌"))?;
        // add a top right corner
        stdout().execute(MoveTo(start_x + (display_str.len() as u16 + 1) + (padding * 2) , start_index - 1))?;
        stdout().execute(Print("┐"))?;

        // add a bottom left corner
        stdout().execute(MoveTo(start_x, start_index + 1))?;
        stdout().execute(Print("└"))?;
        // add a bottom right corner
        stdout().execute(MoveTo(start_x + (display_str.len() as u16 + 1) + (padding * 2) , start_index + 1))?;
        stdout().execute(Print("┘"))?;

        // add a - to the top of the pause
        stdout().execute(MoveTo(start_x + 1, start_index - 1))?;
        for _ in 1..display_str.len() as u16 + 1 + (padding * 2)
        {
            stdout().execute(Print("─"))?;
        }
        // add a - to the bottom of the pause
        stdout().execute(MoveTo(start_x + 1, start_index + 1))?;
        for _ in 1..display_str.len() as u16 + 1 + (padding * 2)
        {
            stdout().execute(Print("─"))?;
        }

        Ok(())
    }

    pub async fn render_pause(&self, start_index : u16, padding : u16, start_x : u16) -> Result<(), Box<dyn Error>>
    {
        if self.control == Control::Pause
        {
            // set foreground to green
            stdout().execute(SetForegroundColor(Color::Green))?;
        }

        self.render_box_around_text(start_index, padding, "Pause".to_string(), start_x)?;

        // reset Color
        stdout().execute(ResetColor)?;
        stdout().flush()?;

        Ok(())
    }

    pub async fn render_skip_back(&self, start_index : u16, padding : u16, start_x : u16) -> Result<(), Box<dyn Error>>
    {
        if self.control == Control::SkipBack
        {
            // set foreground to green
            stdout().execute(SetForegroundColor(Color::Green))?;
        }
        
        self.render_box_around_text(start_index, padding, "<-".to_string(), start_x)?;

        // reset Color
        stdout().execute(ResetColor)?;
        stdout().flush()?;
        Ok(())
    }

    pub async fn render_skip_forward(&self, start_index : u16, padding : u16, start_x : u16) -> Result<(), Box<dyn Error>>
    {
        if self.control == Control::SkipForward
        {
            // set foreground to green
            stdout().execute(SetForegroundColor(Color::Green))?;
        }

        self.render_box_around_text(start_index, padding, "->".to_string(), start_x)?;

        // Reset Color
        stdout().execute(ResetColor)?;
        stdout().flush()?;
        Ok(())
    }


    pub async fn render_current_playing(&mut self) -> Result<(), Box<dyn Error>>
    {
        // Get the current song
        let current_track = match self.auth.get_current_playing().await? {
            Some(track) => format!("🎵 {} - {}", track.song, track.artists),
            None => "🎵 No song playing".to_string(),
        };

        // Get terminal height for centering
        let (width, _) = terminal::size()?;

        // Lock The Mutex before doing anything with it
        let mut last_track : MutexGuard<Option<String>>= LAST_TRACK.lock().unwrap();
        let mut last_width : MutexGuard<u16> = LAST_WIDTH.lock().unwrap();
        // highest check for a redraw should be if the width is changed
        if *last_width != width
        {
            *last_width = width;
            *last_track = Some(current_track.clone());
        }
        // If the song hasn't changed, don't redraw
        else if *last_track == Some(current_track.clone())
        {
            return Ok(());
        }


        // Calculate available width inside borders (subtract 2 for left/right borders)
        let available_width = width.saturating_sub(2);

        // Truncate the song text if it's too long for the available width,
        // reserving 3 characters for the ellipsis "..."
        let display_track = if current_track.len() > available_width as usize {
            let mut truncated: String = current_track.chars().take(available_width as usize - 3).collect();
            truncated.push_str("...");
            truncated
        } else {
            current_track.clone()
        };

        // Calculate x position to center the text within the available width:
        // x = 1 (left border) + ((available_width - text_width) / 2)
        let x = 1 + available_width.saturating_sub(display_track.len() as u16) / 2;

        // Render the text at the calculated position (y coordinate set to 1)
        stdout().execute(MoveTo(x, 1))?;
        stdout().execute(Clear(ClearType::CurrentLine))?;
        stdout().execute(Print(&display_track))?;


        // wanna print a box around the song
        stdout().execute(MoveTo(1, 0))?; // top left
        // draw a line straight to the right till 1 before the max
        for _ in 1..width - 1
        {
            stdout().execute(Print("─"))?;
        }
        stdout().execute(MoveTo(1, 2))?; // bottom left
        // draw a line straight to the right till 1 before the max
        for _ in 1..width - 1
        {
            stdout().execute(Print("─"))?;
        }

        // draw the corners will be special
        stdout().execute(MoveTo(0, 0))?; // top left
        stdout().execute(Print("┌"))?;
        stdout().execute(MoveTo(width - 1, 0))?; // top right
        stdout().execute(Print("┐"))?;
        stdout().execute(MoveTo(0, 2))?; // bottom left
        stdout().execute(Print("└"))?;
        stdout().execute(MoveTo(width - 1, 2))?; // bottom right
        stdout().execute(Print("┘"))?;

        // add a bar between the corners at the 1 y

        stdout().execute(MoveTo(0, 1))?; // top left
        stdout().execute(Print("│"))?;
        stdout().execute(MoveTo(width - 1, 1))?; // top right
        stdout().execute(Print("│"))?;

        stdout().flush()?;

        Ok(())
    }

    pub fn render_state(&self) -> Result<(), Box<dyn Error>>
    {
        // get the total height of the terminal so we can display at the bottom
        let (_, height) = terminal::size()?;
        match self.state
        {
            TuiState::CommandMode => {
                // render command mode
                stdout().execute(MoveTo(0, height - 1))?;
                stdout().execute(Clear(ClearType::CurrentLine))?;
                stdout().execute(Print("Command Mode"))?;
            }
            TuiState::NormalMode => {
                // render normal mode
                stdout().execute(MoveTo(0, height - 1))?;
                stdout().execute(Clear(ClearType::CurrentLine))?;
                stdout().execute(Print("Normal Mode"))?;
            }
        }

        stdout().flush()?;

        Ok(())
    }


    #[allow(dead_code)]
    pub async fn print_log(&self, log: &str) -> Result<(), Box<dyn Error>>
    {
        let (_, height) = terminal::size().expect("Error getting terminal size");

        stdout().execute(MoveTo(0, height - 3)).expect("MoveTo failed");
        stdout().execute(Clear(ClearType::CurrentLine)).expect("Clear failed");
        stdout().execute(Print(log)).expect("Print failed");
        stdout().flush().expect("Flush failed");

        sleep(Duration::from_secs(5)).await; // Wait 5 seconds

        stdout().execute(MoveTo(0, height - 3)).expect("MoveTo failed");
        stdout().execute(Clear(ClearType::CurrentLine)).expect("Clear failed");
        stdout().flush().expect("Flush failed");
        Ok(())
    }

    pub async fn handle_input(&mut self) -> Result<(), Box<dyn Error>>
    {
        if poll(Duration::from_millis(50))? 
        {
            let event = read()?;
            match event
            {
                Event::Key(event) if event.kind == KeyEventKind::Press => {
                    // handle key press

                    // Control C will toggle the command mode and normal mode
                    if event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char('c')
                    {
                        self.state = match self.state
                        {
                            TuiState::CommandMode => TuiState::NormalMode,
                            TuiState::NormalMode => TuiState::CommandMode
                        };
                    }

                    // if we are in command mode
                    if self.state == TuiState::CommandMode
                    {

                        // Replicating vim : command
                        if event.code == KeyCode::Char(':')
                        {
                            self.handle_colon_command().await?;
                        }
                    }
                    if self.state == TuiState::NormalMode
                    {
                        if self.state == TuiState::NormalMode {
                            match event.code 
                            {
                                KeyCode::Char('p') | KeyCode::Char(' ') => {
                                    self.control = Control::Pause;
                                }
                                KeyCode::Char('b') | KeyCode::Left => {
                                    self.control = Control::SkipBack;
                                }
                                KeyCode::Char('f') | KeyCode::Right => {
                                    self.control = Control::SkipForward;
                                }
                                KeyCode::Enter => {
                                    // wanna print that we pressed enter
                                    // Execute based on current selection
                                    match self.control {
                                        Control::Pause => {
                                            self.auth.pause().await?;
                                        }
                                        Control::SkipBack => {
                                            self.auth.skip_back().await?;
                                        }
                                        Control::SkipForward => {
                                            self.auth.skip_forward().await?;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn handle_colon_command(&mut self) -> Result<(), Box<dyn Error>>
    {
        let mut command = ":".to_string();

        // Render the initial `:` BEFORE entering the loop
        let (_, height) = terminal::size()?;
        stdout().execute(MoveTo(0, height - 1))?;
        stdout().execute(Clear(ClearType::CurrentLine))?;
        stdout().execute(Print(&command))?;
        stdout().flush()?;

        loop
        {
            if poll(Duration::from_millis(50))?
            {
                if let Event::Key(key_event) = read()?
                {
                    // handle all key presses when typing with the : command
                    match key_event.code
                    {
                        KeyCode::Esc => {
                            break;
                        },
                        KeyCode::Enter => {
                            self.handle_command(&command).await?;
                            break;
                        },
                        KeyCode::Backspace => {
                            command.pop();
                        },
                        KeyCode::Char(c) => {
                            command.push(c);
                        }
                        _ => {}
                    }

                    // render the command
                    let (_, height) = terminal::size()?;
                    stdout().execute(MoveTo(0, height - 1))?;
                    stdout().execute(Clear(ClearType::CurrentLine))?;
                    stdout().execute(Print(&command))?;
                    stdout().flush()?;
                }
            }
        }
        Ok(())
    }

    pub async fn handle_command(&mut self, command: &str) -> Result<(), Box<dyn Error>>
    {
        match command
        {
            ":q" => {
                self.state = TuiState::NormalMode;
                self.running = false;
            },
            ":print_token" => {
                // get the height of the terminal
                let (_, height) = terminal::size().expect("Error getting terminal size");
                stdout().execute(MoveTo(0, height - 2)).expect("Error moving cursor");
                stdout().execute(Clear(ClearType::CurrentLine)).expect("Error clearing line");
                stdout().execute(Print(&self.auth.get_token().await?)).expect("Error printing token");
            }
            ":c" => {
                // clear the terminal
                stdout().execute(Clear(ClearType::All)).expect("Error clearing terminal");

                // set the song to none so that it detects a change
                let mut last_track : MutexGuard<Option<String>>= LAST_TRACK.lock().unwrap();
                *last_track = None;
            }
            _ => {}
        }
        Ok(())
    }
}
