use nvim_oxi as oxi;
use oxi::api::ToFunction;
use oxi::{Dictionary, Error, Function, Result};

mod core;
mod ffi;

use self::core::{move_line, Direction::*};

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
