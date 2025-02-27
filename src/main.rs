
/** 

    Project Name : Comfyfy
    Description  : Terminal TUI app to listen to music

**/

mod spotify_client_auth;
mod tui;

use dotenv::dotenv;
use spotify_client_auth::SpotifyClientAuth;
use tui::Tui;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    dotenv().ok();

    // start a new instance of SpotifyClientAuth
    let mut auth : SpotifyClientAuth = SpotifyClientAuth::new().await.unwrap();

    if let Err(_) = auth.get_token().await {
        eprintln!("❌ Failed to authenticate");
        return Ok(());
    }


    // want to start the TUI here
    let mut tui : Tui = Tui::new(auth);
    tui.start().await?;

    Ok(())
}
