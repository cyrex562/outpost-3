pub struct PolarPosition {
    pub distance: f64,
    pub angle: f64
}

pub struct CartesianPosition {
    pub x: f64,
    pub y: f64
}

pub struct OrbitalParameters {
    pub semi_major_axis: f64,
    pub eccentricity: f64,
    pub orbital_period: f64, 
    pub mean_anomaly: f64,
}

pub struct OrbitalState {
    pub parameters: OrbitalParameters,
    pub current_position: PolarPosition,
    pub current_date: NaiveDate
}

impl OrbitalState {
    /// Creates a new orbital state with initial position based on mean anomaly
    pub fn new(parameters: OrbitalParameters, start_date: NaiveDate) -> Self {
        let initial_angle = parameters.mean_anomaly.to_radians();
        let initial_distance = Self::calculate_distance_from_angle(
            parameters.semi_major_axis,
            parameters.eccentricity,
            initial_angle
        );
        
        Self {
            parameters,
            current_position: PolarPosition {
                distance: initial_distance,
                angle: initial_angle,
            },
            current_date: start_date,
        }
    }
    
    /// Updates the orbital position for a given time step (in days)
    pub fn update_position(&mut self, days_elapsed: f64) {
        // Calculate the new mean anomaly
        let mean_motion = 2.0 * PI / self.parameters.orbital_period; // radians per day
        let mean_anomaly_change = mean_motion * days_elapsed;
        
        // Convert current angle to mean anomaly
        let current_mean_anomaly = self.current_position.angle;
        let new_mean_anomaly = current_mean_anomaly + mean_anomaly_change;
        
        // Convert mean anomaly to true anomaly (angle from perihelion)
        let true_anomaly = Self::mean_anomaly_to_true_anomaly(
            new_mean_anomaly,
            self.parameters.eccentricity
        );
        
        // Calculate new distance and angle
        let new_distance = Self::calculate_distance_from_angle(
            self.parameters.semi_major_axis,
            self.parameters.eccentricity,
            true_anomaly
        );
        
        // Update position (normalize angle to 0-2π)
        let new_angle = (true_anomaly + 2.0 * PI) % (2.0 * PI);
        
        self.current_position = PolarPosition {
            distance: new_distance,
            angle: new_angle,
        };
        
        // Update date
        self.current_date = self.current_date + chrono::Duration::days(days_elapsed as i64);
    }
    
    /// Converts mean anomaly to true anomaly using Kepler's equation
    fn mean_anomaly_to_true_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
        // For small eccentricities, we can use a simplified approach
        // For more accuracy, we'd need to solve Kepler's equation iteratively
        if eccentricity < 0.1 {
            // Small eccentricity approximation
            mean_anomaly + eccentricity * mean_anomaly.sin()
        } else {
            // For larger eccentricities, use a more accurate approximation
            let mut eccentric_anomaly = mean_anomaly;
            
            // Newton's method to solve Kepler's equation: M = E - e*sin(E)
            for _ in 0..5 {
                let delta = (eccentric_anomaly - eccentricity * eccentric_anomaly.sin() - mean_anomaly) 
                           / (1.0 - eccentricity * eccentric_anomaly.cos());
                eccentric_anomaly -= delta;
            }
            
            // Convert eccentric anomaly to true anomaly
            2.0 * ((1.0 + eccentricity).sqrt() / (1.0 - eccentricity).sqrt() * (eccentric_anomaly / 2.0).tan()).atan()
        }
    }
    
    /// Calculates distance from Sun given true anomaly
    fn calculate_distance_from_angle(semi_major_axis: f64, eccentricity: f64, true_anomaly: f64) -> f64 {
        semi_major_axis * (1.0 - eccentricity * eccentricity) / (1.0 + eccentricity * true_anomaly.cos())
    }
    
    /// Converts polar position to Cartesian coordinates
    pub fn to_cartesian(&self) -> CartesianPosition {
        CartesianPosition {
            x: self.current_position.distance * self.current_position.angle.cos(),
            y: self.current_position.distance * self.current_position.angle.sin(),
        }
    }
    
    /// Checks if the position change is significant enough to warrant a screen update
    pub fn is_significant_change(&self, previous_angle: f64, threshold_degrees: f64) -> bool {
        let angle_diff = (self.current_position.angle - previous_angle).abs();
        let angle_diff_degrees = angle_diff * 180.0 / PI;
        angle_diff_degrees >= threshold_degrees
    }
    
    /// Formats the current date as "Year Month Day"
    pub fn formatted_date(&self) -> String {
        self.current_date.format("%Y %B %d").to_string()
    }
    
    /// Gets the angle in degrees for logging
    pub fn angle_degrees(&self) -> f64 {
        self.current_position.angle * 180.0 / PI
    }
}