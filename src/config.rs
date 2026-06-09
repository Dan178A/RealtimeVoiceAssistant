use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub gemini_api_key: String,
    pub server_port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        // Try to load .env file, ignore if not found (since it could be set as environment variables)
        let _ = dotenvy::dotenv();

        let gemini_api_key = env::var("GEMINI_API_KEY")
            .expect("GEMINI_API_KEY environment variable is required");

        let server_port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8000);

        Self {
            gemini_api_key,
            server_port,
        }
    }
}
