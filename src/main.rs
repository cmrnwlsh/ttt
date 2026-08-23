#![feature(iter_intersperse)]

use std::{
  io::{self, stdout},
  iter,
  ops::{Deref, DerefMut},
};

use rand::{rng, seq::IteratorRandom};
use ratatui::{
  DefaultTerminal, TerminalOptions, Viewport,
  buffer::Buffer,
  crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
  },
  layout::Rect,
  style::{Color, Stylize},
  text::{Line, Span, Text},
  widgets::Widget,
};

fn main() -> io::Result<()> {
  let mut term = Terminal::init()?;
  let mut state = State::init();

  Ok(
    while let Winner::None = {
      term.draw(|frame| frame.render_widget(&state, frame.area()))?;
      &state.won
    } {
      let ev = event::read()?;

      if let Event::Resize(_, _) = ev {
        term.origin = None;
        continue;
      }
      let Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        ..
      }) = ev
      else {
        continue;
      };
      match (code, modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
          break;
        }
        (code, KeyModifiers::NONE)
          if let Ok(cmd) = Input::try_from(code)
            && state.input(cmd) =>
        {
          state.comp_move()
        }
        _ => continue,
      }
    },
  )
}

const CONDS: [[usize; 3]; 8] = [
  [0, 1, 2],
  [3, 4, 5],
  [6, 7, 8],
  [0, 3, 6],
  [1, 4, 7],
  [2, 5, 8],
  [0, 4, 8],
  [6, 4, 2],
];

enum Winner {
  Some(&'static [usize]),
  None,
  Tied,
}

struct State {
  grid: [Mark; 9],
  sel: usize,
  won: Winner,
}

impl State {
  fn init() -> Self {
    Self {
      grid: [Mark::Empty; 9],
      sel: 4,
      won: Winner::None,
    }
  }

  fn input(&mut self, cmd: Input) -> bool {
    let mut s = self.sel as isize;
    let rs = s - s.rem_euclid(3);

    match cmd {
      Input::Up => s -= 3,
      Input::Down => s += 3,
      Input::Left => s = rs + (s - 1).rem_euclid(3),
      Input::Right => s = rs + (s + 1).rem_euclid(3),
      Input::Select => {
        if let Mark::Empty = self.grid[self.sel] {
          self.grid[self.sel] = Mark::Player;
          return !self.check_win();
        }
      }
    };

    self.sel = s.rem_euclid(9) as usize;
    false
  }

  fn comp_move(&mut self) {
    let r = &mut rng();
    let grid = &mut self.grid;
    let mut target = None;

    for want in [Mark::Comp, Mark::Player] {
      for cond in CONDS {
        let group = [grid[cond[0]], grid[cond[1]], grid[cond[2]]];
        if target.is_none()
          && let Some(idx) = group.iter().position(|&m| m == Mark::Empty)
          && group.iter().filter(|&&m| m == want).count() == 2
        {
          target = Some(cond[idx]);
        }
      }
    }

    let mut choose = |choices: &'static [usize; 4]| {
      choices
        .iter()
        .filter(|&&c| grid[c] == Mark::Empty)
        .choose(r)
    };

    if let Some(&choice) = [
      (grid[4] == Mark::Empty).then_some(&4),
      choose(&[0, 2, 6, 8]),
      choose(&[1, 3, 5, 7]),
    ]
    .into_iter()
    .find_map(|c| c)
      && target.is_none()
    {
      target = Some(choice)
    }

    if let Some(idx) = target {
      grid[idx] = Mark::Comp
    }

    self.check_win();
  }

  fn check_win(&mut self) -> bool {
    let grid = self.grid;
    for cond in CONDS.iter() {
      if cond
        .iter()
        .all(|&c| grid[c] != Mark::Empty && grid[c] == grid[cond[0]])
      {
        self.won = Winner::Some(cond);
        return true;
      }
    }
    false
  }

  fn get_cell(&self, row: usize, col: usize) -> Span<'_> {
    let t = row * 3 + col;
    let s = Span::from(self.grid[t]);
    if match self.won {
      Winner::None => self.sel == t,
      Winner::Tied => false,
      Winner::Some(cond) => cond.contains(&t),
    } {
      s.reversed()
    } else {
      s
    }
  }
}

impl Widget for &State {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let rows = (0..3)
      .map(|row| {
        let cells = (0..3)
          .map(|col| self.get_cell(row, col))
          .intersperse("║".into());
        iter::once(Span::from("║"))
          .chain(cells)
          .chain(iter::once(Span::from("║")))
          .collect::<Line>()
      })
      .intersperse("╠═══╬═══╬═══╣".into());

    iter::once(Line::from("╔═══╦═══╦═══╗"))
      .chain(rows)
      .chain(iter::once(Line::from("╚═══╩═══╩═══╝")))
      .collect::<Text>()
      .render(area, buf);
  }
}

struct Terminal {
  origin: Option<(u16, u16)>,
  term: ratatui::DefaultTerminal,
}

impl Terminal {
  fn init() -> io::Result<Self> {
    Ok(Self {
      origin: Some(cursor::position()?),
      term: ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(7),
      }),
    })
  }
}

impl Drop for Terminal {
  fn drop(&mut self) {
    if let Some((col, row)) = self.origin {
      let _ = execute!(stdout(), cursor::MoveTo(col, row + 7));
    }
    ratatui::restore();
    println!()
  }
}

impl Deref for Terminal {
  type Target = DefaultTerminal;

  fn deref(&self) -> &Self::Target {
    &self.term
  }
}

impl DerefMut for Terminal {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.term
  }
}

#[derive(Copy, Clone, PartialEq)]
enum Mark {
  Empty,
  Player,
  Comp,
}

impl From<Mark> for Span<'_> {
  fn from(value: Mark) -> Self {
    let (glyph, color) = match value {
      Mark::Player => (" X ", Color::LightBlue),
      Mark::Comp => (" O ", Color::Red),
      Mark::Empty => (" . ", Color::default()),
    };
    glyph.fg(color)
  }
}

enum Input {
  Up,
  Down,
  Left,
  Right,
  Select,
}

impl TryFrom<KeyCode> for Input {
  type Error = ();

  fn try_from(value: KeyCode) -> Result<Self, Self::Error> {
    match value {
      KeyCode::Char('w') | KeyCode::Up => Ok(Self::Up),
      KeyCode::Char('a') | KeyCode::Left => Ok(Self::Left),
      KeyCode::Char('s') | KeyCode::Down => Ok(Self::Down),
      KeyCode::Char('d') | KeyCode::Right => Ok(Self::Right),
      KeyCode::Char(' ') | KeyCode::Enter => Ok(Self::Select),
      _ => Err(()),
    }
  }
}
