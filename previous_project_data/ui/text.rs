use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct TextRenderer {
    // Simple text renderer without external fonts
}

impl TextRenderer {
    pub fn new(_canvas: &Canvas<Window>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(TextRenderer {})
    }

    pub fn render_text(
        &self,
        canvas: &mut Canvas<Window>,
        text: &str,
        x: i32,
        y: i32,
        color: Color,
        size: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Simple text rendering using rectangles
        canvas.set_draw_color(color);
        
        let char_width = (size / 2) as u32;
        let char_height = size as u32;
        
        for (i, _char) in text.chars().enumerate() {
            let char_x = x + (i as i32 * char_width as i32);
            canvas.fill_rect(Rect::new(char_x, y, char_width, char_height))?;
        }
        
        Ok(())
    }
} 