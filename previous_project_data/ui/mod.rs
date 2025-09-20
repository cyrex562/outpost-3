pub mod renderer;
pub mod solar_system_view;
pub mod button;
pub mod text;

use sdl2::pixels::Color;

pub const SCREEN_WIDTH: u32 = 1920;
pub const SCREEN_HEIGHT: u32 = 1080;

// Color definitions for solar system bodies
pub const SUN_COLOR: Color = Color::RGB(255, 255, 200); // Light yellow
pub const EARTH_COLOR: Color = Color::RGB(100, 150, 255); // Blue
pub const MARS_COLOR: Color = Color::RGB(200, 100, 50); // Rust red
pub const VENUS_COLOR: Color = Color::RGB(255, 150, 100); // Orange-brown
pub const MERCURY_COLOR: Color = Color::RGB(150, 150, 150); // Medium gray
pub const ASTEROID_BELT_COLOR: Color = Color::RGB(150, 255, 150); // Light green

// UI colors
pub const BUTTON_COLOR: Color = Color::RGB(80, 80, 80);
pub const BUTTON_HOVER_COLOR: Color = Color::RGB(120, 120, 120);
pub const BUTTON_TEXT_COLOR: Color = Color::RGB(255, 255, 255);
pub const DATE_TEXT_COLOR: Color = Color::RGB(255, 255, 255);
pub const ORBITAL_PATH_COLOR: Color = Color::RGB(100, 100, 100);

#[derive(Debug)]
pub enum UIEvent {
    Quit,
    EndTurn,
    Continue,
}

pub struct UI {
    pub renderer: renderer::Renderer,
    pub solar_system_view: solar_system_view::SolarSystemView,
    pub end_turn_button: button::Button,
    pub text_renderer: text::TextRenderer,
}

impl UI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let renderer = renderer::Renderer::new()?;
        let solar_system_view = solar_system_view::SolarSystemView::new();
        let end_turn_button = button::Button::new(
            SCREEN_WIDTH as i32 - 150,
            SCREEN_HEIGHT as i32 - 80,
            120,
            50,
            "End Turn".to_string(),
        );
        let text_renderer = text::TextRenderer::new(&renderer.canvas)?;

        Ok(UI {
            renderer,
            solar_system_view,
            end_turn_button,
            text_renderer,
        })
    }

    pub fn render(&mut self, game_state: &crate::game_state::GameState) -> Result<(), Box<dyn std::error::Error>> {
        // Clear screen
        self.renderer.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.renderer.canvas.clear();

        // Render solar system
        self.solar_system_view.render(&mut self.renderer, game_state)?;

        // Render end turn button
        self.end_turn_button.render(&mut self.renderer, &mut self.text_renderer)?;

        // Render date display
        self.render_date_display(game_state)?;

        // Present the frame
        self.renderer.canvas.present();

        Ok(())
    }

    fn render_date_display(&mut self, game_state: &crate::game_state::GameState) -> Result<(), Box<dyn std::error::Error>> {
        let date_text = format!("{} (Turn {})", 
            game_state.get_formatted_date(), 
            game_state.simulation.current_turn
        );
        
        self.text_renderer.render_text(
            &mut self.renderer.canvas,
            &date_text,
            20,
            20,
            DATE_TEXT_COLOR,
            24,
        )?;

        Ok(())
    }

    pub fn handle_events(&mut self, event_pump: &mut sdl2::EventPump) -> Result<UIEvent, Box<dyn std::error::Error>> {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => return Ok(UIEvent::Quit),
                sdl2::event::Event::MouseButtonDown { x, y, .. } => {
                    if self.end_turn_button.is_clicked(x, y) {
                        return Ok(UIEvent::EndTurn);
                    }
                }
                sdl2::event::Event::MouseMotion { x, y, .. } => {
                    self.end_turn_button.update_hover(x, y);
                }
                _ => {}
            }
        }
        Ok(UIEvent::Continue)
    }
} 