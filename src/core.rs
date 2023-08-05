use nvim_oxi as oxi;
use oxi::conversion::FromObject;
use oxi::{Error, Function, Object, Result};

use crate::ffi::{require, viml_fn, StackGuard};
use Direction::*;

fn foldclosedend(line: isize) -> Option<usize> {
    let _ = StackGuard::new();

    viml_fn("foldclosedend")?.call(line).ok()
}

fn foldclosed(line: isize) -> Option<usize> {
    let _ = StackGuard::new();

    viml_fn("foldclosed")?.call(line).ok()
}

/// Handle folds
fn calc_fold(line: usize, offset: isize) -> Option<usize> {
    let line = line as isize + offset;
    if offset > 0 {
        foldclosedend(line)
    } else {
        foldclosed(line)
    }
}

#[derive(Debug, PartialEq)]
pub enum Direction {
    Up,
    Down,
}

/// Move a line up or down
pub fn move_line(dir: Direction) -> Result<()> {
    let mut buf = oxi::api::Buffer::current();
    // Get last line of file
    let last_line = buf.line_count()?;

    // Get current cursor position
    let mut win = oxi::api::Window::current();
    let (line, col) = win.get_cursor()?;

    // Make sue we're not trying to move the last line down or the first line up
    if (line == 1 && dir == Up) || (line == last_line && dir == Down) {
        return Ok(());
    }

    // Allow vim count
    let mut count = {
        let count = oxi::api::get_vvar("count")?;
        if count > 0 {
            count
        } else {
            1
        }
    };

    // Add direction to target
    let mut target = if dir == Down {
        if line + count > last_line {
            count = last_line - line + 1;
        }
        line + count
    } else {
        if count > line {
            count = line - 1;
        }
        line - count
    };

    let offset = target as isize - line as isize;

    if let Some(fold) = calc_fold(line, offset) {
        target = fold as usize;
    }

    let mut contents = buf
        .get_lines(line - 1..=line, true)?
        .map(|s| {
            let cow = s.to_string_lossy();
            cow.to_string()
        })
        .collect::<Vec<String>>();

    // Auto-indent the line
    let ts_indent = require("nvim-treesitter.indent")?;
    let get_indent: Function<_, isize> = ts_indent
        .get("get_indent")
        .map(Object::to_owned)
        .map(Function::from_object)
        .map(std::result::Result::ok)
        .flatten()
        .ok_or(Error::Api(oxi::api::Error::Other(
            "nvim-treesitter not installed".to_owned(),
        )))?;

    let initial = get_indent.call(line)?;
    buf.set_lines::<oxi::String, _, _>(line - 1..=line, true, [])?;
    buf.set_lines::<oxi::String, _, _>(
        target - 1..target,
        true,
        contents
            .clone()
            .into_iter()
            .map(|s| oxi::String::from_bytes(s.as_bytes())),
    )?;

    let indent = get_indent.call(target)?;

    if indent == -1 {
        let filetype = buf
            .get_option("filetype")
            .ok()
            .map(String::from_object)
            .map(std::result::Result::ok)
            .flatten()
            .unwrap_or("<unknown>".to_owned());

        oxi::api::err_writeln(&*format!(
            "treesitter parser for {} not installed",
            filetype
        ));
    } else {
        let expandtab = buf
            .get_option("expandtab")
            .ok()
            .map(bool::from_object)
            .map(std::result::Result::ok)
            .flatten()
            .unwrap_or(false);

        let tabstop = buf
            .get_option("tabstop")
            .ok()
            .map(i64::from_object)
            .map(std::result::Result::ok)
            .flatten()
            .unwrap_or(4);

        let indent_str = if expandtab {
            " ".repeat(indent as usize)
        } else {
            "\t".repeat(indent as usize / tabstop as usize)
                + &" ".repeat(indent as usize % tabstop as usize)
        };

        for line_str in &mut contents.iter_mut() {
            let trimmed = line_str.trim_start();
            *line_str = if trimmed.is_empty() {
                String::new()
            } else {
                let mut indent = indent_str.clone();
                indent.push_str(&trimmed);
                indent
            };
        }

        buf.set_lines::<oxi::String, _, _>(
            target - 1..=target,
            true,
            contents
                .into_iter()
                .map(|s| oxi::String::from_bytes(s.as_bytes())),
        )?;

        let indent_diff = indent - initial;
        win.set_cursor(target, (col as isize + indent_diff) as usize)?;
    }

    Ok(())
}
