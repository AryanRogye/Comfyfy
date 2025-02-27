

use serde_json::Value;

use std::{
    fs,
    path::Path,
    collections::HashMap,
    time::{
        Duration, 
        Instant
    },
    io::{Read, Write}
};
use reqwest::{Client, Response};
use webbrowser;


pub struct SpotifyCurrentPlaying
{
    pub artists : String,
    pub album : String,
    pub song : String,
}

/** 
        This is the struct that will hold the token and the expiry time
**/
pub struct SpotifyClientAuth
{
    access_token : String,
    refresh_token : String,
    expires_at : Instant,
}

impl SpotifyClientAuth 
{
    pub async fn get_token(&mut self) -> Result<String, Box<dyn std::error::Error>>
    {
        // check if the token has expired
        if Instant::now() >= self.expires_at {
            // refresh it 
            let (access_token, expires_in , refresh_token) = SpotifyClientAuth::refresh_new_tokens(self.refresh_token.clone()).await?;
            // update the struct

            self.access_token = access_token;
            self.expires_at = Instant::now() + Duration::from_secs(expires_in);
            self.refresh_token = refresh_token;
        }
        // and return it
        Ok(self.access_token.clone())
    }

    pub async fn add_debug_log(&mut self, log : String) -> Result<(), Box<dyn std::error::Error>>
    {
        // this is just a simple function to add a debug log to a file called debug.log
        // if it doesnt exist it will create it
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.log")?;
        if let Err(e) = writeln!(file, "{}", log) {
            eprintln!("Couldn't write to file: {}", e);
        }
        Ok(())
    }

    /** 

    Function is called at the start to get a access_token and a refresh_token
    refresh_token should be used to refresh

    **/
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> 
    {
        // Check to make sure that the token json file exists
        let token_path = Path::new("token.json");
        if token_path.exists()
        {
            // this means that the refresh token does exist
            let token_data = fs::read_to_string(token_path)?;
            let saved_refresh_token = token_data.trim().to_string();
            // make sure that the refresh token is not empty
            if !saved_refresh_token.is_empty() {

                // get the token and the expiry time
                let (access_token, expires_in, refresh_token) = SpotifyClientAuth::refresh_new_tokens(saved_refresh_token.clone()).await?;
                // return the struct
                return Ok(
                    Self {
                        access_token,
                        refresh_token,
                        expires_at : Instant::now() + Duration::from_secs(expires_in)
                    }
                )
            }
        }

        // get the token and the expiry time
        let user_token : String = SpotifyClientAuth::open_auth_login().await?;
        let (access_token, expires_in, refresh_token) = SpotifyClientAuth::get_api_key(user_token.clone()).await?;
    
        // save the refresh token to a file
        fs::write(token_path, refresh_token.clone())?;

        Ok(
            Self {
                access_token,
                refresh_token,
                expires_at : Instant::now() + Duration::from_secs(expires_in)
            }
        )
    }

    pub async fn refresh_new_tokens(refresh_token : String) -> Result<(String, u64, String), Box<dyn std::error::Error>>
    {
        // this is the same as refresh but static so that it can be called from anywhere
        let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID not found in .env file");
        let client_secret = std::env::var("CLIENT_SECRET").expect("CLIENT_SECRET not found in .env file");

        let request = "https://accounts.spotify.com/api/token";

        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", &refresh_token);
        params.insert("client_id", &client_id);
        params.insert("client_secret", &client_secret);

        let client = Client::new();
        let response : Value = client
            .post(request)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let access_token = response["access_token"].as_str().unwrap().to_string();
        let expires_in = response["expires_in"].as_u64().unwrap();
        
        let mut refresh_token : String = "".to_string();
        if response["refresh_token"].is_string() {
            refresh_token = response["refresh_token"].as_str().unwrap().to_string();
        }

        Ok((access_token, expires_in, refresh_token))
    }

    /** 
        The Reason I have to do this is cuz spotify refreshes the token every 1 hour
    **/
    async fn get_api_key(user_token : String) -> Result<(String, u64, String), Box<dyn std::error::Error>> 
    {
        // get the client id and the client secret from the .env file
        let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID not found in .env file");
        let client_secret = std::env::var("CLIENT_SECRET").expect("CLIENT_SECRET not found in .env file");
    
        // wanna post to this url
        let request = "https://accounts.spotify.com/api/token";
        
        let mut params = HashMap::new();
    
        params.insert("grant_type", "authorization_code");
        params.insert("code", &user_token);
        params.insert("redirect_uri", "http://localhost:8888/");
        params.insert("client_id", &client_id);
        params.insert("client_secret", &client_secret);
        params.insert("scope", "user-read-currently-playing user-read-playback-state user-modify-playback-state");
    
        let client = Client::new();
        let response : Value = client
            .post(request)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        for (key, value) in response.as_object().unwrap() {
            println!("{}: {}", key, value);
        }
    
        let token = response["access_token"].as_str().unwrap().to_string();
        let expires_in = response["expires_in"].as_u64().unwrap();
        let refresh_token = response["refresh_token"].as_str().unwrap().to_string();
    
        Ok((token, expires_in, refresh_token))
    }

    /** 
        This will open up the browser and ask the user to login to spotify
    **/
    async fn open_auth_login() -> Result<String, Box<dyn std::error::Error>>
    {
        let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID not found in .env file");
        let scopes = "user-read-currently-playing user-read-playback-state user-modify-playback-state";
        let redirect_uri = format!(
            "https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri=http://localhost:8888/&scope={}",
            client_id, scopes
        );

        match webbrowser::open(&redirect_uri) {
            Ok(_) => println!("🔓 Opened Spotify login page in browser"),
            Err(_) => println!("❌ Failed to open Spotify login page in browser"),
        }

        // starting a webserver to get the code
        let listener = std::net::TcpListener::bind("127.0.0.1:8888")?;
        for stream in listener.incoming() {
            let mut stream = stream?;
            let mut buffer = [0; 1024];
            stream.read(&mut buffer)?;

            // Convert bytes to string
            let request = String::from_utf8_lossy(&buffer[..]);

            // Extract the "code" from the URL
            if let Some(code_start) = request.find("code=") {
                let code_end = request[code_start..].find(' ').unwrap_or(request.len());
                let code = &request[code_start + 5..code_start + code_end];

                // Send a response to the browser
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n
                    <h1>Spotify Login Successful! You can close this tab.</h1>";
                stream.write_all(response.as_bytes())?;
                stream.flush()?;

                return Ok(code.to_string());
            }
        }

        Ok("Failed to get code".to_string())
    }

    pub async fn pause(&mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        let mut request = "https://api.spotify.com/v1/me/player";
        let mut client = Client::new();

        let response = client
            .get(request)
            .header("Authorization", format!("Bearer {}", self.get_token().await?))
            .send()
            .await?;

        // now inside here we want to get the is playing value
        let response_json: Value = response.json().await?;
        let is_playing = response_json.get("is_playing").unwrap().as_bool().unwrap();

        if is_playing
        {
            request = "https://api.spotify.com/v1/me/player/pause";
            client = Client::new();

            let response = client
                .put(request)
                .header("Authorization", format!("Bearer {}", self.get_token().await?))
                .header("Content-Length", "0")
                .send()
                .await?;
            if response.status() == reqwest::StatusCode::NO_CONTENT {
                self.add_debug_log("⏸ Paused".to_string()).await?;
            }
        } else {
            // then we do the play version
            request = "https://api.spotify.com/v1/me/player/play";
            client = Client::new();

            let response = client
                .put(request)
                .header("Authorization" , format!("Bearer {}", self.get_token().await?))
                .header("Content-Length", "0")
                .send()
                .await?;

            if response.status() == reqwest::StatusCode::NO_CONTENT {
                self.add_debug_log("▶️ Resumed".to_string()).await?;
            }
        }
        Ok(())
    }

    /**
        Helper Function to send playback info for going back and forward
        This is a post request
    **/
    async fn send_play_back_info(&mut self,  request : &str) -> Result<Response, Box<dyn std::error::Error>>
    {
        let client = Client::new();

        let response : Response = client
            .post(request)
            .header("Authorization", format!("Bearer {}", self.get_token().await?))
            .header("Content-Length", "0")
            .send()
            .await?;

        Ok(response)
    }
    pub async fn skip_back(&mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        let request = "https://api.spotify.com/v1/me/player/previous";
        let response : Response = self.send_play_back_info(request).await?;

        if response.status() == reqwest::StatusCode::NO_CONTENT {
            self.add_debug_log("⏮ Skipped back".to_string()).await?;
        }

        Ok(())
    }
    pub async fn skip_forward(&mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        let request = "https://api.spotify.com/v1/me/player/next";
        let response : Response = self.send_play_back_info(request).await?;

        if response.status() == reqwest::StatusCode::NO_CONTENT {
            self.add_debug_log("⏭ Skipped forward".to_string()).await?;
        }

        Ok(())
    }
    pub async fn get_current_playing(&mut self) -> Result<Option<SpotifyCurrentPlaying>, Box<dyn std::error::Error>>
    {

        let request = "https://api.spotify.com/v1/me/player/currently-playing";
        let client = Client::new();

        let response = client
            .get(request)
            .header("Authorization", format!("Bearer {}", self.get_token().await?))
            .send()
        .await?;

        let token = self.get_token().await?;
        self.add_debug_log(format!("Token: {}", token)).await?;


        // Check if the response is empty (no currently playing track)
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None); // No song is playing
        }

        let response_json: Value = response.json().await?;

        // Handle case where no song is playing
        if response_json.get("item").is_none() {
            return Ok(None); // Nothing is playing
        }

        let item = response_json.get("item").unwrap();

        // Safely extract song name
        let song = item.get("name").and_then(|s| s.as_str()).unwrap_or("Unknown Song");

        // Extract album name
        let album = item.get("album")
            .and_then(|a| a.get("name"))
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown Album");

        // Extract first artist's name safely
        let artist = item.get("artists")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|a| a.get("name"))
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown Artist");

        Ok(Some(SpotifyCurrentPlaying {
            artists: artist.to_string(),
            album: album.to_string(),
            song: song.to_string(),
        }))
    }
}
