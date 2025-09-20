use sdl2::pixels::Color;
use sdl2::rect::Rect;
use crate::ui::{SUN_COLOR, EARTH_COLOR, MARS_COLOR, VENUS_COLOR, MERCURY_COLOR, ASTEROID_BELT_COLOR, ORBITAL_PATH_COLOR, SCREEN_WIDTH, SCREEN_HEIGHT};
use crate::ui::renderer::Renderer;
use crate::game_state::GameState;

pub struct SolarSystemView {
    center_x: i32,
    center_y: i32,
    scale_factor: f64, // AU to pixels
}

impl SolarSystemView {
    pub fn new() -> Self {
        let center_x = (SCREEN_WIDTH / 2) as i32;
        let center_y = (SCREEN_HEIGHT / 2) as i32;
        
        // Scale factor: 1 AU = 200 pixels (adjust for visibility)
        let scale_factor = 200.0;
        
        SolarSystemView {
            center_x,
            center_y,
            scale_factor,
        }
    }

    pub fn render(&self, renderer: &mut Renderer, game_state: &GameState) -> Result<(), Box<dyn std::error::Error>> {
        // Draw asteroid belt first (background)
        self.draw_asteroid_belt(renderer)?;
        
        // Draw orbital paths
        self.draw_orbital_paths(renderer, game_state)?;
        
        // Draw sun
        self.draw_sun(renderer)?;
        
        // Draw planets
        self.draw_planets(renderer, game_state)?;
        
        Ok(())
    }

    fn draw_sun(&self, renderer: &mut Renderer) -> Result<(), Box<dyn std::error::Error>> {
        // Draw sun as a large yellow circle
        renderer.draw_circle(self.center_x, self.center_y, 30, SUN_COLOR)?;
        Ok(())
    }

    fn draw_asteroid_belt(&self, renderer: &mut Renderer) -> Result<(), Box<dyn std::error::Error>> {
        // Asteroid belt roughly between 2.2 and 3.3 AU
        let inner_radius = (2.2 * self.scale_factor) as i32;
        let outer_radius = (3.3 * self.scale_factor) as i32;
        
        renderer.draw_ring(self.center_x, self.center_y, inner_radius, outer_radius, ASTEROID_BELT_COLOR)?;
        Ok(())
    }

    fn draw_orbital_paths(&self, renderer: &mut Renderer, game_state: &GameState) -> Result<(), Box<dyn std::error::Error>> {
        let bodies = game_state.solar_system.get_all_bodies();
        
        // Draw orbital paths for inner planets
        let inner_planets = ["Mercury", "Venus", "Earth", "Mars"];
        
        for planet_name in inner_planets.iter() {
            if let Some(body) = bodies.get(*planet_name) {
                if let Some(ref orbital_state) = body.orbital_state {
                    let semi_major_axis = orbital_state.parameters.semi_major_axis;
                    let eccentricity = orbital_state.parameters.eccentricity;
                    
                    // Convert AU to pixels
                    let radius_x = (semi_major_axis * self.scale_factor) as i32;
                    let radius_y = (semi_major_axis * (1.0 - eccentricity * eccentricity).sqrt() * self.scale_factor) as i32;
                    
                    // Draw orbital ellipse
                    renderer.draw_ellipse(self.center_x, self.center_y, radius_x, radius_y, ORBITAL_PATH_COLOR)?;
                }
            }
        }
        
        Ok(())
    }

    fn draw_planets(&self, renderer: &mut Renderer, game_state: &GameState) -> Result<(), Box<dyn std::error::Error>> {
        let bodies = game_state.solar_system.get_all_bodies();
        
        // Planet definitions with colors and names
        let planets = [
            ("Mercury", MERCURY_COLOR),
            ("Venus", VENUS_COLOR),
            ("Earth", EARTH_COLOR),
            ("Mars", MARS_COLOR),
        ];
        
        for (planet_name, color) in planets.iter() {
            if let Some(body) = bodies.get(*planet_name) {
                if let Some(ref orbital_state) = body.orbital_state {
                    let cartesian = orbital_state.to_cartesian();
                    
                    // Convert AU to pixels
                    let x = self.center_x + (cartesian.x / 149597870.7 * self.scale_factor) as i32;
                    let y = self.center_y + (cartesian.y / 149597870.7 * self.scale_factor) as i32;
                    
                    // Draw planet (uniform size)
                    renderer.draw_circle(x, y, 8, *color)?;
                    
                    // Draw planet label
                    self.draw_planet_label(renderer, planet_name, x, y)?;
                }
            }
        }
        
        Ok(())
    }

    fn draw_planet_label(&self, renderer: &mut Renderer, planet_name: &str, x: i32, y: i32) -> Result<(), Box<dyn std::error::Error>> {
        // Simple text rendering using rectangles for now
        // In a full implementation, this would use proper text rendering
        let label_color = Color::RGB(255, 255, 255);
        renderer.canvas.set_draw_color(label_color);
        
        // Position label to the right of the planet
        let label_x = x + 15;
        let label_y = y - 5;
        
        // Draw a simple rectangle as placeholder for text
        renderer.canvas.fill_rect(Rect::new(label_x, label_y, 60, 10))?;
        
        Ok(())
    }
} 