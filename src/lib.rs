use nvim_utils::prelude::*;

/// Handle folds
fn calc_fold(lua: &Lua, line: u64, dir: i64) -> LuaResult<i64> {
    if dir > 0 {
        vim::func::foldclosedend(lua, line + dir as u64)
    } else {
        vim::func::foldclosed(lua, line - dir.abs() as u64)
    }
}

/// Perform the actual swapping of lines
fn swap_line(lua: &Lua, source: u64, target: u64, cursor_col: u64) -> LuaResult<()> {
    // Get the line contents
    let source_line = vim::func::getline(lua, source, None)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
        .into_string()?;
    let target_line = vim::func::getline(lua, target, None)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
        .into_string()?;

    // perform the swap
    vim::func::setline(lua, source, &target_line)?;
    vim::func::setline(lua, target, &source_line)?;

    // Move the cursor to the new line
    vim::api::nvim_win_set_cursor(lua, 0, (target, cursor_col))?;
    Ok(())
}

fn move_lines(lua: &Lua, dir: i64) -> LuaResult<()> {
    // Get the selection start and end
    let cursor = vim::api::nvim_win_get_cursor(lua, 0)?;
    let mut selection_start = vim::func::line(lua, "v")? - 1;
    let mut selection_end = cursor.get(1)?;
    let last_line = vim::func::line(lua, "$")?;

    // Select in one direction only
    let swap = if selection_start > selection_end || selection_start == selection_end {
        selection_start = std::mem::replace(&mut selection_end, selection_start) - 1;
        selection_end += 1;
        true
    } else {
        false
    };

    // Silently fail if we're at an edge
    if (selection_start == 0 && dir < 0) || (selection_end == last_line && dir > 0) {
        return Ok(());
    }

    // Allow vim count
    let mut count = {
        let count = vim::v::count(lua)?;
        if count > 0 {
            count
        } else {
            1
        }
    };

    // Calculate target location
    let (target_start, target_end) = if dir > 0 {
        if selection_end + count > last_line {
            count = last_line - selection_end;
        }
        (selection_start + count, selection_end + count)
    } else {
        if count > selection_start {
            count = selection_start;
        }
        (selection_start - count, selection_end - count)
    };

    let mut lines = vim::api::nvim_buf_get_lines(lua, 0, selection_start, selection_end, true)?;
    if dir > 0 {
        let mut replace_lines =
            vim::api::nvim_buf_get_lines(lua, 0, selection_end, target_end, true)?;
        replace_lines.extend(lines.drain(..));
        lines = replace_lines;
        vim::api::nvim_buf_set_lines(lua, 0, selection_start, target_end, true, lines)?;
    } else {
        lines.extend(vim::api::nvim_buf_get_lines(
            lua,
            0,
            target_start,
            selection_start,
            true,
        )?);
        vim::api::nvim_buf_set_lines(lua, 0, target_start, selection_end, true, lines)?;
    };

    let mode: String = vim::func::mode(lua)?;
    vim::api::nvim_feedkeys(
        lua,
        &*format!(
            "{}{}gg{}{}gg=gv",
            &mode,
            if swap { target_end } else { target_start + 1 },
            &mode,
            if swap { target_start + 1 } else { target_end },
        ),
        "n",
        false,
    )?;

    Ok(())
}

/// Move a line up or down
fn move_line(lua: &Lua, dir: i64) -> LuaResult<()> {
    // Get last line of file
    let last_line = vim::func::line(lua, "$")?;

    // Get current cursor position
    let cursor = vim::api::nvim_win_get_cursor(lua, 0)?;
    let line = cursor.get(1)?;
    let col = cursor.get(2)?;

    // Make sue we're not trying to move the last line down or the first line up
    if (line == 1 && dir < 0) || (line == last_line && dir > 0) {
        return Ok(());
    }

    // Account for folds
    let fold = calc_fold(lua, line, dir)?;
    let target = if fold == -1 { line } else { fold as u64 };

    // Allow vim count
    let mut count = {
        let count = vim::v::count(lua)?;
        if count > 0 {
            count
        } else {
            1
        }
    };

    // Add direction to target
    let td = if dir > 0 {
        if line + count > last_line {
            count = last_line - line + 1;
        }
        target + count
    } else {
        if count > line {
            count = line - 1;
        }
        target - count
    };
    // Swap the lines
    swap_line(lua, line, td, col)?;

    // Auto-indent the line
    vim::api::nvim_exec(lua, "silent! normal! v=", false)?;

    Ok(())
}

/// Public function to move line up
fn up(lua: &Lua, _: ()) -> LuaResult<()> {
    move_line(lua, -1)
}

/// Public function to move line down
fn down(lua: &Lua, _: ()) -> LuaResult<()> {
    move_line(lua, 1)
}

/// Public function to move a block up
fn block_up(lua: &Lua, _: ()) -> LuaResult<()> {
    move_lines(lua, -1)
}

/// Public function to move a block down
fn block_down(lua: &Lua, _: ()) -> LuaResult<()> {
    move_lines(lua, 1)
}

#[mlua::lua_module]
fn moveline(lua: &Lua) -> LuaResult<LuaTable> {
    ModuleBuilder::new(lua)
        // move_line_up and move_line_down are deprecated in favor of up, down, block_up and
        // block_down, but they will stay for a while to avoid breaking configs
        .with_fn("move_line_down", down)?
        .with_fn("move_line_up", up)?
        .with_fn("up", up)?
        .with_fn("down", down)?
        .with_fn("block_up", block_up)?
        .with_fn("block_down", block_down)?
        .build()
}
