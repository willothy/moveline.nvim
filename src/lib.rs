use std::ffi::CString;

use nvim_oxi as oxi;
use oxi::api::ToFunction;
use oxi::conversion::FromObject;
use oxi::lua::ffi::{
    lua_call, lua_getfield, lua_getglobal, lua_gettop, lua_pushinteger, lua_pushstring, lua_settop,
    lua_tointeger, lua_type, LUA_TNIL,
};
use oxi::lua::with_state;
use oxi::lua::{cstr, Poppable};
use oxi::Function;
use oxi::{Dictionary, Error};
use oxi::{Object, Result};
use Direction::*;

fn foldclosedend(line: isize) -> isize {
    unsafe {
        with_state(|l| {
            let vim = CString::new("vim").unwrap();
            let viml_funcs = CString::new("fn").unwrap();
            let viml_foldclosedend = CString::new("foldclosedend").unwrap();
            let stack_guard = lua_gettop(l);

            // Get `vim.fn`
            lua_getglobal(l, vim.as_ptr());
            lua_getfield(l, -1, viml_funcs.as_ptr());
            lua_getfield(l, -1, viml_foldclosedend.as_ptr());

            // Argument
            lua_pushinteger(l, line as isize);

            lua_call(l, 1, 1);

            // Pop the number
            let result = lua_tointeger(l, -1);

            // Clean up the stack "frame"
            lua_settop(l, stack_guard);

            result
        })
    }
}

fn foldclosed(line: isize) -> isize {
    unsafe {
        with_state(|l| {
            let vim = CString::new("vim").unwrap();
            let viml_funcs = CString::new("fn").unwrap();
            let viml_foldclosed = CString::new("foldclosed").unwrap();
            let stack_guard = lua_gettop(l);

            // Get `vim.fn`
            lua_getglobal(l, vim.as_ptr());
            lua_getfield(l, -1, viml_funcs.as_ptr());
            lua_getfield(l, -1, viml_foldclosed.as_ptr());

            // Argument
            lua_pushinteger(l, line as isize);

            lua_call(l, 1, 1);

            // Pop the number
            let result = lua_tointeger(l, -1);

            // Clean up the stack "frame"
            lua_settop(l, stack_guard);

            result
        })
    }
}

/// Handle folds
fn calc_fold(line: usize, offset: isize) -> isize {
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
    let fold = calc_fold(line, offset);

    if fold != -1 {
        target = fold as usize;
    }

    let mut contents = buf
        .get_lines(line - 1..=line, true)?
        .map(|s| {
            let cow = s.to_string_lossy();
            cow.to_string()
        })
        .collect::<Vec<String>>();

    buf.set_lines::<oxi::String, _, _>(line - 1..=line, true, [])?;
    buf.set_lines::<oxi::String, _, _>(
        target - 1..target,
        true,
        contents
            .clone()
            .into_iter()
            .map(|s| oxi::String::from_bytes(s.as_bytes())),
    )?;

    win.set_cursor(target, col)?;

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

    let indent = get_indent.call(target)?;
    let initial = get_indent.call(line)?;

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

        let mut indent_str = if expandtab {
            " ".repeat(indent as usize)
        } else {
            "\t".repeat(indent as usize / tabstop as usize)
                + &" ".repeat(indent as usize % tabstop as usize)
        };

        for line in &mut contents {
            let trimmed = line.trim_start();
            *line = if trimmed.is_empty() {
                String::new()
            } else {
                indent_str.push_str(&trimmed);
                let indent = indent_str.clone();
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

        let new_col = if initial != indent {
            col - (initial - indent) as usize
        } else {
            col
        };

        win.set_cursor(target, new_col)?;
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
