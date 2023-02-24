use nvim_utils::prelude::*;

/// Handle folds
fn calc_fold(lua: &Lua, line: u64, dir: i8) -> LuaResult<i64> {
    if dir > 0 {
        vim::func::foldclosedend(lua, line + dir as u64)
    } else {
        vim::func::foldclosed(lua, line - dir.abs() as u64)
    }
}

/// Add direction to line number
fn add_dir(num: u64, dir: i8) -> u64 {
    if dir >= 0 {
        num + dir as u64
    } else {
        num - dir.abs() as u64
    }
}

/// Count the indent of a line
fn count_indent(lua: &Lua, line: u64) -> LuaResult<u64> {
    Ok(vim::func::indent(lua, line)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
        / vim::func::shiftwidth(lua)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?)
}

/// Calculate the indent of the target line
fn calc_indent(lua: &Lua, target: u64, dir: i8) -> LuaResult<u64> {
    let target_count = count_indent(lua, target)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    let next_count = count_indent(lua, add_dir(target, dir))
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;

    if target_count < next_count {
        Ok(next_count)
    } else {
        Ok(target_count)
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
fn move_line(lua: &Lua, dir: i8) -> LuaResult<()> {
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

    // Add direction to target
    let td = add_dir(target, dir);
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
