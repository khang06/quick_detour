# quick_detour
A simple wrapper for the [minhook-sys](https://github.com/YaLTeR/minhook-sys) library that makes making hooks much more concise.

### Example
```rust
use std::ffi::{CStr, CString};

use quick_detour::make_hook;
use windows::Win32::{
    Foundation::{BOOL, HWND},
    System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

unsafe fn get_api(
    module_name: &'static str,
    export: &'static str,
) -> unsafe extern "system" fn() -> isize {
    let module = LoadLibraryW(module_name);
    if let Err(x) = module.ok() {
        panic!("Failed to load {}! {}", module_name, x);
    }
    GetProcAddress(module, export)
        .unwrap_or_else(|| panic!("Failed to load export {} from {}!", export, module_name))
}

pub unsafe fn install_hooks() -> Result<(), i32> {
    make_hook!(
        std::mem::transmute(get_api("user32.dll", "SetWindowTextA")),
        unsafe extern "system" fn(HWND, *const u8) -> BOOL,
        |orig, hwnd, text| -> BOOL {
            if !text.is_null() && let Ok(cstr) = CStr::from_ptr(text as _).to_str() {
                let new_str = cstr.to_string() + " (hooked!)";
                let new_cstr = CString::new(new_str).unwrap();
                orig(hwnd, new_cstr.as_ptr() as _)
            } else {
                orig(hwnd, text)
            }
        }
    )?;

    Ok(())
}
```