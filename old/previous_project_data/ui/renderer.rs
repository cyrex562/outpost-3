use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::Sdl;

pub struct Renderer {
    pub canvas: Canvas<Window>,
    _sdl_context: Sdl,
}

impl Renderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        
        let window = video_subsystem
            .window("Harsh Realm - Solar System", crate::ui::SCREEN_WIDTH, crate::ui::SCREEN_HEIGHT)
            .position_centered()
            .build()?;

        let canvas = window.into_canvas().build()?;

        Ok(Renderer {
            canvas,
            _sdl_context: sdl_context,
        })
    }

    pub fn draw_circle(&mut self, center_x: i32, center_y: i32, radius: i32, color: Color) -> Result<(), Box<dyn std::error::Error>> {
        self.canvas.set_draw_color(color);
        
        // Simple circle drawing using multiple small rectangles
        for y in -radius..=radius {
            for x in -radius..=radius {
                if x * x + y * y <= radius * radius {
                    self.canvas.fill_rect(Rect::new(center_x + x, center_y + y, 1, 1))?;
                }
            }
        }
        
        Ok(())
    }

    pub fn draw_ellipse(&mut self, center_x: i32, center_y: i32, radius_x: i32, radius_y: i32, color: Color) -> Result<(), Box<dyn std::error::Error>> {
        self.canvas.set_draw_color(color);
        
        // Simple ellipse drawing using the ellipse equation
        for y in -radius_y..=radius_y {
            for x in -radius_x..=radius_x {
                if (x * x * radius_y * radius_y + y * y * radius_x * radius_x) <= (radius_x * radius_x * radius_y * radius_y) {
                    self.canvas.fill_rect(Rect::new(center_x + x, center_y + y, 1, 1))?;
                }
            }
        }
        
        Ok(())
    }

    pub fn draw_ring(&mut self, center_x: i32, center_y: i32, inner_radius: i32, outer_radius: i32, color: Color) -> Result<(), Box<dyn std::error::Error>> {
        self.canvas.set_draw_color(color);
        
        // Draw ring (annulus) by drawing outer circle and clearing inner circle
        for y in -outer_radius..=outer_radius {
            for x in -outer_radius..=outer_radius {
                let distance_squared = x * x + y * y;
                if distance_squared <= outer_radius * outer_radius && distance_squared >= inner_radius * inner_radius {
                    self.canvas.fill_rect(Rect::new(center_x + x, center_y + y, 1, 1))?;
                }
            }
        }
        
        Ok(())
    }
} 