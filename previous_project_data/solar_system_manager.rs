pub struct SolarSystemDataRow {
    body: String,
    body_type: Option<String>,
    semi_major_axis: Option<f64>,
    eccentricity: Option<f64>,
    orbital_period: Option<f64>,
    mean_anomaly: Option<f64>,
    mass: Option<f64>,
    diameter: Option<f64>
}


/// Custom deserializer for f64 that handles non-numeric values
fn deserialize_optional_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    
    let value: Option<String> = Option::deserialize(deserializer)?;
    match value {
        Some(s) => {
            if s == "?" || s.is_empty() {
                Ok(None)
            } else {
                s.parse::<f64>().map(Some).map_err(serde::de::Error::custom)
            }
        }
        None => Ok(None),
    }
}

/// Manages the solar system and all celestial bodies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarSystemManager {
    pub celestial_bodies: HashMap<String, CelestialBody>,
    pub game_date: NaiveDate,
    pub previous_positions: HashMap<String, f64>, // Store previous angles for change detection
}

impl SolarSystemManager {
    pub fn new(start_date: NaiveDate) -> Self {
        Self {
            celestial_bodies: HashMap::new(),
            game_date: start_date,
            previous_positions: HashMap::new(),
        }
    }
    
    /// Loads celestial bodies from the CSV file
    pub fn load_from_csv(&mut self, csv_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading solar system data from CSV: {:?}", csv_path);
        
        let file = File::open(csv_path)?;
        let reader = BufReader::new(file);
        let mut csv_reader = csv::Reader::from_reader(reader);
        
        let mut loaded_count = 0;
        let mut skipped_count = 0;
        
        for result in csv_reader.deserialize() {
            let row: SolarSystemDataRow = result?;
            
            // Skip the Sun for now (it's the center of our coordinate system)
            if row.body == "The Sun" {
                info!("Skipping The Sun (center of coordinate system)");
                continue;
            }
            
            // Skip rows without essential orbital data
            if row.semi_major_axis.is_none() || row.eccentricity.is_none() || 
               row.orbital_period.is_none() {
                warn!("Skipping {} - missing essential orbital data (semi_major_axis: {:?}, eccentricity: {:?}, orbital_period: {:?})", 
                      row.body, row.semi_major_axis, row.eccentricity, row.orbital_period);
                skipped_count += 1;
                continue;
            }
            
            // Use mean anomaly if available, otherwise default to 0
            let mean_anomaly = row.mean_anomaly.unwrap_or(0.0);
            
            // Determine body type
            let body_type = self.determine_body_type(&row.body, &row.body_type);
            
            // Create orbital parameters
            let orbital_params = OrbitalParameters {
                semi_major_axis: row.semi_major_axis.unwrap(),
                eccentricity: row.eccentricity.unwrap(),
                orbital_period: row.orbital_period.unwrap(),
                mean_anomaly,
            };
            
            // Create orbital state
            let orbital_state = OrbitalState::new(orbital_params, self.game_date);
            
            // Create celestial body
            let mass = row.mass.unwrap_or(0.0);
            let diameter = row.diameter.unwrap_or(0.0);
            
            let celestial_body = CelestialBody::new(row.body.clone(), body_type, mass, diameter)
                .with_orbital_state(orbital_state);
            
            // Store previous position for change detection
            if let Some(ref orbital_state) = celestial_body.orbital_state {
                self.previous_positions.insert(row.body.clone(), orbital_state.current_position.angle);
            }
            
            self.celestial_bodies.insert(row.body.clone(), celestial_body);
            loaded_count += 1;
        }
        
        info!("Loaded {} celestial bodies from CSV (skipped {} due to missing data)", loaded_count, skipped_count);
        Ok(())
    }
    
    /// Determines the celestial body type based on name and type string
    fn determine_body_type(&self, name: &str, body_type: &Option<String>) -> CelestialBodyType {
        match body_type.as_deref() {
            Some("Star") => CelestialBodyType::Star,
            Some("Rocky Planet") | Some("Gas Giant Planet") => CelestialBodyType::Planet,
            Some("Rocky Moon") => CelestialBodyType::Moon,
            Some("Dwarf Planet") => CelestialBodyType::DwarfPlanet,
            _ => {
                // Fallback to name-based detection
                let name_lower = name.to_lowercase();
                if name_lower.contains("moon") || name_lower.contains("luna") {
                    CelestialBodyType::Moon
                } else if name_lower.contains("asteroid") || name_lower.contains("ceres") || 
                          name_lower.contains("pallas") || name_lower.contains("vesta") {
                    CelestialBodyType::Asteroid
                } else if name_lower.contains("comet") {
                    CelestialBodyType::Comet
                } else if name_lower.contains("pluto") || name_lower.contains("eris") || 
                          name_lower.contains("makemake") || name_lower.contains("haumea") {
                    CelestialBodyType::DwarfPlanet
                } else {
                    CelestialBodyType::Planet
                }
            }
        }
    }
    
    /// Updates all celestial body positions for a given time step
    pub fn update_all_positions(&mut self, days_elapsed: f64) {
        info!("Updating positions of all celestial bodies for {} days", days_elapsed);
        
        let mut significant_changes = Vec::new();
        
        for (name, body) in &mut self.celestial_bodies {
            if let Some(ref mut orbital_state) = body.orbital_state {
                let previous_angle = self.previous_positions.get(name).copied().unwrap_or(0.0);
                
                // Update position
                orbital_state.update_position(days_elapsed);
                
                // Check for significant changes (1 degree threshold)
                if orbital_state.is_significant_change(previous_angle, 1.0) {
                    significant_changes.push((
                        name.clone(),
                        orbital_state.formatted_date(),
                        orbital_state.angle_degrees(),
                        orbital_state.current_position.distance
                    ));
                    
                    // Update previous position
                    self.previous_positions.insert(name.clone(), orbital_state.current_position.angle);
                }
            }
        }
        
        // Log significant changes
        if !significant_changes.is_empty() {
            info!("Significant position changes detected:");
            for (body_name, date, degrees, distance) in significant_changes {
                info!("  {}: {} - Position: {{ degrees: {:.2}, distance: {:.0} km }}", 
                      body_name, date, degrees, distance);
            }
        }
        
        // Update game date
        self.game_date = self.game_date + chrono::Duration::days(days_elapsed as i64);
    }
    
    /// Gets all celestial bodies
    pub fn get_all_bodies(&self) -> &HashMap<String, CelestialBody> {
        &self.celestial_bodies
    }
    
    /// Gets a specific celestial body by name
    pub fn get_body(&self, name: &str) -> Option<&CelestialBody> {
        self.celestial_bodies.get(name)
    }
    
    /// Gets the current game date
    pub fn get_game_date(&self) -> NaiveDate {
        self.game_date
    }
    
    /// Gets formatted game date string
    pub fn get_formatted_date(&self) -> String {
        self.game_date.format("%Y %B %d").to_string()
    }
} 