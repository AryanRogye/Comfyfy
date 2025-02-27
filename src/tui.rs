


use std::{
    error::Error,
    io::stdout,
    io::Write,
    time::Duration,
    thread
};

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


/** 

Basic TUI Setup will be middle current song playing
a pause skip and back thats it

This is meant to be ran in a tmux pane or a seperate terminal window

**/

use super::SpotifyClientAuth;
use crate::spotify_client_auth::SpotifyCurrentPlaying;

#[derive(PartialEq)]
pub enum TuiState
{
    CommandMode,
    NormalMode
}
enum TuiCommandMode
{
    QUIT,
}
enum TuiNormalMode
{
    Chill
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
            
            self.render_pause().await?;
            self.render_skip_back().await?;
            self.render_skip_forward().await?;

            self.render_state()?;
        }

        disable_raw_mode()?;
        Ok(())
    }

    pub async fn render_pause(&self) -> Result<(), Box<dyn Error>>
    {
        if self.control == Control::Pause
        {
            // set foreground to green
            stdout().execute(SetForegroundColor(Color::Green))?;
        }

        let pause_str ="▌▌".to_string();
        let (width, height) = terminal::size()?;

        let base_width = width / 2 - ((pause_str.len() / 2) as u16);
        let base_height = height / 2;

        stdout().execute(MoveTo(base_width, base_height))?; // center of the screen
        stdout().execute(Print(&pause_str))?;

        // Box Dimensions
        let box_width = pause_str.len() as u16 + 2;
        let box_height = 3;
        
        // Top border
        stdout().execute(MoveTo(base_width - 2, base_height - 1))?;
        stdout().execute(Print("┌"))?;

        for _ in 0..pause_str.len() - 2 {
            stdout().execute(Print("─"))?;
        }
        stdout().execute(Print("┐"))?;

        // Middle (Pause Symbol already drawn)
        stdout().execute(MoveTo(base_width - 2, base_height))?;
        stdout().execute(Print("│"))?;
        stdout().execute(MoveTo(width / 2, base_height))?;
        stdout().execute(Print("│"))?;

        // Bottom border
        stdout().execute(MoveTo(base_width - 2, base_height + 1))?;
        stdout().execute(Print("└"))?;
        for _ in 0..pause_str.len() - 2 {
            stdout().execute(Print("─"))?;
        }
        stdout().execute(Print("┘"))?;

        // reset Color
        stdout().execute(ResetColor)?;
        stdout().flush()?;

        Ok(())
    }

    pub async fn render_skip_back(&self) -> Result<(), Box<dyn Error>>
    {
        if self.control == Control::SkipBack
        {
            // set foreground to green
            stdout().execute(SetForegroundColor(Color::Green))?;
        }
        let skip_back_str = "<-".to_string();
        let (width, height) = terminal::size()?;

        let base_width = width / 4 - ((skip_back_str.len() / 2) as u16);
        let base_height = height / 2;

        stdout().execute(MoveTo(base_width, base_height))?; // center of the screen

        stdout().execute(Print(&skip_back_str))?;

        //// Box Dimensions
        //let box_width = skip_back_str.len() as u16 + 2;
        //let box_height = 3;
        //
        //// Top border
        //stdout().execute(MoveTo(base_width - 2, base_height - 1))?;
        //stdout().execute(Print("┌"))?;
        //
        //for _ in 0..skip_back_str.len() - 2 {
        //    stdout().execute(Print("─"))?;
        //}
        //stdout().execute(Print("┐"))?;
        //
        //// Middle (Skip Back Symbol already drawn)
        //stdout().execute(MoveTo(base_width - 2, base_height))?;
        //stdout().execute(Print("│"))?;
        //stdout().execute(MoveTo(width / 4, base_height))?;
        //stdout().execute(Print("│"))?;
        //
        //// Bottom border
        //stdout().execute(MoveTo(base_width - 2, base_height + 1))?;
        //stdout().execute(Print("└"))?;
        //for _ in 0..skip_back_str.len() - 2 {
        //    stdout().execute(Print("─"))?;
        //}
        //stdout().execute(Print("┘"))?;
        //

        // reset Color
        stdout().execute(ResetColor)?;
        stdout().flush()?;
        Ok(())
    }

    pub async fn render_skip_forward(&self) -> Result<(), Box<dyn Error>>
    {
        if self.control == Control::SkipForward
        {
            // set foreground to green
            stdout().execute(SetForegroundColor(Color::Green))?;
        }
        let skip_forward_str = "->".to_string();
        let (width, height) = terminal::size()?;

        let base_width = 3 * width / 4 - ((skip_forward_str.len() / 2) as u16);
        let base_height = height / 2;

        stdout().execute(MoveTo(base_width, base_height))?; // center of the screen

        stdout().execute(Print(&skip_forward_str))?;

        // Reset Color
        stdout().execute(ResetColor)?;
        stdout().flush()?;
        Ok(())
    }

    pub async fn render_current_playing(&mut self) -> Result<(), Box<dyn Error>>
    {
        static mut LAST_TRACK: Option<String> = None;
        static mut LAST_WIDTH: u16 = 0;

        // Get the current song
        let current_track = match self.auth.get_current_playing().await? {
            Some(track) => format!("🎵 {} - {}", track.song, track.artists),
            None => "🎵 No song playing".to_string(),
        };

        // Get terminal height for centering
        let (width, height) = terminal::size()?;

        unsafe {
            // highest check for a redraw should be if the width is changed
            if LAST_WIDTH != width{
                LAST_TRACK = Some(current_track.clone());
                LAST_WIDTH = width;
            } 
            // If the song hasn't changed, don't redraw
            else if LAST_TRACK.as_ref() == Some(&current_track) 
            {
                return Ok(());
            }
        }
        
        stdout().execute(MoveTo(width / 2 - ((current_track.len() / 2) as u16), 1))?;
        stdout().execute(Clear(ClearType::CurrentLine))?;
        stdout().execute(Print(&current_track))?;


        // wanna print a box around the song
        stdout().execute(MoveTo(1, 0))?; // top left
        // draw a line straight to the right till 1 before the max
        for i in 1..width - 1
        {
            stdout().execute(Print("─"))?;
        }
        stdout().execute(MoveTo(1, 2))?; // bottom left
        // draw a line straight to the right till 1 before the max
        for i in 1..width - 1
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
            }
            _ => {}
        }
        Ok(())
    }
}
