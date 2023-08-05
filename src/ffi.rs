use std::ffi::CString;

use nvim_oxi as oxi;
use oxi::lua::cstr;
use oxi::lua::ffi::{
    lua_call, lua_getfield, lua_getglobal, lua_gettop, lua_pushstring, lua_settop, lua_type,
    LUA_TNIL,
};
use oxi::lua::Poppable;
use oxi::lua::{ffi::lua_State, with_state};
use oxi::Function;
use oxi::Result;
use oxi::{Dictionary, Error};

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

pub fn viml_fn<A, R>(name: &str) -> Option<Function<A, R>> {
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

pub fn require(module: &str) -> Result<Dictionary> {
    let module = CString::new(module).unwrap();
    unsafe {
        with_state(|l| {
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
