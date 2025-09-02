pub mod game_versions;

use std::str::FromStr;

use game_versions::{FFXIV_3_30_VER, FFXIV_7_30H_VER};

#[repr(C)]
#[derive(PartialEq, Eq, Debug)]
pub enum Game {
  Ffxiv,
  Unknown
}

#[repr(C)]
#[derive(PartialEq, Eq, Debug)]
pub enum GameVersion {
  Ffxiv7_30h,
  Ffxiv3_30,
  Unknown
}

#[derive(Default)]
pub struct FfxivGameVersion {
  pub year: u32,
  pub month: u32,
  pub day: u32,
  pub major: u32,
  pub minor: u32
}

impl FromStr for FfxivGameVersion {
  type Err = String;
  fn from_str(version: &str) -> Result<Self, Self::Err> {
    if version.eq_ignore_ascii_case("") {
      return Ok(FfxivGameVersion::default());
    }

    let parts: Vec<&str> = version.split('.').collect();
    let mut int_parts = Vec::with_capacity(parts.len());

    for p in parts {
      match p.parse::<u32>() {
        Ok(v) => int_parts.push(v),
        Err(_) => return Err("Bad formatting in version string".to_string())
      }
    }

    match int_parts.as_slice() {
      [a] => Ok(FfxivGameVersion {
        year: *a,
        ..Default::default()
      }),
      [a, b] => Ok(FfxivGameVersion {
        year: *a,
        month: *b,
        ..Default::default()
      }),
      [a, b, c] => Ok(FfxivGameVersion {
        year: *a,
        month: *b,
        day: *c,
        ..Default::default()
      }),
      [a, b, c, d] => Ok(FfxivGameVersion {
        year: *a,
        month: *b,
        day: *c,
        major: *d,
        ..Default::default()
      }),
      [a, b, c, d, e] => Ok(FfxivGameVersion {
        year: *a,
        month: *b,
        day: *c,
        major: *d,
        minor: *e
      }),
      _ => Err("Too many parts in version string".to_string())
    }
  }
}

pub fn get_game(process_name: &str) -> Game {
  match process_name {
    "ffxiv_dx11.exe" => Game::Ffxiv,
    _ => Game::Unknown
  }
}

pub fn get_game_version(game: &Game, version: &str) -> GameVersion {
  match (game, version) {
    (Game::Ffxiv, ver) if ver == FFXIV_7_30H_VER => GameVersion::Ffxiv7_30h,
    (Game::Ffxiv, ver) if ver == FFXIV_3_30_VER => GameVersion::Ffxiv3_30,
    (_, _) => GameVersion::Unknown
  }
}
