use nvim_utils::prelude::*;

fn move_line_up(lua: &Lua, _: ()) -> LuaResult<()> {
    move_line(lua, -1)
}

fn move_line_down(lua: &Lua, _: ()) -> LuaResult<()> {
    move_line(lua, 1)
}
fn calc_fold(lua: &Lua, line: u64, dir: i8) -> LuaResult<i64> {
    Ok(if dir > 0 {
        vim::func::foldclosedend(lua, line + dir as u64)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
    } else {
        vim::func::foldclosed(lua, line - dir.abs() as u64)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
    })
}

fn indent(lua: &Lua, amount: u64, start_line: u64, end_line: Option<u64>) -> LuaResult<()> {
    let end_line = end_line.unwrap_or(start_line);

    let c_indent = count_indent(lua, start_line)?;
    let diff = amount - c_indent;

    vim::cmd(lua, "silent! normal! ==")
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    let new_indent = count_indent(lua, start_line)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;

    vim::cmd(
        lua,
        &format!(
            "silent! {},{}{}",
            start_line,
            end_line,
            (0..new_indent).map(|_| "<").collect::<String>()
        ),
    )
    .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    vim::cmd(
        lua,
        &format!(
            "silent! {},{}{}",
            start_line,
            end_line,
            (0..c_indent).map(|_| ">").collect::<String>()
        ),
    )
    .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;

    if c_indent != new_indent && diff != 0 {
        if c_indent < new_indent {
            vim::cmd(
                lua,
                &format!(
                    "silent! {},{}{}",
                    start_line,
                    end_line,
                    (0..new_indent - c_indent).map(|_| ">").collect::<String>()
                ),
            )
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
        } else {
            vim::cmd(
                lua,
                &format!(
                    "silent! {},{}{}",
                    start_line,
                    end_line,
                    (0..c_indent - new_indent).map(|_| "<").collect::<String>()
                ),
            )
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
        }
    } else if diff > 0 {
        vim::cmd(
            lua,
            &format!(
                "silent! {},{}{}",
                start_line,
                end_line,
                (0..diff).map(|_| ">").collect::<String>()
            ),
        )
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    }

    Ok(())
}

fn move_line(lua: &Lua, dir: i8) -> LuaResult<()> {
    let last_line = vim::func::line(lua, "$")?;

    let cursor_pos = vim::api::nvim_win_get_cursor(lua, 0)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    let line = cursor_pos.get(1)?;

    if line == 1 && dir < 0 {
        return Ok(());
    } else if line == last_line && dir > 0 {
        return Ok(());
    }

    if line >= 1 && line <= last_line {
        let mut target = line;
        let fold = calc_fold(lua, line, dir)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;

        if fold != -1 {
            target = fold as u64;
        }

        let td = td(target, dir);
        let amount = calc_indent(lua, td, dir)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
        swap_line(lua, line, td)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
        indent(lua, amount, td, None)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    }

    Ok(())
}

fn td(line: u64, dir: i8) -> u64 {
    if dir > 0 {
        line + dir as u64
    } else {
        line - dir.abs() as u64
    }
}

fn swap_line(lua: &Lua, source: u64, target: u64) -> LuaResult<()> {
    let source_line = vim::func::getline(lua, source, None)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    let target_line = vim::func::getline(lua, target, None)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    let source_line = source_line.into_string().unwrap();
    let target_line = target_line.into_string().unwrap();
    vim::func::setline(lua, source, &target_line)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    vim::func::setline(lua, target, &source_line)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;

    Ok(())
}

fn count_indent(lua: &Lua, line: u64) -> LuaResult<u64> {
    Ok(vim::func::indent(lua, line)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?
        / vim::func::shiftwidth(lua)
            .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?)
}

fn calc_indent(lua: &Lua, target: u64, dir: i8) -> LuaResult<u64> {
    let target_count = count_indent(lua, target)
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;
    let next_count = count_indent(lua, td(target, dir))
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", e, line!())))?;

    if target_count < next_count {
        Ok(next_count)
    } else {
        Ok(target_count)
    }
}

#[mlua::lua_module]
fn moveline(lua: &Lua) -> LuaResult<LuaTable> {
    ModuleBuilder::new(lua)
        .with_fn("move_line_up", move_line_up)?
        .with_fn("move_line_down", move_line_down)?
        .build()
}
