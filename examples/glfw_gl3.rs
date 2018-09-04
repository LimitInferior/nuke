extern crate gl;
extern crate glfw;
extern crate libc;
extern crate nuke;

use nuke::*;
use glfw::{Context, Key, Action};
use gl::types::*;

use std::sync::mpsc::Receiver;
use std::ffi::CString;
use std::ptr;
use std::str;
use std::mem;

macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    }
}

macro_rules! offset_of {
    ($ty: ty, $field: ident) => {
        unsafe {
            // Work with an actual instance of the type since using a null pointer is UB
            let addr: $ty = mem::uninitialized();
            let base = &addr as *const _ as usize;
            let path = &addr.$field as *const _ as usize;
            path - base
        }
    };
}

const TEXT_MAX: usize = 256;
const DOUBLE_CLICK_LO: f32 = 0.02;
const DOUBLE_CLICK_HI: f32 = 0.2;

struct Device {
    cmds: nk_buffer,
    null: nk_draw_null_texture,
    vbo: GLuint,
    vao: GLuint,
    ebo: GLuint,
    prog: GLuint,
    vert_shdr: GLuint,
    frag_shdr: GLuint,
    attrib_pos: GLint,
    attrib_uv: GLint,
    attrib_col: GLint,
    uniform_tex: GLint,
    uniform_proj: GLint,
    font_tex: GLuint,
}

impl Device {
    unsafe fn new() -> Self {
        let mut cmds: nk_buffer = mem::zeroed();
        nk_buffer_init_default(&mut cmds);
        let mut dev = Self {
            cmds,
            null: mem::zeroed(),
            vbo: 0,
            vao: 0,
            ebo: 0,
            prog: gl::CreateProgram(),
            vert_shdr: gl::CreateShader(gl::VERTEX_SHADER),
            frag_shdr: gl::CreateShader(gl::FRAGMENT_SHADER),
            attrib_pos: 0,
            attrib_uv: 0,
            attrib_col: 0,
            uniform_tex: 0,
            uniform_proj: 0,
            font_tex: 0,
        };
        let c_str_vert = CString::new(VERTEX_SHADER.as_bytes()).unwrap();
        gl::ShaderSource(dev.vert_shdr, 1, &c_str_vert.as_ptr(), ptr::null());
        gl::CompileShader(dev.vert_shdr);
        let mut success = gl::FALSE as GLint;
        gl::GetShaderiv(dev.vert_shdr, gl::COMPILE_STATUS, &mut success);
        assert_eq!(gl::TRUE as GLint, success);
        let c_str_frag = CString::new(FRAGMENT_SHADER.as_bytes()).unwrap();
        gl::ShaderSource(dev.frag_shdr, 1, &c_str_frag.as_ptr(), ptr::null());
        gl::CompileShader(dev.frag_shdr);
        gl::GetShaderiv(dev.frag_shdr, gl::COMPILE_STATUS, &mut success);
        assert_eq!(gl::TRUE as GLint, success);
        gl::AttachShader(dev.prog, dev.vert_shdr);
        gl::AttachShader(dev.prog, dev.frag_shdr);
        gl::LinkProgram(dev.prog);
        gl::GetProgramiv(dev.prog, gl::LINK_STATUS, &mut success);
        assert_eq!(gl::TRUE as GLint, success);
        dev.uniform_tex = gl::GetUniformLocation(dev.prog, b"Texture\0".as_ptr() as _);
        dev.uniform_proj = gl::GetUniformLocation(dev.prog, b"ProjMtx\0".as_ptr() as _);
        dev.attrib_pos = gl::GetAttribLocation(dev.prog, b"Position\0".as_ptr() as _);
        dev.attrib_uv = gl::GetAttribLocation(dev.prog, b"TexCoord\0".as_ptr() as _);
        dev.attrib_col = gl::GetAttribLocation(dev.prog, b"Color\0".as_ptr() as _);
        /* buffer setup */
        let vs = mem::size_of::<Vertex>() as GLsizei;
        let vp = offset_of!(Vertex, position);
        let vt = offset_of!(Vertex, uv);
        let vc = offset_of!(Vertex, col);
        gl::GenBuffers(1, &mut dev.vbo);
        gl::GenBuffers(1, &mut dev.ebo);
        gl::GenVertexArrays(1, &mut dev.vao);

        gl::BindVertexArray(dev.vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, dev.vbo);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, dev.ebo);

        gl::EnableVertexAttribArray(dev.attrib_pos as GLuint);
        gl::EnableVertexAttribArray(dev.attrib_uv as GLuint);
        gl::EnableVertexAttribArray(dev.attrib_col as GLuint);

        gl::VertexAttribPointer(dev.attrib_pos as GLuint, 2, gl::FLOAT, gl::FALSE, vs, vp as _);
        gl::VertexAttribPointer(dev.attrib_uv as GLuint, 2, gl::FLOAT, gl::FALSE, vs, vt as _);
        gl::VertexAttribPointer(dev.attrib_col as GLuint, 4, gl::UNSIGNED_BYTE, gl::TRUE, vs, vc as _);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
        dev
    }

    unsafe fn upload_atlas(&mut self, image: *const libc::c_void, width: libc::c_int, height: libc::c_int) {
        gl::GenTextures(1, &mut self.font_tex);
        gl::BindTexture(gl::TEXTURE_2D, self.font_tex);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as GLint, width as GLsizei, height as GLsizei, 0,
                    gl::RGBA, gl::UNSIGNED_BYTE, image as _);
    }
}

struct GlfwContext {
    pub window: glfw::Window,
    width: i32,
    height: i32,
    display_width: i32,
    display_height: i32,
    device: Device,
    context: nk_context,
    atlas: nk_font_atlas,
    fb_scale: nk_vec2,
    text: [u32; TEXT_MAX],
    text_len: usize,
    scroll: nk_vec2,
    last_button_click: f64,
    is_double_click_down: i32,
    double_click_pos: nk_vec2,
}

fn pressed(action: Action) -> i32 {
    if action == Action::Press {
        1
    } else {
        0
    }
}

impl GlfwContext {
    unsafe fn new(window: glfw::Window, max_vertex_buffer: usize, max_element_buffer: usize) -> Self {
        let mut context: nk_context = mem::zeroed();
        nk_init_default(&mut context, ptr::null());
    // if (init_state == NK_GLFW3_INSTALL_CALLBACKS) {
    //     glfwSetScrollCallback(win, nk_gflw3_scroll_callback);
    //     glfwSetCharCallback(win, nk_glfw3_char_callback);
    //     glfwSetMouseButtonCallback(win, nk_glfw3_mouse_button_callback);
    // }
    // glfw.ctx.clip.copy = nk_glfw3_clipbard_copy;
    // glfw.ctx.clip.paste = nk_glfw3_clipbard_paste;
    // glfw.ctx.clip.userdata = nk_handle_ptr(0);
        let mut device = Device::new();
        let mut atlas: nk_font_atlas = mem::zeroed();
        nk_font_atlas_init_default(&mut atlas);
        nk_font_atlas_begin(&mut atlas);
        let (mut w, mut h) = (0, 0);
        let image = nk_font_atlas_bake(&mut atlas, &mut w, &mut h, NK_FONT_ATLAS_RGBA32);
        device.upload_atlas(image, w, h);
        nk_font_atlas_end(&mut atlas, nk_handle_id(device.font_tex as i32), &mut device.null);
        if atlas.default_font != ptr::null_mut() {
            nk_style_set_font(&mut context, &(*atlas.default_font).handle);
        }
        Self {
            window,
            width: 0,
            height: 0,
            display_width: 0,
            display_height: 0,
            device: Device::new(),
            context,
            atlas,
            fb_scale: nk_vec2(0.0, 0.0),
            text: [0; TEXT_MAX],
            text_len: 0,
            scroll: nk_vec2(0.0, 0.0),
            last_button_click: 0.0,
            is_double_click_down: 0,
            double_click_pos: nk_vec2(0.0, 0.0),
        }
    }

    unsafe fn new_frame(&mut self) {
        let (w, h) = self.window.get_size();
        self.width = w;
        self.height = h;
        let (fw, fh) = self.window.get_framebuffer_size();
        self.display_width = fw;
        self.display_height = fh;
        self.fb_scale.x = fw as f32 / w as f32;
        self.fb_scale.y = fh as f32 / h as f32;
        let ctx = &mut self.context;
        nk_input_begin(ctx);
        for i in 0 .. self.text_len {
            nk_input_unicode(ctx, self.text[i]);
        }
// #ifdef NK_GLFW_GL3_MOUSE_GRABBING
//     /* optional grabbing behavior */
//     if (ctx->input.mouse.grab)
//         glfwSetInputMode(glfw.win, GLFW_CURSOR, GLFW_CURSOR_HIDDEN);
//     else if (ctx->input.mouse.ungrab)
//         glfwSetInputMode(glfw.win, GLFW_CURSOR, GLFW_CURSOR_NORMAL);
// #endif

        nk_input_key(ctx, NK_KEY_DEL, pressed(self.window.get_key(Key::Delete)));
        nk_input_key(ctx, NK_KEY_ENTER, pressed(self.window.get_key(Key::Enter)));
        nk_input_key(ctx, NK_KEY_TAB, pressed(self.window.get_key(Key::Tab)));
        nk_input_key(ctx, NK_KEY_BACKSPACE, pressed(self.window.get_key(Key::Backspace)));
        nk_input_key(ctx, NK_KEY_UP, pressed(self.window.get_key(Key::Up)));
        nk_input_key(ctx, NK_KEY_DOWN, pressed(self.window.get_key(Key::Down)));
        nk_input_key(ctx, NK_KEY_TEXT_START, pressed(self.window.get_key(Key::Home)));
        nk_input_key(ctx, NK_KEY_TEXT_END, pressed(self.window.get_key(Key::End)));
        nk_input_key(ctx, NK_KEY_SCROLL_START, pressed(self.window.get_key(Key::Home)));
        nk_input_key(ctx, NK_KEY_SCROLL_END, pressed(self.window.get_key(Key::End)));
        nk_input_key(ctx, NK_KEY_SCROLL_DOWN, pressed(self.window.get_key(Key::Down)));
        nk_input_key(ctx, NK_KEY_SCROLL_UP, pressed(self.window.get_key(Key::Up)));
        nk_input_key(ctx, NK_KEY_SHIFT, pressed(self.window.get_key(Key::LeftShift)) | pressed(self.window.get_key(Key::RightShift)));

        if pressed(self.window.get_key(Key::LeftControl)) | pressed(self.window.get_key(Key::RightControl)) != 0 {
            nk_input_key(ctx, NK_KEY_COPY, pressed(self.window.get_key(Key::C)));
            nk_input_key(ctx, NK_KEY_PASTE, pressed(self.window.get_key(Key::V)));
            nk_input_key(ctx, NK_KEY_CUT, pressed(self.window.get_key(Key::X)));
            nk_input_key(ctx, NK_KEY_TEXT_UNDO, pressed(self.window.get_key(Key::Z)));
            nk_input_key(ctx, NK_KEY_TEXT_REDO, pressed(self.window.get_key(Key::R)));
            nk_input_key(ctx, NK_KEY_TEXT_WORD_LEFT, pressed(self.window.get_key(Key::Left)));
            nk_input_key(ctx, NK_KEY_TEXT_WORD_RIGHT, pressed(self.window.get_key(Key::Right)));
            nk_input_key(ctx, NK_KEY_TEXT_LINE_START, pressed(self.window.get_key(Key::B)));
            nk_input_key(ctx, NK_KEY_TEXT_LINE_END, pressed(self.window.get_key(Key::E)));
        } else {
            nk_input_key(ctx, NK_KEY_LEFT, pressed(self.window.get_key(Key::Left)));
            nk_input_key(ctx, NK_KEY_RIGHT, pressed(self.window.get_key(Key::Right)));
            nk_input_key(ctx, NK_KEY_COPY, 0);
            nk_input_key(ctx, NK_KEY_PASTE, 0);
            nk_input_key(ctx, NK_KEY_CUT, 0);
            nk_input_key(ctx, NK_KEY_SHIFT, 0);
        }

        let (x, y) = self.window.get_cursor_pos();
        let (x, y) = (x as i32, y as i32);
        nk_input_motion(ctx, x, y);
    // #ifdef NK_GLFW_GL3_MOUSE_GRABBING
    //     if (ctx->input.mouse.grabbed) {
    //         glfwSetCursorPos(glfw.win, ctx->input.mouse.prev.x, ctx->input.mouse.prev.y);
    //         ctx->input.mouse.pos.x = ctx->input.mouse.prev.x;
    //         ctx->input.mouse.pos.y = ctx->input.mouse.prev.y;
    //     }
    // #endif
        nk_input_button(ctx, NK_BUTTON_LEFT, x, y, pressed(self.window.get_mouse_button(glfw::MouseButtonLeft)));
        nk_input_button(ctx, NK_BUTTON_MIDDLE, x, y, pressed(self.window.get_mouse_button(glfw::MouseButtonMiddle)));
        nk_input_button(ctx, NK_BUTTON_RIGHT, x, y, pressed(self.window.get_mouse_button(glfw::MouseButtonRight)));
        nk_input_button(ctx, NK_BUTTON_DOUBLE, self.double_click_pos.x as i32, self.double_click_pos.y as i32, self.is_double_click_down);
        nk_input_scroll(ctx, self.scroll);
        nk_input_end(ctx);
        self.text_len = 0;
        self.scroll = nk_vec2(0.0, 0.0);
    }
}

#[repr(C)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    col: [u8; 4],
}

// settings
const WINDOW_WIDTH: u32 = 1200;
const WINDOW_HEIGHT: u32 = 800;
const MAX_VERTEX_BUFFER: usize = 512 * 1024;
const MAX_ELEMENT_BUFFER: usize = 128 * 1024;

const VERTEX_SHADER: &str = r#"
    #version 330
    uniform mat4 ProjMtx;
    in vec2 Position;
    in vec2 TexCoord;
    in vec4 Color;
    out vec2 Frag_UV;
    out vec4 Frag_Color;
    void main() {
       Frag_UV = TexCoord;
       Frag_Color = Color;
       gl_Position = ProjMtx * vec4(Position.xy, 0, 1);
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 330
    precision mediump float;
    uniform sampler2D Texture;
    in vec2 Frag_UV;
    in vec4 Frag_Color;
    out vec4 Out_Color;
    void main(){
       Out_Color = Frag_Color * texture(Texture, Frag_UV.st);
    }
"#;

#[allow(non_snake_case)]
pub fn main() {
    // glfw: initialize and configure
    // ------------------------------
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    // glfw window creation
    // --------------------
    let (mut window, events) = glfw.create_window(WINDOW_WIDTH, WINDOW_HEIGHT, "LearnOpenGL", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window");

    window.make_current();
    window.set_key_polling(true);
    window.set_framebuffer_size_polling(true);
    let mut bg = nk_colorf {
        r: 0.10, 
        g: 0.18,
        b: 0.24,
        a: 1.0,
    };
    // gl: load all OpenGL function pointers
    // ---------------------------------------
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    unsafe {
        let mut context = GlfwContext::new(window, MAX_VERTEX_BUFFER, MAX_ELEMENT_BUFFER);
        // render loop
        // -----------
        while !context.window.should_close() {
            process_events(&mut context.window, &events);
            context.new_frame();
            // events
            // -----
        {
            let ctx = &mut context.context;
            if nk_begin(ctx, c_str!("Demo"), nk_rect(50.0, 50.0, 230.0, 250.0), NK_WINDOW_BORDER | NK_WINDOW_MOVABLE | NK_WINDOW_SCALABLE | NK_WINDOW_MINIMIZABLE | NK_WINDOW_TITLE) != 0 {
                // #[derive(PartialEq)]
                // enum Op {
                //     EASY, 
                //     HARD
                // }
                // let mut op = Op::EASY;
                // let mut property = 20;
                // nk_layout_row_static(ctx, 30.0, 80, 1);
                // if nk_button_label(ctx, c_str!("button")) != 0 {
                //     println!("button pressed");
                // }
                // nk_layout_row_dynamic(ctx, 30.0, 2);
                // if nk_option_label(ctx, c_str!("easy"), if op == Op::EASY { 1 } else { 0 }) != 0 {
                //     op = Op::EASY;
                // }
                // if nk_option_label(ctx, c_str!("hard"), if op == Op::HARD { 1 } else { 0 }) != 0 {
                //     op = Op::HARD;
                // }
                // nk_layout_row_dynamic(ctx, 25.0, 1);
                // nk_property_int(ctx, c_str!("Compression:"), 0, &mut property, 100, 10, 1.0);

                // nk_layout_row_dynamic(ctx, 20.0, 1);
                // nk_label(ctx, c_str!("background:"), NK_TEXT_LEFT);
                // nk_layout_row_dynamic(ctx, 25.0, 1);
                // if nk_combo_begin_color(ctx, nk_rgb_cf(bg), nk_vec2(nk_widget_width(ctx), 400.0)) != 0 {
                //     nk_layout_row_dynamic(ctx, 120.0, 1);
                //     bg = nk_color_picker(ctx, bg, NK_RGBA);
                //     nk_layout_row_dynamic(ctx, 25.0, 1);
                //     bg.r = nk_propertyf(ctx, c_str!("#R:"), 0.0, bg.r, 1.0, 0.01, 0.005);
                //     bg.g = nk_propertyf(ctx, c_str!("#G:"), 0.0, bg.g, 1.0, 0.01, 0.005);
                //     bg.b = nk_propertyf(ctx, c_str!("#B:"), 0.0, bg.b, 1.0, 0.01, 0.005);
                //     bg.a = nk_propertyf(ctx, c_str!("#A:"), 0.0, bg.a, 1.0, 0.01, 0.005);
                //     nk_combo_end(ctx);
                // }
            }
            nk_end(ctx);
        }
            // render
            // ------
                gl::ClearColor(bg.r, bg.g, bg.b, bg.a);
                gl::Clear(gl::COLOR_BUFFER_BIT);

                // draw our first triangle
                // gl::UseProgram(shaderProgram);
                // gl::BindVertexArray(VAO); // seeing as we only have a single VAO there's no need to bind it every time, but we'll do so to keep things a bit more organized
                // gl::DrawArrays(gl::TRIANGLES, 0, 3);
                // glBindVertexArray(0); // no need to unbind it every time

            // glfw: swap buffers and poll IO events (keys pressed/released, mouse moved etc.)
            // -------------------------------------------------------------------------------
            context.window.swap_buffers();
            glfw.poll_events();
        }
    }
}

// NOTE: not the same version as in common.rs!
fn process_events(window: &mut glfw::Window, events: &Receiver<(f64, glfw::WindowEvent)>) {
    for (_, event) in glfw::flush_messages(events) {
        match event {
            glfw::WindowEvent::FramebufferSize(width, height) => {
                // make sure the viewport matches the new window dimensions; note that width and
                // height will be significantly larger than specified on retina displays.
                unsafe { gl::Viewport(0, 0, width, height) }
            }
            glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
            _ => {}
        }
    }
}