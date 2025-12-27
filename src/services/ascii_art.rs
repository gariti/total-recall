//! ASCII art collection inspired by Total Recall (1990).

use rand::seq::SliceRandom;

/// Quaid/Arnold portrait - stylized action hero face
const QUAID: &str = r#"
        ▄▄▄████████▄▄▄
      ▄██▀▀        ▀▀██▄
     ██▀   ▄▄    ▄▄   ▀██
    ██    █▀▀█  █▀▀█    ██
    ██     ▀▀    ▀▀     ██
    ██▌    ▄▀▀▀▀▄     ▐██
     ██    ▀▄▄▄▄▀     ██
      ██▄   ▀▀▀▀   ▄██
       ▀███▄▄▄▄▄▄███▀
          ▀▀▀▀▀▀▀

     "Get your ass to Claude."

     ← →  Navigate
     Enter  Resume session
     q  Quit
"#;

/// Mars dome skyline with red planet atmosphere
const MARS_DOME: &str = r#"
    ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄
  ▄█░░░░░░░░░░░░░░░░░░░░░░░░░█▄
 ██░░▄▄▄▄▄░░░░░░░░░░▄▄▄▄▄░░░░██
 ██░█▀   ▀█░░░░░░░░█▀   ▀█░░░██
 ██░█     █░░▄▄▄▄░░█     █░░░██
 ██░█▄   ▄█░█    █░█▄   ▄█░░░██
 ██░░▀▀▀▀▀░░█    █░░▀▀▀▀▀░░░░██
▄██▄▄▄▄▄▄▄▄▄█▄▄▄▄█▄▄▄▄▄▄▄▄▄▄▄██▄
████████████████████████████████

     "Get your ass to Claude."

     ← →  Navigate
     Enter  Resume session
     q  Quit
"#;

/// The alien reactor core on Mars
const REACTOR: &str = r#"
          ▄▄████▄▄
        ▄██▀░░░░▀██▄
       ██░░▄██▄░░░░██
      ██░░██████░░░░██
     ██░░░██████░░░░░██
     ██░░░░▀██▀░░░░░░██
     ██░░░░░░░░░░░░░░██
      ██░░▀██████▀░░██
       ▀██░░░░░░░░██▀
         ▀▀████▀▀
    START THE REACTOR...

     "Get your ass to Claude."

     ← →  Navigate
     Enter  Resume session
     q  Quit
"#;

/// "Two weeks" disguise lady
const TWO_WEEKS: &str = r#"
         ▄▄▄▄▄▄▄▄▄
       ▄█▀▀     ▀▀█▄
      ██   ▄▄ ▄▄   ██
      ██  █░░█░░█  ██
      ██   ▀▀ ▀▀   ██
      ██    ___    ██
       ██  |   |  ██
        ██ |▓▓▓| ██
         █▄▄▄▄▄▄▄█
    T̷̢W̵O̶ ̵W̷E̶E̴K̷S̵.̶.̵.̷

     "Get your ass to Claude."

     ← →  Navigate
     Enter  Resume session
     q  Quit
"#;

/// Kuato - "Open your mind..."
const KUATO: &str = r#"
       ▄▄▄███████▄▄▄
     ▄██▀▀       ▀▀██▄
    ██   ▄▄▄▄▄▄▄▄▄   ██
   ██  ▄█░░░░░░░░█▄  ██
   ██  █ ▀▄   ▄▀ █  ██
   ██  █   ▄▄▄   █  ██
   ██  ▀█ ▀███▀ █▀  ██
    ██   ▀▀▀▀▀▀▀   ██
     ▀██▄▄▄▄▄▄▄▄▄██▀
   "OPEN YOUR MIND..."

     "Get your ass to Claude."

     ← →  Navigate
     Enter  Resume session
     q  Quit
"#;

/// Venusville - the mutant bar
const VENUSVILLE: &str = r#"
   ╔═══════════════════════╗
   ║  V E N U S V I L L E  ║
   ╠═══════════════════════╣
   ║ ░▒▓█ LAST RESORT █▓▒░ ║
   ║                       ║
   ║  ▄█▄  ▄█▄  ▄█▄  ▄█▄   ║
   ║  ███  ███  ███  ███   ║
   ║  ▀█▀  ▀█▀  ▀█▀  ▀█▀   ║
   ╚═══════════════════════╝
     THE LAST RESORT

     "Get your ass to Claude."

     ← →  Navigate
     Enter  Resume session
     q  Quit
"#;

/// All available ASCII art pieces.
const ARTS: &[&str] = &[QUAID, MARS_DOME, REACTOR, TWO_WEEKS, KUATO, VENUSVILLE];

/// Returns a randomly selected ASCII art piece.
pub fn random_art() -> &'static str {
    let mut rng = rand::thread_rng();
    ARTS.choose(&mut rng).copied().unwrap_or(QUAID)
}
