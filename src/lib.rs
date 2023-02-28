use nvim_utils::prelude::*;

/// Handle folds
fn calc_fold(lua: &Lua, line: u64, dir: i64) -> LuaResult<i64> {
    if dir > 0 {
        vim::func::foldclosedend(lua, line + dir as u64)
    } else {
        vim::func::foldclosed(lua, line - dir.abs() as u64)
    }
}

/// Add direction to line number
fn add_dir(num: u64, mut dir: i64, count: Option<u64>) -> u64 {
    if let Some(count) = count {
        dir *= count as i64;
    }
    if dir >= 0 {
        num + dir as u64
    } else {
        num - dir.abs() as u64
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
    let count = {
        let count = vim::v::count(lua)?;
        if count > 0 {
            Some(count)
        } else {
            None
        }
    };

    // Add direction to target
    let td = add_dir(target, dir, count);
    // Swap the lines
    swap_line(lua, line, td, col)?;

    // Auto-indent the line
    vim::api::nvim_exec(lua, "silent! normal! v=", false)?;

    Ok(())
}

/// Public function to move line up
fn move_line_up(lua: &Lua, _: ()) -> LuaResult<()> {
    move_line(lua, -1)
}

/// Public function to move line down
fn move_line_down(lua: &Lua, _: ()) -> LuaResult<()> {
    move_line(lua, 1)
}

#[mlua::lua_module]
fn moveline(lua: &Lua) -> LuaResult<LuaTable> {
    ModuleBuilder::new(lua)
        .with_fn("move_line_up", move_line_up)?
        .with_fn("move_line_down", move_line_down)?
        .build()
}
