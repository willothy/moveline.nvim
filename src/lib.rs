use std::ffi::CString;

use nvim_oxi as oxi;
use oxi::api::ToFunction;
use oxi::conversion::FromObject;
use oxi::lua::ffi::{
    lua_State, lua_call, lua_getfield, lua_getglobal, lua_gettop, lua_pushstring, lua_settop,
    lua_type, LUA_TNIL,
};
use oxi::lua::with_state;
use oxi::lua::{cstr, Poppable};
use oxi::Function;
use oxi::{Dictionary, Error};
use oxi::{Object, Result};
use Direction::*;

pub struct StackGuard {
    guard: i32,
    state: *mut lua_State,
}

impl StackGuard {
    pub fn new() -> Self {
        unsafe {
            with_state(|l| Self {
                guard: lua_gettop(l),
                state: l,
            })
        }
    }
}

impl Drop for StackGuard {
    fn drop(&mut self) {
        unsafe { lua_settop(self.state, self.guard) }
    }
}

fn viml_fn<A, R>(name: &str) -> Option<Function<A, R>> {
    let name = CString::new(name).unwrap();
    unsafe {
        with_state(|l| {
            lua_getglobal(l, cstr!("vim"));
            lua_getfield(l, -1, cstr!("fn"));
            lua_getfield(l, -1, name.as_ptr());
            Function::pop(l).ok()
        })
    }
}

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

fn require(module: &str) -> Result<Dictionary> {
    let module = CString::new(module).unwrap();
    unsafe {
        with_state(|l| {
            // aowidoaiw
            lua_getglobal(l, cstr!("require"));
            lua_pushstring(l, module.as_ptr());
            lua_call(l, 1, 1);
            if lua_type(l, -1) == LUA_TNIL {
                Err(Error::Api(oxi::api::Error::Other(format!(
                    "module '{}' not found",
                    module.to_str().unwrap()
                ))))
            } else {
                Ok(Dictionary::pop(l)?)
            }
        })
    }
}

/// Move a line up or down
fn move_line(dir: Direction) -> Result<()> {
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

/// Public function to move line up
fn up(_: ()) -> Result<()> {
    move_line(Up)
}

/// Public function to move line down
fn down(_: ()) -> Result<()> {
    move_line(Down)
}

/// Public function to move a block up
fn block_up(_: ()) -> Result<()> {
    return Err(Error::Api(oxi::api::Error::Other(
        "Not implemented".to_owned(),
    )));
}

/// Public function to move a block down
fn block_down(_: ()) -> Result<()> {
    return Err(Error::Api(oxi::api::Error::Other(
        "Not implemented".to_owned(),
    )));
}

#[oxi::module]
fn moveline() -> Result<Dictionary> {
    Ok(Dictionary::from_iter([
        ("up", Function::from_fn(up).to_object()),
        ("down", Function::from_fn(down).to_object()),
        ("block_up", Function::from_fn(block_up).to_object()),
        ("block_down", Function::from_fn(block_down).to_object()),
    ]))
}
