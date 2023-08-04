use std::ffi::CString;

use nvim_oxi as oxi;
use oxi::api::ToFunction;
use oxi::lua::ffi::{
    lua_call, lua_getfield, lua_getglobal, lua_gettop, lua_pushinteger, lua_settop, lua_tointeger,
};
use oxi::lua::with_state;
use oxi::Dictionary;
use oxi::Function;
use oxi::Result;
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

// /// Perform the actual swapping of lines
// fn swap_line(lua: &Lua, source: u64, target: u64, cursor_col: u64) -> LuaResult<()> {
//     // Get the line contents
//     let source_line = vim::func::getline(lua, source, None)
//         .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
//         .into_string()?;
//     let target_line = vim::func::getline(lua, target, None)
//         .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
//         .into_string()?;
//
//     // perform the swap
//     vim::func::setline(lua, source, &target_line)?;
//     vim::func::setline(lua, target, &source_line)?;
//
//     // Move the cursor to the new line
//     vim::api::nvim_win_set_cursor(lua, 0, (target, cursor_col))?;
//     Ok(())
// }
//
// fn move_lines(lua: &Lua, dir: i64) -> LuaResult<()> {
//     // Get the selection start and end
//     let cursor = vim::api::nvim_win_get_cursor(lua, 0)?;
//     let mut selection_start = vim::func::line(lua, "v")? - 1;
//     let mut selection_end = cursor.get(1)?;
//     let last_line = vim::func::line(lua, "$")?;
//
//     // Select in one direction only
//     let swap = if selection_start > selection_end || selection_start == selection_end {
//         selection_start = std::mem::replace(&mut selection_end, selection_start) - 1;
//         selection_end += 1;
//         true
//     } else {
//         false
//     };
//
//     // Silently fail if we're at an edge
//     if (selection_start == 0 && dir < 0) || (selection_end == last_line && dir > 0) {
//         return Ok(());
//     }
//
//     // Allow vim count
//     let mut count = {
//         let count = vim::v::count(lua)?;
//         if count > 0 {
//             count
//         } else {
//             1
//         }
//     };
//
//     // Calculate target location
//     let (target_start, target_end) = if dir > 0 {
//         if selection_end + count > last_line {
//             count = last_line - selection_end;
//         }
//         (selection_start + count, selection_end + count)
//     } else {
//         if count > selection_start {
//             count = selection_start;
//         }
//         (selection_start - count, selection_end - count)
//     };
//
//     let mut lines = vim::api::nvim_buf_get_lines(lua, 0, selection_start, selection_end, true)?;
//     if dir > 0 {
//         let mut replace_lines =
//             vim::api::nvim_buf_get_lines(lua, 0, selection_end, target_end, true)?;
//         replace_lines.extend(lines.drain(..));
//         lines = replace_lines;
//         vim::api::nvim_buf_set_lines(lua, 0, selection_start, target_end, true, lines)?;
//     } else {
//         lines.extend(vim::api::nvim_buf_get_lines(
//             lua,
//             0,
//             target_start,
//             selection_start,
//             true,
//         )?);
//         vim::api::nvim_buf_set_lines(lua, 0, target_start, selection_end, true, lines)?;
//     };
//
//     let mode: String = vim::func::mode(lua)?;
//     vim::api::nvim_feedkeys(
//         lua,
//         &*format!(
//             "{}{}gg{}{}gg=gv",
//             &mode,
//             if swap { target_end } else { target_start + 1 },
//             &mode,
//             if swap { target_start + 1 } else { target_end },
//         ),
//         "n",
//         false,
//     )?;
//
//     Ok(())
// }

#[derive(Debug, PartialEq)]
pub enum Direction {
    Up,
    Down,
}

/// Move a line up or down
fn move_line(dir: Direction) -> Result<()> {
    // Get last line of file
    let mut buf = oxi::api::Buffer::current();
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

    let contents = buf.get_lines(line - 1..=line, true)?;

    buf.set_lines::<oxi::String, _, _>(line - 1..=line, true, [])?;
    buf.set_lines::<oxi::String, _, _>(target - 1..target, true, contents)?;

    win.set_cursor(target, col)?;

    // Auto-indent the line
    // TODO: Use treesitter
    oxi::api::exec("silent! normal! v=", false)?;

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
    oxi::api::err_writeln("visual selection movement not yet implemented");
    Ok(())
}

/// Public function to move a block down
fn block_down(_: ()) -> Result<()> {
    oxi::api::err_writeln("visual selection movement not yet implemented");
    Ok(())
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
