use sdl2::pixels::Color;
use sdl2::rect::Rect;
use crate::ui::{BUTTON_COLOR, BUTTON_HOVER_COLOR, BUTTON_TEXT_COLOR};
use crate::ui::renderer::Renderer;

pub struct Button {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: String,
    pub is_hovered: bool,
}

impl Button {
    pub fn new(x: i32, y: i32, width: i32, height: i32, text: String) -> Self {
        Button {
            x,
            y,
            width,
            height,
            text,
            is_hovered: false,
        }
    }

    pub fn render(&self, renderer: &mut Renderer, text_renderer: &mut crate::ui::text::TextRenderer) -> Result<(), Box<dyn std::error::Error>> {
        // Choose color based on hover state
        let color = if self.is_hovered { BUTTON_HOVER_COLOR } else { BUTTON_COLOR };
        
        // Draw button background
        renderer.canvas.set_draw_color(color);
        renderer.canvas.fill_rect(Rect::new(self.x, self.y, self.width as u32, self.height as u32))?;
        
        // Draw button border
        renderer.canvas.set_draw_color(Color::RGB(255, 255, 255));
        renderer.canvas.draw_rect(Rect::new(self.x, self.y, self.width as u32, self.height as u32))?;
        
        // Draw button text
        let text_x = self.x + (self.width / 2) - 30; // Approximate text centering
        let text_y = self.y + (self.height / 2) - 8;
        
        text_renderer.render_text(
            &mut renderer.canvas,
            &self.text,
            text_x,
            text_y,
            BUTTON_TEXT_COLOR,
            16,
        )?;
        
        Ok(())
    }

    pub fn is_clicked(&self, mouse_x: i32, mouse_y: i32) -> bool {
        mouse_x >= self.x && mouse_x <= self.x + self.width &&
        mouse_y >= self.y && mouse_y <= self.y + self.height
    }

    pub fn update_hover(&mut self, mouse_x: i32, mouse_y: i32) {
        self.is_hovered = self.is_clicked(mouse_x, mouse_y);
    }
} 