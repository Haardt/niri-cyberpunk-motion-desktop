use crate::buffer::Buffer;
use crate::config::Config;
pub use smithay_client_toolkit::seat::keyboard::Modifiers as ModifiersState;
#[derive(Clone, Copy)]
pub enum KeyState {
    Pressed,
}

pub struct DrawContext<'a> {
    pub buf: &'a mut Buffer<'a>,
    pub force: bool,
    pub config: &'a Config,
}

#[derive(Debug)]
pub struct DrawReport {
    pub width: u32,
    pub height: u32,
    pub damage: Vec<(i32, i32, i32, i32)>,
    pub full_damage: bool,
}

impl DrawReport {
    pub fn empty(width: u32, height: u32) -> DrawReport {
        DrawReport {
            width,
            height,
            damage: Vec::new(),
            full_damage: false,
        }
    }
}

pub trait Widget {
    fn poll(&mut self) -> bool {
        false
    }
    fn wake_fd(&self) -> Option<i32> {
        None
    }
    fn authenticated(&self) -> bool {
        false
    }
    fn size(&self) -> (u32, u32);
    fn draw(
        &mut self,
        ctx: &mut DrawContext,
        pos: (u32, u32),
    ) -> Result<DrawReport, ::std::io::Error>;

    fn keyboard_input(
        &mut self,
        keysym: u32,
        modifier_state: ModifiersState,
        key_state: KeyState,
        interpreted: Option<String>,
    );
    fn mouse_click(&mut self, button: u32, pos: (u32, u32));
}
