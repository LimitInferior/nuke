use libc;
use math::{Point, Rect};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Input {
    pub keyboard: Keyboard,
    pub mouse: Mouse,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Mouse {
    pub buttons: [MouseButton; 4],
    pub pos: Point,
    pub prev: Point,
    pub delta: Point,
    pub scroll_delta: Point,
    pub grab: libc::c_uchar,
    pub grabbed: libc::c_uchar,
    pub ungrab: libc::c_uchar,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct MouseButton {
    pub down: libc::c_int,
    pub clicked: libc::c_uint,
    pub clicked_pos: Point,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Keyboard {
    pub keys: [Key; 30],
    pub text: [libc::c_char; 16],
    pub text_len: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Key {
    pub down: libc::c_int,
    pub clicked: libc::c_uint,
}

pub type Buttons = libc::c_uint;
pub const NK_BUTTON_MAX: Buttons = 4;
pub const NK_BUTTON_DOUBLE: Buttons = 3;
pub const NK_BUTTON_RIGHT: Buttons = 2;
pub const NK_BUTTON_MIDDLE: Buttons = 1;
pub const NK_BUTTON_LEFT: Buttons = 0;

pub type Keys = libc::c_uint;
pub const NK_KEY_MAX: Keys = 30;
pub const NK_KEY_SCROLL_UP: Keys = 29;
pub const NK_KEY_SCROLL_DOWN: Keys = 28;
pub const NK_KEY_SCROLL_END: Keys = 27;
/* Shortcuts: scrollbar */
pub const NK_KEY_SCROLL_START: Keys = 26;
pub const NK_KEY_TEXT_WORD_RIGHT: Keys = 25;
pub const NK_KEY_TEXT_WORD_LEFT: Keys = 24;
pub const NK_KEY_TEXT_SELECT_ALL: Keys = 23;
pub const NK_KEY_TEXT_REDO: Keys = 22;
pub const NK_KEY_TEXT_UNDO: Keys = 21;
pub const NK_KEY_TEXT_END: Keys = 20;
pub const NK_KEY_TEXT_START: Keys = 19;
pub const NK_KEY_TEXT_LINE_END: Keys = 18;
pub const NK_KEY_TEXT_LINE_START: Keys = 17;
pub const NK_KEY_TEXT_RESET_MODE: Keys = 16;
pub const NK_KEY_TEXT_REPLACE_MODE: Keys = 15;
/* Shortcuts: text field */
pub const NK_KEY_TEXT_INSERT_MODE: Keys = 14;
pub const NK_KEY_RIGHT: Keys = 13;
pub const NK_KEY_LEFT: Keys = 12;
pub const NK_KEY_DOWN: Keys = 11;
pub const NK_KEY_UP: Keys = 10;
pub const NK_KEY_PASTE: Keys = 9;
pub const NK_KEY_CUT: Keys = 8;
pub const NK_KEY_COPY: Keys = 7;
pub const NK_KEY_BACKSPACE: Keys = 6;
pub const NK_KEY_TAB: Keys = 5;
pub const NK_KEY_ENTER: Keys = 4;
pub const NK_KEY_DEL: Keys = 3;
pub const NK_KEY_CTRL: Keys = 2;
pub const NK_KEY_SHIFT: Keys = 1;
pub const NK_KEY_NONE: Keys = 0;

pub unsafe fn nk_input_has_mouse_click(i: *const Input, id: Buttons) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let btn = &(*i).mouse.buttons[id as usize] as *const MouseButton;
        return if 0 != (*btn).clicked && (*btn).down == 0 {
            1
        } else {
            0
        };
    };
}

pub unsafe fn nk_input_any_mouse_click_in_rect(in_0: *const Input, b: Rect) -> libc::c_int {
    let mut down: libc::c_int = 0;
    let mut i = 0;
    while i < NK_BUTTON_MAX as libc::c_int {
        down = (0 != down || 0 != nk_input_is_mouse_click_in_rect(in_0, i as Buttons, b))
            as libc::c_int;
        i += 1
    }
    return down;
}

pub unsafe fn nk_input_is_mouse_released(i: *const Input, id: Buttons) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        return (0 == (*i).mouse.buttons[id as usize].down
            && 0 != (*i).mouse.buttons[id as usize].clicked) as libc::c_int;
    };
}

pub unsafe fn nk_input_is_key_released(i: *const Input, key: Keys) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let k = &(*i).keyboard.keys[key as usize] as *const Key;
        if 0 == (*k).down && 0 != (*k).clicked
            || 0 != (*k).down && (*k).clicked >= 2 as libc::c_uint
        {
            return 1;
        } else {
            return 0;
        }
    };
}

pub unsafe fn nk_input_is_key_down(i: *const Input, key: Keys) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let k = &(*i).keyboard.keys[key as usize] as *const Key;
        if 0 != (*k).down {
            return 1;
        } else {
            return 0;
        }
    };
}

pub unsafe fn nk_input_is_key_pressed(i: *const Input, key: Keys) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let k = &(*i).keyboard.keys[key as usize] as *const Key;
        if 0 != (*k).down && 0 != (*k).clicked
            || 0 == (*k).down && (*k).clicked >= 2i32 as libc::c_uint
        {
            return 1;
        } else {
            return 0;
        }
    };
}

pub unsafe fn nk_input_mouse_clicked(
    i: *const Input,
    id: Buttons,
    rect: Rect,
) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else if 0 == nk_input_is_mouse_hovering_rect(i, rect) {
        return 0;
    } else {
        return nk_input_is_mouse_click_in_rect(i, id, rect);
    };
}

pub unsafe fn nk_input_is_mouse_click_in_rect(
    i: *const Input,
    id: Buttons,
    b: Rect,
) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let btn = &(*i).mouse.buttons[id as usize] as *const MouseButton;
        return if 0 != nk_input_has_mouse_click_down_in_rect(i, id, b, 0)
            && 0 != (*btn).clicked
        {
            1
        } else {
            0
        };
    };
}

pub unsafe fn nk_input_has_mouse_click_down_in_rect(
    i: *const Input,
    id: Buttons,
    b: Rect,
    down: libc::c_int,
) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let btn = &(*i).mouse.buttons[id as usize] as *const MouseButton;
        return (0 != nk_input_has_mouse_click_in_rect(i, id, b) && (*btn).down == down)
            as libc::c_int;
    };
}

pub unsafe fn nk_input_is_mouse_prev_hovering_rect(
    i: *const Input,
    rect: Rect,
) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        return (rect.x <= (*i).mouse.prev.x
            && (*i).mouse.prev.x < rect.x + rect.w
            && (rect.y <= (*i).mouse.prev.y && (*i).mouse.prev.y < rect.y + rect.h))
            as libc::c_int;
    };
}

pub unsafe fn nk_input_is_mouse_pressed(i: *const Input, id: Buttons) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let b = &(*i).mouse.buttons[id as usize] as *const MouseButton;
        if 0 != (*b).down && 0 != (*b).clicked {
            return 1;
        } else {
            return 0;
        }
    };
}

pub unsafe fn nk_input_is_mouse_down(i: *const Input, id: Buttons) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        return (*i).mouse.buttons[id as usize].down;
    };
}

pub unsafe fn nk_input_has_mouse_click_in_rect(
    i: *const Input,
    id: Buttons,
    b: Rect,
) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        let btn = &(*i).mouse.buttons[id as usize] as *const MouseButton;
        if !(b.x <= (*btn).clicked_pos.x
            && (*btn).clicked_pos.x < b.x + b.w
            && (b.y <= (*btn).clicked_pos.y && (*btn).clicked_pos.y < b.y + b.h))
        {
            return 0;
        } else {
            return 1;
        }
    };
}

pub unsafe fn nk_input_is_mouse_hovering_rect(i: *const Input, rect: Rect) -> libc::c_int {
    if i.is_null() {
        return 0;
    } else {
        return (rect.x <= (*i).mouse.pos.x
            && (*i).mouse.pos.x < rect.x + rect.w
            && (rect.y <= (*i).mouse.pos.y && (*i).mouse.pos.y < rect.y + rect.h))
            as libc::c_int;
    };
}
