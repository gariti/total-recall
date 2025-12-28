//! ASCII art collection inspired by Total Recall (1990).

use rand::seq::SliceRandom;

/// ANSI-colored comic book style arts (generated from Gemini images via chafa)
pub const REKALL_COMIC: &str = include_str!("../../assets/rekall_comic.ansi");
pub const KUATO_COMIC: &str = include_str!("../../assets/kuato_comic.ansi");
pub const MARS_COMIC: &str = include_str!("../../assets/mars_comic.ansi");

/// All available colored comic arts
const COMIC_ARTS: &[&str] = &[REKALL_COMIC, KUATO_COMIC, MARS_COMIC];

/// Returns a randomly selected colored comic art piece.
pub fn random_comic_art() -> &'static str {
    let mut rng = rand::thread_rng();
    COMIC_ARTS.choose(&mut rng).copied().unwrap_or(REKALL_COMIC)
}

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

     g Git  b GitHub  t Terminal  e Editor
     Enter Resume  n New  q Quit
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

     g Git  b GitHub  t Terminal  e Editor
     Enter Resume  n New  q Quit
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

     g Git  b GitHub  t Terminal  e Editor
     Enter Resume  n New  q Quit
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

     g Git  b GitHub  t Terminal  e Editor
     Enter Resume  n New  q Quit
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

     g Git  b GitHub  t Terminal  e Editor
     Enter Resume  n New  q Quit
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

     g Git  b GitHub  t Terminal  e Editor
     Enter Resume  n New  q Quit
"#;

/// All available ASCII art pieces.
const ARTS: &[&str] = &[QUAID, MARS_DOME, REACTOR, TWO_WEEKS, KUATO, VENUSVILLE];

/// Returns a randomly selected ASCII art piece.
pub fn random_art() -> &'static str {
    let mut rng = rand::thread_rng();
    ARTS.choose(&mut rng).copied().unwrap_or(QUAID)
}
